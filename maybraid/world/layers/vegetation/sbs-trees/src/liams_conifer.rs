//! **Liam's Conifer** — sparse dry conifer ([RFC-183 §3.1.7.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md), [#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! [`LiamsConiferParams::build`] grows the ball-stick chain once into [`LiamsConifer`], which
//! implements [`VegetationComponents`]. Sticks reuse Northern / Liam banding; foliage uses
//! cheap-ball joint clusters sized by [`LiamsConiferSbs::tuft_world_scale`] (no SucculentTuft
//! mesh under VegetationComponents).
//!
//! [`LiamsConifer::unit_from_num`] / [`LiamsConiferParams::into_unit_from_num`]
//! normalize to unit height and key layout noise by a variant index so many plants
//! share one archetypal mesh (world size goes on placement scale). Emission
//! folds sticks and cheap balls into collections.

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{BallStickChain, LiamsConiferChain, LiamsConiferSbs};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::northern_conifer::canopy::{
	foliage_nodes_banded, foliage_nodes_low_single_proxy, foliage_nodes_medium_no_proxy,
	HIGH_FOLIAGE_BANDS,
};
use crate::northern_conifer::stick::{stick_nodes_high, stick_nodes_low, stick_nodes_medium_liams};
use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

/// Authoring / CLI parameters for Liam's Conifer.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct LiamsConiferParams {
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: LiamsConiferSbs,
}

impl Default for LiamsConiferParams {
	fn default() -> Self {
		Self { geometry: LiamsConiferSbs::default() }
	}
}

impl LiamsConiferParams {
	pub fn build(&self) -> LiamsConifer {
		LiamsConifer::from_params(self)
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.scale.stalk_height.max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.stalk_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
		geometry.canopy_noise.seed = num as i32;
		geometry.anchor_perturbation.noise.seed = num as i32;
		(Self { geometry }, size)
	}
}

/// Built Liam's Conifer: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct LiamsConifer {
	pub geometry: LiamsConiferSbs,
	pub chain: BallStickChain<LiamsConiferChain>,
}

impl LiamsConifer {
	pub fn from_params(params: &LiamsConiferParams) -> Self {
		Self { geometry: params.geometry.clone(), chain: params.geometry.build_chain() }
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&LiamsConiferParams::unit_from_num(num))
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius_or_default().max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.scale.stalk_height * 0.5, 0.0)
	}

	fn height(&self) -> f32 {
		self.geometry.scale.stalk_height.max(1e-6)
	}

	fn tuft_radius_world(&self) -> f32 {
		self.geometry.tuft_world_scale()
	}
}

impl VegetationComponents for LiamsConifer {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_medium_liams(&self.chain),
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
		let tuft_r = self.tuft_radius_world();
		// Arid look: Medium drops density via banding only (no mass proxy); Low keeps one proxy.
		let nodes = match level {
			LodSceneLevel::High => {
				foliage_nodes_banded(&self.chain, HIGH_FOLIAGE_BANDS, tuft_r, 1.0, 0.0, 0.0)
			}
			LodSceneLevel::Medium => foliage_nodes_medium_no_proxy(&self.chain, tuft_r, 1.0),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => foliage_nodes_low_single_proxy(&self.chain, tuft_r, 1.0),
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
				self.height(),
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
		let a = LiamsConifer::unit_from_num(3);
		let b = LiamsConifer::unit_from_num(3);
		let c = LiamsConifer::unit_from_num(4);
		assert!((a.geometry.scale.stalk_height - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = LiamsConiferParams::default();
		params.geometry.scale.stalk_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_height - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_and_cheap_ball_collections() -> Result<()> {
		let tree = LiamsConifer::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(foliage.len(), 1);
		assert!(matches!(foliage[0].geometry, FoliageGeometry::CheapBallCollection(_)));
		Ok(())
	}
}
