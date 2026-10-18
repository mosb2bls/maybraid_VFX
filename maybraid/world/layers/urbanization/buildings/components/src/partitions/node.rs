//! Partition IR node: style + geometry + placement — fine-phase [`LodScene`] host.
//!
//! Covers both **direct** component mappings (e.g. a single linear / arc kit) and
//! **tessellated** concepts (polyline / continuous arc → many tiles under **one** LOD
//! parent host).

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Transform};
use bevy::scene::prelude::Scene;
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;
use material_ref::MaterialRef;

use crate::kit_merge::{scenes_from_kit_parts, KitPart};
use crate::layer::Layers;
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::parent_confines::ParentConfines;
use crate::partitions::geometry::{JointLod, LinearLod, PartitionGeometry, PartitionTile};
use crate::partitions::host::mesh_scene_ref;
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::partitions::style::PartitionStyle;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children};

/// Authoring IR for a partition feature (primitive — no portals).
///
/// [`Self::style`] picks the kit GLB path. [`Self::material`] is an optional
/// shader look ([`MaterialRef`]) stamped onto that kit after spawn.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct PartitionNode {
	pub style: PartitionStyle,
	pub geometry: PartitionGeometry,
	pub placement: Placement,
	/// External silhouette vs internal detail gating.
	pub confines: ParentConfines,
	pub material: Option<MaterialRef>,
}

impl PartitionNode {
	pub fn new(style: PartitionStyle, geometry: PartitionGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement, confines: ParentConfines::External, material: None }
	}

	pub fn rough_stone(geometry: PartitionGeometry, placement: Placement) -> Self {
		Self::new(PartitionStyle::RoughStonework, geometry, placement)
	}

	pub fn with_confines(mut self, confines: ParentConfines) -> Self {
		self.confines = confines;
		self
	}

	pub fn with_material(mut self, material: MaterialRef) -> Self {
		self.material = Some(material);
		self
	}

	/// Status as if a rough-stone linear partition sat at `center` with `extent` scale.
	pub fn representative_lod_status(
		center: Vec3,
		extent: Vec3,
		lod_ref: &LodRef,
	) -> LodSceneStatus {
		let node = Self::rough_stone(
			PartitionGeometry::linear(),
			Placement::new(center, 0.0).with_scale(extent.max(Vec3::splat(1e-4))),
		);
		node.scene_lod_status(lod_ref)
	}

	pub(crate) fn kit_parts(&self, level: LodSceneLevel) -> Vec<KitPart> {
		self.geometry
			.placed_tiles_for_style(self.style, self.placement)
			.into_iter()
			.filter_map(|piece| {
				let scene = tile_scene_ref(self.style, piece.geom, level)?;
				Some(KitPart {
					scene,
					transform: pose(piece.placement),
					material: self.material.clone(),
					confines: self.confines,
				})
			})
			.collect()
	}
}

fn tile_scene_ref(
	style: PartitionStyle,
	tile: PartitionTile,
	level: LodSceneLevel,
) -> Option<scene_ref::SceneRef> {
	match style {
		PartitionStyle::RoughStonework => match tile {
			PartitionTile::Joint => {
				JointLod::asset_for_level(level).map(crate::assets::AssetPath::scene_ref)
			}
			PartitionTile::RightTriangle { mirror } => {
				use crate::assets::panels::rough_stonework::{
					RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
				};
				Some(mesh_scene_ref(
					PartitionMeshSet::new(
						RIGHT_TRIANGLE_HIGH,
						RIGHT_TRIANGLE_MID,
						RIGHT_TRIANGLE_LOW,
					),
					level,
					mirror,
				))
			}
			tile => tile.mesh_set().map(|meshes| mesh_scene_ref(meshes, level, None)),
		},
	}
}

impl Layers<PartitionNode> {
	/// Stamp a shader look onto every partition, leaving kit [`PartitionStyle`] unchanged.
	pub fn with_material(self, material: MaterialRef) -> Self {
		self.map(|node| node.with_material(material.clone()))
	}
}

/// Door-frame / empty leaf tiles that lack a mesh set — posed content for `level`.
pub(crate) fn partition_tile_scene(tile: PartitionTile, level: LodSceneLevel) -> Box<dyn Scene> {
	match tile {
		PartitionTile::Joint => Box::new(JointLod::posed_tier(Transform::IDENTITY, level)),
		other => {
			if let Some(meshes) = other.mesh_set() {
				Box::new(LinearLod::posed_tier(meshes, Transform::IDENTITY, level))
			} else {
				Box::new(JointLod::posed_tier(Transform::IDENTITY, LodSceneLevel::Low))
			}
		}
	}
}

impl LodScene for PartitionNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.placement.partition_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.placement.partition_lod_status(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		let _ = lod_ref;
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		scene_children(scenes_from_kit_parts(self.kit_parts(level)))
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}

macro_rules! impl_partition_mesh_lod_scene {
	($ty:ty, $meshes:expr) => {
		impl ::lod::scene::LodScene for $ty {
			fn scene_lod_level(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::scene::LodSceneLevel {
				$crate::partitions::probe::PartitionLodProbe::from_aabb(lod_ref.bounds)
					.level_for(lod_ref.current_transform)
			}

			fn scene_lod_status(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::scene::LodSceneStatus {
				$crate::partitions::probe::PartitionLodProbe::from_aabb(lod_ref.bounds)
					.status_for_lod_ref(lod_ref)
			}

			fn scene_lod_culls(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				current: ::lod::scene::LodSceneLevel,
			) -> ::lod::scene::LodSceneCulls {
				$crate::lod_band::warm_mesh_lod_culls(current)
			}

			fn scene_with_level(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				level: ::lod::scene::LodSceneLevel,
			) -> impl ::bevy::scene::Scene + 'static {
				$crate::partitions::geometry::LinearLod::posed_tier(
					$meshes,
					::bevy::prelude::Transform::IDENTITY,
					level,
				)
			}

			fn scene_chunks_with_level(
				&self,
				lod_ref: &::lod::lod_ref::LodRef,
				level: ::lod::scene::LodSceneLevel,
			) -> ::lod::SceneChunk {
				::lod::SceneChunk::primitive(self.scene_with_level(lod_ref, level))
			}

			fn scene_bounds(&self) -> ::bevy::math::bounding::Aabb3d {
				::bevy::math::bounding::Aabb3d::from_min_max(
					::bevy::math::Vec3::new(0.0, 0.0, 0.0),
					::bevy::math::Vec3::new(1.0, 1.0, 1.0),
				)
			}
		}
	};
}

pub(crate) use impl_partition_mesh_lod_scene;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::partitions::geometry::PartitionGeometry;
	use crate::placed::Placement;

	#[test]
	fn with_material_stamps_ref_without_changing_style() {
		let node = PartitionNode::rough_stone(PartitionGeometry::linear(), Placement::default())
			.with_material(MaterialRef::named("stucco"));
		assert_eq!(node.style, PartitionStyle::RoughStonework);
		assert_eq!(
			node.material.as_ref().map(|m| &m.name),
			Some(&material_ref::MaterialId::named("stucco"))
		);
	}
}
