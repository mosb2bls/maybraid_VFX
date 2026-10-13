//! **Sope's Banyan** — end-to-end tree assembly for Chico ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! [`SopesBanyanParams::build`] grows the ball-stick chain once into [`SopesBanyan`],
//! which implements [`VegetationComponents`].
//!
//! [`SopesBanyan::unit_from_num`] / [`SopesBanyanParams::into_unit_from_num`] normalize
//! to unit stalk height (canopy height, base radius, and meter perturbation scale
//! with it) and key layout noise by a variant index. Chain flair hops and stick
//! radii are fractions of stalk height so the grown mesh is actually unit-sized.
//! Emission folds sticks into a collection; High/Medium
//! layered canopy stays separate. Cheap-ball Low foliage merges.
//!
//! Structural LOD (tree-radius bands):
//! - **High** — within `8 ×` tree radius: full sticks; dense azimuth×height layered canopy
//! - **Medium** — `8…20 ×` radius: trunk + band-sampled sticks; layered outer foliage + mid proxy
//! - **Low** — `20…32 ×` radius: trunk + ~1/4 descenders; cheap-ball outer foliage + mid proxy

mod canopy;
mod stick;

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{BallStickChain, SopesBanyanChain, SopesBanyanSbs};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};
use canopy::{
	banded_outer_canopy_balls, banded_outer_canopy_with_proxy, CanopyBallKit, HIGH_FOLIAGE_BANDS,
	LOW_FOLIAGE_BANDS, MEDIUM_FOLIAGE_BANDS,
};
use stick::{
	keep_stick_on_low, stick_node_for_segment, stick_nodes_medium_banded, stick_role_for_segment,
};

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 10.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 30.0;
const STRUCTURAL_LOW_FACTOR: f32 = 50.0;

/// Authoring / CLI parameters for Sope's Banyan.
#[derive(Component, Clone, Args, Debug)]
#[command(rename_all = "kebab-case")]
pub struct SopesBanyanParams {
	/// Scale, anchors, growth, and topology noise for the ball-stick geometry.
	#[command(flatten, next_help_heading = "Geometry")]
	pub geometry: SopesBanyanSbs,
}

impl Default for SopesBanyanParams {
	fn default() -> Self {
		Self { geometry: SopesBanyanSbs::default() }
	}
}

impl SopesBanyanParams {
	/// Grow the ball-stick chain once for presentation / LOD emission.
	pub fn build(&self) -> SopesBanyan {
		SopesBanyan::from_params(self)
	}

	/// Unit-stalk-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit stalk height keyed by `num`.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut geometry = self.geometry;
		let size = geometry.scale.stalk_height.max(1e-4);
		let inv = 1.0 / size;
		geometry.scale.stalk_height = 1.0;
		geometry.scale.canopy_height = (geometry.scale.canopy_height * inv).max(1e-4);
		geometry.scale.stalk_base_radius = (geometry.scale.stalk_base_radius * inv).max(1e-6);
		let v = geometry.anchor_perturbation.vertical_offset;
		geometry.anchor_perturbation.vertical_offset =
			procedural_common::UnitRange::new(v.start * inv, v.end * inv);
		let r = geometry.anchor_perturbation.radius_offset;
		geometry.anchor_perturbation.radius_offset =
			procedural_common::UnitRange::new(r.start * inv, r.end * inv);
		geometry.canopy_noise.seed = num as i32;
		geometry.anchor_perturbation.noise.seed = num as i32;
		(Self { geometry }, size)
	}
}

/// Built Sope's Banyan: params plus a single grown [`BallStickChain`].
#[derive(Clone)]
pub struct SopesBanyan {
	pub geometry: SopesBanyanSbs,
	pub chain: BallStickChain<SopesBanyanChain>,
}

impl SopesBanyan {
	pub fn from_params(params: &SopesBanyanParams) -> Self {
		Self { geometry: params.geometry.clone(), chain: params.geometry.build_chain() }
	}

	/// Unit-stalk-height tree whose layout noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&SopesBanyanParams::unit_from_num(num))
	}

	fn footprint_radius(&self) -> f32 {
		self.chain
			.footprint_radius_at_least(self.geometry.scale.stalk_base_radius.max(1e-3))
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.geometry.scale.stalk_height * 0.5, 0.0)
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

	fn foliage_nodes_high(&self) -> Vec<FoliageNode> {
		banded_outer_canopy_balls(
			&self.chain,
			HIGH_FOLIAGE_BANDS,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
			CanopyBallKit::Layered,
		)
	}

	fn foliage_nodes_medium(&self) -> Vec<FoliageNode> {
		banded_outer_canopy_with_proxy(
			&self.chain,
			MEDIUM_FOLIAGE_BANDS,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
			CanopyBallKit::Layered,
		)
	}

	fn foliage_nodes_low(&self) -> Vec<FoliageNode> {
		banded_outer_canopy_with_proxy(
			&self.chain,
			LOW_FOLIAGE_BANDS,
			self.geometry.crown_floor_world_y(),
			self.geometry.leaf_ball_size(),
			CanopyBallKit::Cheap,
		)
	}
}

impl VegetationComponents for SopesBanyan {
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
		let nodes = match level {
			LodSceneLevel::High => self.foliage_nodes_high(),
			LodSceneLevel::Medium => self.foliage_nodes_medium(),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.foliage_nodes_low(),
		};
		let nodes: Vec<_> =
			nodes.into_iter().map(|n| n.with_material(leaf_material_ref())).collect();
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

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = SopesBanyan::unit_from_num(3);
		let b = SopesBanyan::unit_from_num(3);
		let c = SopesBanyan::unit_from_num(4);
		assert!((a.geometry.scale.stalk_height - 1.0).abs() < 1e-5);
		let max_y = a.chain.nodes.iter().map(|n| n.position.y).fold(0.0f32, f32::max);
		let max_xz = a.chain.nodes.iter().map(|n| n.horizontal_radius()).fold(0.0f32, f32::max);
		assert!(max_y < 2.5, "unit Sope chain height {max_y} should stay near 1");
		assert!(max_xz < 3.0, "unit Sope footprint {max_xz} should stay near 1");
		assert_eq!(a.geometry.canopy_noise.seed, 3);
		assert_eq!(a.geometry.canopy_noise.seed, b.geometry.canopy_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.geometry.canopy_noise.seed, c.geometry.canopy_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = SopesBanyanParams::default();
		params.geometry.scale.stalk_height = 20.0;
		params.geometry.scale.canopy_height = 40.0;
		params.geometry.scale.stalk_base_radius = 0.8;
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 20.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_height - 1.0).abs() < 1e-5);
		assert!((unit.geometry.scale.canopy_height - 2.0).abs() < 1e-5);
		assert!((unit.geometry.scale.stalk_base_radius - 0.04).abs() < 1e-5);
		assert!((unit.geometry.anchor_perturbation.vertical_offset.start + 0.05).abs() < 1e-5);
		assert!((unit.geometry.anchor_perturbation.vertical_offset.end - 0.05).abs() < 1e-5);
		assert_eq!(unit.geometry.canopy_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn high_emits_merged_stick_collection() -> Result<()> {
		let tree = SopesBanyan::unit_from_num(1);
		let sticks = tree.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = tree.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!foliage.is_empty());
		assert!(foliage.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}
}
