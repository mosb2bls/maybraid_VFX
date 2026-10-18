//! Foliage IR node: geometry + placement + collection presentation.
//!
//! [`FoliageNode`] is authoring IR and the fine-phase [`lod::scene::LodScene`] host.
//! Scene emission is [`present`]; banding / culls are [`lod`].

mod lod;
mod present;

use std::collections::HashMap;

use bevy::prelude::{Component, Vec3};
use material_ref::{MaterialId, MaterialRef};

use crate::foliage::collection::{CheapBallCollection, FrondCollection};
use crate::foliage::geometry::FoliageGeometry;
use crate::foliage::present::CollectionPresent;
use crate::foliage::probe::FoliageLodProbe;
use crate::materials::frond_material_ref;
use crate::placed::Placement;

/// Authoring IR for a foliage cluster — also the fine-phase [`lod::scene::LodScene`] host.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct FoliageNode {
	pub geometry: FoliageGeometry,
	pub placement: Placement,
	/// Deferred material. Defaults to [`MaterialRef::default()`] (green standard);
	/// frond constructors stamp [`crate::frond_material_ref`]; higher-order
	/// types set leaf / palette as needed.
	pub material: MaterialRef,
	/// Kit collections only. Single kits ignore this. Default [`CollectionPresent::Merge`].
	pub collection_present: CollectionPresent,
}

impl FoliageNode {
	pub fn new(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self {
			geometry,
			placement,
			material: MaterialRef::default(),
			collection_present: CollectionPresent::Merge,
		}
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = material;
		self
	}

	/// Kit-collection presenter. No-op for single kits.
	pub fn with_collection_present(mut self, present: CollectionPresent) -> Self {
		self.collection_present = present;
		self
	}

	/// Instance each collection member as a posed kit under this host.
	pub fn instanced(self) -> Self {
		self.with_collection_present(CollectionPresent::Instance)
	}

	/// Bake collection members into one [`scene_ref::MultiSceneMerge`] (default).
	pub fn merged(self) -> Self {
		self.with_collection_present(CollectionPresent::Merge)
	}

	/// Layered ball using `vegetation/foliage/standard/layered_ball_001_*` GLBs.
	pub fn layered_ball(placement: Placement) -> Self {
		Self::new(FoliageGeometry::LayeredBall, placement)
	}

	/// Cheap ball using `vegetation/foliage/standard/cheap_ball_001_*` GLBs.
	///
	/// Prefer for dense packed clusters where silhouette comes from density.
	pub fn cheap_ball(placement: Placement) -> Self {
		Self::new(FoliageGeometry::CheapBall, placement)
	}

	/// Square-ended straight frond segment (`straight_frond_segment_001_*`).
	pub fn straight_frond_segment(placement: Placement) -> Self {
		Self::new(FoliageGeometry::StraightFrondSegment, placement)
			.with_material(frond_material_ref())
	}

	/// Point-tip straight frond (`straight_frond_001_*`); prefer [`Self::straight_frond_segment`].
	pub fn straight_frond(placement: Placement) -> Self {
		Self::new(FoliageGeometry::StraightFrond, placement).with_material(frond_material_ref())
	}

	/// Frond collection under one LOD parent. Default presenter is merge.
	///
	/// Parent [`Placement`] is usually identity when members are already tree-local.
	/// Use [`Self::instanced`] when members must stay separate meshes.
	pub fn frond_collection(collection: FrondCollection, placement: Placement) -> Self {
		Self::new(FoliageGeometry::FrondCollection(collection), placement)
			.with_material(frond_material_ref())
	}

	/// Cheap-ball collection under one LOD parent. Default presenter is merge.
	pub fn cheap_ball_collection(collection: CheapBallCollection, placement: Placement) -> Self {
		Self::new(FoliageGeometry::CheapBallCollection(collection), placement)
	}

	/// Fold cheap-ball nodes into one collection (shared material, baked probe).
	pub fn merge_cheap_balls(nodes: impl IntoIterator<Item = Self>) -> Option<Self> {
		let nodes: Vec<Self> = nodes.into_iter().collect();
		let material = nodes.first()?.material.clone();
		let mut placements = Vec::with_capacity(nodes.len());
		for node in &nodes {
			match &node.geometry {
				FoliageGeometry::CheapBallCollection(existing) => {
					placements.extend(existing.placements.iter().copied());
				}
				FoliageGeometry::CheapBall => placements.push(node.placement),
				_ => {}
			}
		}
		if placements.is_empty() {
			return None;
		}
		let collection = CheapBallCollection::new(placements).bake_bounds_from_placements();
		Some(Self::cheap_ball_collection(collection, Placement::IDENTITY).with_material(material))
	}

