//! **Jungle Storybook Tree** — dense Storybook construction ([#235](https://github.com/ramate-io/maybraid/issues/235), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-jungle-storybook-tree/README.md)).
//!
//! Same [`StorybookTreeChain`] geometry as [#230](https://github.com/ramate-io/maybraid/issues/230);
//! VegetationComponents emits jungle-growth clusters (palm fronds + spears) plus cheap canopy balls.
//!
//! [`JungleStorybookTree::unit_from_num`] / [`JungleStorybookTreeParams::into_unit_from_num`]
//! normalize to unit height and key layout noise by a variant index so many plants
//! share one archetypal mesh (world size goes on [`Placement`](vegetation_components::Placement)
//! scale). Emission folds sticks and cheap balls into collections; frond growth stays separate.

mod canopy;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{BallStickChain, JungleStorybookTreeSbs, StorybookTreeChain};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::storybook_tree::canopy::MEDIUM_STICK_BANDS;
use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
use crate::torch_tree::{stick_nodes_banded, stick_nodes_high, stick_nodes_low};
use canopy::{foliage_nodes_for_level, DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

/// Authoring / CLI parameters for Jungle Storybook Tree.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct JungleStorybookTreeParams {
	/// Flattened [`StorybookTreeSbs`](sbs_geometry::StorybookTreeSbs) (clap defaults are storybook, not jungle).
	/// [`Self::build`] / [`JungleStorybookTree::from_params`] call [`JungleStorybookTreeSbs::apply_jungle_preset`].
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: JungleStorybookTreeSbs,

	/// Share of foliage-eligible nodes that spawn jungle growth.
	#[arg(long, default_value_t = 0.22)]
	pub growth_spawn_fraction: f32,

	/// Assembly scale for jungle-growth fronds (Storybook-specific; independent of Honu).
	#[arg(long, default_value_t = DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE)]
	pub jungle_growth_radius_scale: f32,
}

impl Default for JungleStorybookTreeParams {
	fn default() -> Self {
		Self {
			geometry: JungleStorybookTreeSbs::default(),
			growth_spawn_fraction: 0.22,
			jungle_growth_radius_scale: DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE,
		}
	}
}

impl JungleStorybookTreeParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> JungleStorybookTree {
		JungleStorybookTree::from_params(self)
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	///
	/// Applies the jungle preset first so world size is the post-preset height.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		geometry.apply_jungle_preset();
		let size = geometry.height().max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.tree_height = 1.0;
		if let Some(radius) = geometry.scale.stalk_base_radius {
			geometry.scale.stalk_base_radius = Some((radius * inv).max(1e-6));
		}
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

/// Built Jungle Storybook Tree: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct JungleStorybookTree {
	pub geometry: JungleStorybookTreeSbs,
	pub chain: BallStickChain<StorybookTreeChain>,
	pub growth_spawn_fraction: f32,
	pub jungle_growth_radius_scale: f32,
}

impl JungleStorybookTree {
	pub fn from_params(params: &JungleStorybookTreeParams) -> Self {
		let mut geometry = params.geometry.clone();
		geometry.apply_jungle_preset();
		Self {
			chain: geometry.build_chain(),
			geometry,
			growth_spawn_fraction: params.growth_spawn_fraction,
			jungle_growth_radius_scale: params.jungle_growth_radius_scale,
		}
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

	fn height(&self) -> f32 {
		self.geometry.height()
	}

	/// Unit-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&JungleStorybookTreeParams::unit_from_num(num))
	}
}

impl VegetationComponents for JungleStorybookTree {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let nodes = match level {
			LodSceneLevel::High => stick_nodes_high(&self.chain),
			LodSceneLevel::Medium => stick_nodes_banded(&self.chain, MEDIUM_STICK_BANDS),
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
		let nodes: Vec<_> = foliage_nodes_for_level(
			&self.chain,
			level,
			self.growth_spawn_fraction,
			self.jungle_growth_radius_scale,
			self.leaf_radius_world(),
		)
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
		let a = JungleStorybookTree::unit_from_num(3);
		let b = JungleStorybookTree::unit_from_num(3);
		let c = JungleStorybookTree::unit_from_num(4);
		assert!((a.geometry.height() - 1.0).abs() < 1e-5);
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = JungleStorybookTreeParams::default();
		params.geometry.scale.tree_height = 8.0;
		params.geometry.scale.stalk_base_radius = Some(0.4);
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.geometry.height() - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius.unwrap() - 0.05).abs() < 1e-5);
		assert!(
			(unit.jungle_growth_radius_scale - DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE / 8.0).abs()
				< 1e-5
		);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_and_cheap_ball_collections() -> Result<()> {
		let tree = JungleStorybookTree::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		let cheap = foliage
			.iter()
			.filter(|n| matches!(n.geometry, FoliageGeometry::CheapBallCollection(_)))
			.count();
		assert_eq!(cheap, 1);
		assert!(!foliage.iter().any(|n| matches!(n.geometry, FoliageGeometry::CheapBall)));
		Ok(())
	}
}
