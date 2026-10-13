//! **Honu Banyan** — wide spreading banyan ([#250](https://github.com/ramate-io/maybraid/issues/250), [RFC §3.1.7.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md)).
//!
//! [`HonuBanyanParams::build`] grows the ball-stick chain once into [`HonuBanyan`],
//! which implements [`VegetationComponents`].
//!
//! [`HonuBanyan::unit_from_num`] / [`HonuBanyanParams::into_unit_from_num`] normalize
//! to unit height and key layout noise by a variant index. Emission folds sticks and
//! cheap canopy balls into collections; jungle-growth fronds stay separate.
//!
//! Structural LOD:
//! - **High** — full sticks (3×4 rings, depth 3..5, child 1..3, ±70° ray, longer hops); jungle growth + banded canopy
//! - **Medium** — trunk + banded sticks; banded growth/canopy + mid layered proxy
//! - **Low** — trunk + ~1/4 descenders; cheap canopy balls + mid proxy

mod canopy;
mod stick;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{BallStickChain, HonuBanyanChain, HonuBanyanSbs};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
use canopy::foliage_nodes_for_level;
pub use canopy::{
	jungle_growth_radius_scale_for_height, DEFAULT_HONU_GROWTH_RADIUS_SCALE,
	HONU_GROWTH_REFERENCE_HEIGHT,
};
use stick::{
	keep_stick_on_low, stick_node_for_segment, stick_nodes_medium_banded, stick_role_for_segment,
};

/// Authoring / CLI parameters for Honu Banyan.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct HonuBanyanParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: HonuBanyanSbs,

	/// Fraction of qualifying outer-ring nodes that spawn jungle growth.
	#[arg(long, default_value_t = 0.28)]
	pub growth_spawn_fraction: f32,

	/// Assembly scale for jungle-growth fronds (Honu-specific; independent of Storybook).
	#[arg(long, default_value_t = DEFAULT_HONU_GROWTH_RADIUS_SCALE)]
	pub jungle_growth_radius_scale: f32,
}

impl Default for HonuBanyanParams {
	fn default() -> Self {
		Self {
			geometry: HonuBanyanSbs::default(),
			growth_spawn_fraction: 0.28,
			jungle_growth_radius_scale: DEFAULT_HONU_GROWTH_RADIUS_SCALE,
		}
	}
}

impl HonuBanyanParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> HonuBanyan {
		HonuBanyan::from_params(self)
	}

	/// Set [`Self::jungle_growth_radius_scale`] from authored tree height.
	pub fn with_growth_scale_for_height(mut self) -> Self {
		self.jungle_growth_radius_scale =
			jungle_growth_radius_scale_for_height(self.geometry.scale.tree_height);
		self
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.scale.tree_height.max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.tree_height = 1.0;
		geometry.canopy_noise.seed = num as i32;
		geometry.anchor_perturbation.noise.seed = num as i32;
		(
			Self {
				geometry,
				growth_spawn_fraction: self.growth_spawn_fraction,
				jungle_growth_radius_scale: (self.jungle_growth_radius_scale * inv).max(1e-6),
			},
			size,
		)
	}
}

/// Built Honu Banyan: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct HonuBanyan {
	pub geometry: HonuBanyanSbs,
	pub chain: BallStickChain<HonuBanyanChain>,
	pub growth_spawn_fraction: f32,
	pub jungle_growth_radius_scale: f32,
}

impl HonuBanyan {
	pub fn from_params(params: &HonuBanyanParams) -> Self {
		Self {
			geometry: params.geometry.clone(),
			chain: params.geometry.build_chain(),
			growth_spawn_fraction: params.growth_spawn_fraction,
			jungle_growth_radius_scale: params.jungle_growth_radius_scale,
		}
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&HonuBanyanParams::unit_from_num(num))
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.to_stalk().stalk_base_radius.max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.scale.tree_height * 0.5, 0.0)
	}

	fn stick_nodes_high(&self) -> Vec<StickNode> {
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| stick_node_for_segment(&segment, parent))
			.collect()
	}

	fn stick_nodes_medium(&self) -> Vec<StickNode> {
		stick_nodes_medium_banded(
			self.chain
				.segments_with_hysteresis()
				.map(|(segment, parent, _)| (segment, parent)),
		)
	}

	fn stick_nodes_low(&self) -> Vec<StickNode> {
		let mut descender_index = 0usize;
		self.chain
			.segments_with_hysteresis()
			.filter_map(|(segment, parent, _)| {
				let role = stick_role_for_segment(&segment, parent);
				if !keep_stick_on_low(role, &mut descender_index) {
					return None;
				}
				stick_node_for_segment(&segment, parent)
			})
			.collect()
	}

	fn foliage_for(&self, level: LodSceneLevel) -> Vec<FoliageNode> {
		foliage_nodes_for_level(
			&self.chain,
			level,
			self.growth_spawn_fraction,
			self.jungle_growth_radius_scale,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
		)
	}
}

impl VegetationComponents for HonuBanyan {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => self.stick_nodes_high(),
			LodSceneLevel::Medium => self.stick_nodes_medium(),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.stick_nodes_low(),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(stick_material_ref())).collect();
		Layers::from_free(merge_kit_sticks(nodes))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let nodes: Vec<_> = self
			.foliage_for(level)
			.into_iter()
			.map(|n| {
				if n.geometry.is_frond_collection() {
					n
				} else {
					n.with_material(leaf_material_ref())
				}
			})
			.collect();
		Layers::from_free(merge_cheap_ball_foliage(nodes))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(StructuralLod::new(self.structural_center(), self.footprint_radius()).with_factors(
			STRUCTURAL_HIGH_FACTOR,
			STRUCTURAL_MEDIUM_FACTOR,
			STRUCTURAL_LOW_FACTOR,
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;
	use vegetation_components::FoliageGeometry;

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = HonuBanyan::unit_from_num(3);
		let b = HonuBanyan::unit_from_num(3);
		let c = HonuBanyan::unit_from_num(4);
		assert!((a.geometry.scale.tree_height - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = HonuBanyanParams::default();
		params.geometry.scale.tree_height = 24.0;
		params.jungle_growth_radius_scale = 4.0;
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 24.0).abs() < 1e-5);
		assert!((unit.geometry.scale.tree_height - 1.0).abs() < 1e-5);
		assert!((unit.jungle_growth_radius_scale - 4.0 / 24.0).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_and_cheap_ball_collections() -> Result<()> {
		let tree = HonuBanyan::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(foliage
			.iter()
			.any(|n| matches!(n.geometry, FoliageGeometry::CheapBallCollection(_))));
		assert!(foliage.iter().any(|n| n.geometry.is_frond_collection()));
		Ok(())
	}
}