	/// Fold cheap balls into one collection **per material recipe**; leave fronds
	/// and other geometries as-is.
	///
	/// Grove Low / UltraLow canopy proxies use this so a tile is a few posed kits,
	/// not one [`lod::LodScene`] host per plant. Trunk (stick) and crown (leaf)
	/// must not share a kit — [`Self::merge_cheap_balls`] keeps the first
	/// material, which painted umbrellas bark-colored.
	pub fn merge_canopy_proxies(nodes: impl IntoIterator<Item = Self>) -> Vec<Self> {
		let mut groups: HashMap<MaterialId, Vec<Self>> = HashMap::new();
		let mut rest = Vec::new();
		for node in nodes {
			match &node.geometry {
				FoliageGeometry::CheapBall | FoliageGeometry::CheapBallCollection(_) => {
					groups.entry(node.material.name.clone()).or_default().push(node);
				}
				_ => rest.push(node),
			}
		}
		for cheap in groups.into_values() {
			if let Some(merged) = Self::merge_cheap_balls(cheap) {
				rest.insert(0, merged);
			}
		}
		rest
	}

	/// Expand the cheap-ball collection probe to at least `radius` around `center`.
	pub fn with_cheap_ball_probe(mut self, center: Vec3, radius: f32) -> Self {
		if let FoliageGeometry::CheapBallCollection(collection) = &mut self.geometry {
			collection.center = center;
			collection.radius = radius.max(collection.radius).max(1e-4);
		}
		self
	}

	pub fn standard(geometry: FoliageGeometry, placement: Placement) -> Self {
		Self::new(geometry, placement)
	}

	fn probe(&self) -> FoliageLodProbe {
		match &self.geometry {
			FoliageGeometry::FrondCollection(collection) => {
				let (local_center, local_radius) = collection.center_and_extent();
				let (center, extent) = self.composed_collection_extent(local_center, local_radius);
				let mut probe = FoliageLodProbe::for_kit_collection(center, extent);
				probe.center = center;
				probe.extent = extent;
				probe
			}
			FoliageGeometry::CheapBallCollection(collection) => {
				let (local_center, local_radius) = collection.center_and_extent();
				let (center, extent) = self.composed_collection_extent(local_center, local_radius);
				FoliageLodProbe::for_cheap_ball_probe(center, extent)
			}
			_ => FoliageLodProbe::from_placement(&self.placement),
		}
	}

	fn composed_collection_extent(&self, local_center: Vec3, local_radius: f32) -> (Vec3, f32) {
		let world_center =
			self.placement.compose_child(Placement::new(local_center, 0.0)).translation;
		let scale = self.placement.scale.abs().max_element().max(1e-4);
		(world_center, (local_radius * scale).max(1e-4))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::materials::{frond_material_ref, FROND_MATERIAL};
	use material_ref::MaterialId;

	#[test]
	fn frond_constructors_use_chico_frond_material() {
		let expected = MaterialId::named(FROND_MATERIAL);
		assert_eq!(
			FoliageNode::straight_frond_segment(Placement::IDENTITY).material.name,
			expected
		);
		assert_eq!(FoliageNode::straight_frond(Placement::IDENTITY).material.name, expected);
		assert_eq!(
			FoliageNode::frond_collection(FrondCollection::new([]), Placement::IDENTITY)
				.material
				.name,
			expected
		);
		assert_eq!(frond_material_ref().name, expected);
	}

	#[test]
	fn collection_defaults_to_merge_and_instanced_opts_out() {
		let merged = FoliageNode::frond_collection(FrondCollection::new([]), Placement::IDENTITY);
		assert_eq!(merged.collection_present, CollectionPresent::Merge);
		assert_eq!(merged.clone().instanced().collection_present, CollectionPresent::Instance);
		assert_eq!(merged.instanced().merged().collection_present, CollectionPresent::Merge);
	}
}
