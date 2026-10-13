//! **Waialea Palm** — arched trunk + light upward frond crown ([#255](https://github.com/ramate-io/maybraid/issues/255), [RFC §3.1.7.8](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/08-waialea-palm/README.md)).
//!
//! [`WaialeaPalmParams::build`] grows the arched trunk once into [`WaialeaPalm`], which
//! implements [`VegetationComponents`]: trunk sticks at every band + per-frond
//! collections at High/Medium and a shared five-chord Low star at Low/UltraLow
//! (no mid-tree canopy to hide a missing stalk). Probe and collection nodes bake
//! at build so produce / grove emit do not rebuild rings.
//!
//! [`WaialeaPalm::unit_from_num`] / [`WaialeaPalmParams::into_unit_from_num`] normalize
//! the trunk to unit height and key trunk noise by a variant index. Emission folds
//! trunk sticks into a collection; frond collections stay separate.

mod crown;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::FrondCrownShape;
use sbs_geometry::{BallStickChain, WaialeaPalmChain, WaialeaPalmSbs};
use vegetation_components::{
	stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents,
};

use crate::palm_crown::{
	DETAIL_FROND_LENGTH_FRACTION, DETAIL_FROND_WIDTH_FRACTION, FROND_RING_SEED_SALT,
};
use crate::palm_tree::{
	crown_lod_probe, frond_collection_nodes, low_star_nodes_for_rings, trunk_stick_nodes,
	world_space_frond_shape,
};
use crate::storybook_tree::merge_kit_sticks;
use crown::frond_shape_for_ring;

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 50.0;
const STRUCTURAL_LOW_FACTOR: f32 = 72.0;

/// Authoring / CLI parameters for Waialea Palm.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct WaialeaPalmParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: WaialeaPalmSbs,
}

impl Default for WaialeaPalmParams {
	fn default() -> Self {
		Self { geometry: WaialeaPalmSbs::default() }
	}
}

impl WaialeaPalmParams {
	pub fn build(&self) -> WaialeaPalm {
		WaialeaPalm::from_params(self)
	}

	/// Unit-height palm whose trunk noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.stalk_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.trunk_noise.seed = num as i32;
		(Self { geometry }, size)
	}
}

/// Built Waialea Palm: geometry, arched trunk, plus baked unit probe / collection nodes.
#[derive(Clone)]
pub struct WaialeaPalm {
	pub geometry: WaialeaPalmSbs,
	pub chain: BallStickChain<WaialeaPalmChain>,
	structural_lod: StructuralLod,
	high_nodes: Vec<FoliageNode>,
	low_nodes: Vec<FoliageNode>,
}

impl WaialeaPalm {
	pub fn from_params(params: &WaialeaPalmParams) -> Self {
		let geometry = params.geometry.clone();
		let chain = geometry.build_chain();
		let seed = geometry.trunk_noise.seed;
		let scale = geometry.frond_world_scale;
		let rings: Vec<(Vec3, FrondCrownShape)> = (0..geometry.crown.ring_count)
			.map(|ring| {
				let anchor = geometry.crown_ring_position(&chain, ring);
				let local = frond_shape_for_ring(
					&geometry,
					ring,
					seed.wrapping_add(ring as i32 * FROND_RING_SEED_SALT),
				);
				(anchor, world_space_frond_shape(local, scale))
			})
			.collect();
		let height = geometry.height();
		let footprint = chain
			.footprint_radius_at_least(geometry.scale.stalk_base_radius_or_default().max(1e-3));
		let (center, radius) = crown_lod_probe(&rings, Some((footprint, height)));
		let structural_lod = StructuralLod::new(center, radius).with_factors(
			STRUCTURAL_HIGH_FACTOR,
			STRUCTURAL_MEDIUM_FACTOR,
			STRUCTURAL_LOW_FACTOR,
		);
		let high_nodes = frond_collection_nodes(&rings, center, radius);
		let low_nodes = low_star_nodes_for_rings(
			&rings,
			DETAIL_FROND_LENGTH_FRACTION * height,
			DETAIL_FROND_WIDTH_FRACTION * height,
			Some((footprint, height)),
		);
		Self { geometry, chain, structural_lod, high_nodes, low_nodes }
	}

	/// Unit-height palm whose trunk noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&WaialeaPalmParams::unit_from_num(num))
	}
}

impl VegetationComponents for WaialeaPalm {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		let nodes: Vec<_> = trunk_stick_nodes(&self.chain)
			.into_iter()
			.map(|n| n.with_material(stick_material_ref()))
			.collect();
		Layers::from_free(merge_kit_sticks(nodes))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				Layers::from_free(self.high_nodes.clone())
			}
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => Layers::from_free(self.low_nodes.clone()),
		}
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(self.structural_lod)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use lod::scene::LodSceneLevel;

	#[test]
	fn high_collections_use_structural_crown_probe() -> Result<()> {
		crate::palm_tree::assert_high_collections_match_structural_lod(
			&WaialeaPalmParams::default().build(),
		);
		Ok(())
	}

	#[test]
	fn low_keeps_trunk_sticks_and_shared_star() -> Result<()> {
		let built = WaialeaPalmParams::default().build();
		assert!(!built.stick_nodes_for_level(LodSceneLevel::Low).flatten().is_empty());
		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), crate::palm_tree::LOW_STAR_FROND_COUNT as usize);
		assert!(low.iter().all(|n| n.geometry.is_frond_collection()));
		for node in &low {
			let collection = node.geometry.as_frond_collection().expect("star");
			assert_eq!(collection.runs.len(), 1);
			assert_eq!(collection.runs[0].segments.len(), 1);
		}
		Ok(())
	}

	#[test]
	fn structural_lod_is_baked_unit_probe() -> Result<()> {
		let built = WaialeaPalmParams::default().build();
		let a = built.structural_lod();
		let b = built.structural_lod();
		assert_eq!(a, b);
		let band = a.expect("probe");
		assert!((band.high_factor - STRUCTURAL_HIGH_FACTOR).abs() < 1e-5);
		assert!(band.tree_radius > 0.0);
		Ok(())
	}

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = WaialeaPalm::unit_from_num(3);
		let b = WaialeaPalm::unit_from_num(3);
		let c = WaialeaPalm::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.trunk_noise.seed, 3);
		assert_eq!(a.geometry.trunk_noise.seed, b.geometry.trunk_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.trunk_noise.seed, c.geometry.trunk_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = WaialeaPalmParams::default();
		params.geometry.scale.stalk_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.trunk_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_collection() -> Result<()> {
		let tree = WaialeaPalm::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!foliage.is_empty());
		assert!(foliage.iter().all(|n| n.geometry.is_frond_collection()));
		Ok(())
	}
}
