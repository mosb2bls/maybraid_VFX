//! **Rory's Head-trained** — top-heavy trained tree with a single horizontal canopy ring ([#254](https://github.com/ramate-io/maybraid/issues/254), [RFC §3.1.7.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md)).
//!
//! [`RorysHeadTrainedParams::build`] grows the ball-stick chain once into [`RorysHeadTrained`],
//! which implements [`VegetationComponents`].
//!
//! [`RorysHeadTrained::unit_from_num`] / [`RorysHeadTrainedParams::into_unit_from_num`]
//! normalize to unit height and key layout noise by a variant index so many plants
//! share one archetypal mesh (world size goes on [`Placement`](vegetation_components::Placement)
//! scale). Emission folds sticks and cheap balls into collections.
//!
//! Structural / stick LOD matches Penmarch Torch (`torch_tree`); foliage keeps joint
//! candidates with cheap-ball banding and no layered mass proxies.

mod canopy;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{BallStickChain, RorysHeadTrainedSbs, StorybookTreeChain};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
use crate::torch_tree::{stick_nodes_high, stick_nodes_low, stick_nodes_medium};
use canopy::{foliage_nodes_banded, foliage_nodes_low, foliage_nodes_medium, HIGH_FOLIAGE_BANDS};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

/// Authoring / CLI parameters for Rory's Head-trained.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct RorysHeadTrainedParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: RorysHeadTrainedSbs,
}

impl Default for RorysHeadTrainedParams {
	fn default() -> Self {
		Self { geometry: RorysHeadTrainedSbs::default() }
	}
}

impl RorysHeadTrainedParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> RorysHeadTrained {
		RorysHeadTrained::from_params(self)
	}

	/// RFC bush / grape-vine preset (shorter stalk, `0.60 * H` spread).
	pub fn apply_bush_preset(&mut self) {
		self.geometry.apply_bush_preset();
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.tree_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.canopy_noise.seed = num as i32;
		geometry.anchor_perturbation.noise.seed = num as i32;
		(Self { geometry }, size)
	}
}

/// Built Rory's Head-trained: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct RorysHeadTrained {
	pub geometry: RorysHeadTrainedSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
}

impl RorysHeadTrained {
	pub fn from_params(params: &RorysHeadTrainedParams) -> Self {
		Self { geometry: params.geometry.clone(), chain: params.geometry.build_chain() }
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.height() * 0.5, 0.0)
	}

	fn leaf_radius_world(&self) -> f32 {
		self.geometry.leaf_radius_world()
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&RorysHeadTrainedParams::unit_from_num(num))
	}
}

impl VegetationComponents for RorysHeadTrained {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium(&self.chain),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => stick_nodes_low(&self.chain),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(stick_material_ref())).collect();
		Layers::from_free(merge_kit_sticks(nodes))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let leaf_r = self.leaf_radius_world();
		let nodes = match level {
			LodSceneLevel::High => foliage_nodes_banded(&self.chain, HIGH_FOLIAGE_BANDS, leaf_r),
			LodSceneLevel::Medium => foliage_nodes_medium(&self.chain, leaf_r),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low(&self.chain, leaf_r),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(leaf_material_ref())).collect();
		Layers::from_free(merge_cheap_ball_foliage(nodes))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.geometry.height(),
			)
			.with_factors(
				STRUCTURAL_HIGH_FACTOR,
				STRUCTURAL_MEDIUM_FACTOR,
				STRUCTURAL_LOW_FACTOR,
			),
		)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use vegetation_components::FoliageGeometry;

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = RorysHeadTrained::unit_from_num(3);
		let b = RorysHeadTrained::unit_from_num(3);
		let c = RorysHeadTrained::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = RorysHeadTrainedParams::default();
		params.geometry.scale.tree_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_and_cheap_ball_collections() -> Result<()> {
		let tree = RorysHeadTrained::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(foliage.len(), 1);
		assert!(matches!(foliage[0].geometry, FoliageGeometry::CheapBallCollection(_)));
		Ok(())
	}
}
