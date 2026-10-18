//! **High Bush Shoots** — multi-shoot bush as [`VegetationComponents`].
//!
//! [`HighBushShootsParams::build`] grows [`HighBushShootsShape::build_chain`] once into
//! [`HighBushShoots`]. Sticks emit per segment (High all / Medium subsample / Low sparse);
//! foliage follows [`HighBushFoliageStyle`] (default layered-ball; cheap-ball at High/Medium
//! with a layered-ball Low proxy) using the Common High Bush ball-selection rule
//! (terminals, upper canopy, or branch order > 1).
//!
//! [`HighBushShoots::unit_from_num`] / [`HighBushShootsParams::into_unit_from_num`]
//! normalize to unit height and key chain noise by a variant index so many plants
//! share one archetypal mesh (world size goes on [`Placement`] scale). Emission
//! folds sticks into a collection; cheap balls merge when that style is selected.
//! Layered balls stay separate nodes (shared GLBs).

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::{
	should_allocate_foliage, BallStickChain, HighBushChain, HighBushFoliageStyle,
	HighBushShootsShape,
};
use vegetation_components::{
	leaf_material_ref, stick_material_ref, FoliageNode, Layers, Placement, StickNode,
	StructuralLod, VegetationComponents,
};

use crate::storybook_tree::{merge_cheap_ball_foliage, merge_kit_sticks};

/// High when `distance / footprint_radius ≤` this.
const HIGH_BUSH_STRUCTURAL_HIGH_FACTOR: f32 = 30.0;
const HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR: f32 = 45.0;
const HIGH_BUSH_STRUCTURAL_LOW_FACTOR: f32 = 60.0;

/// Authoring / CLI parameters for High Bush Shoots (VegetationComponents path).
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct HighBushShootsParams {
	#[command(flatten, next_help_heading = "High Bush Shape")]
	pub shape: HighBushShootsShape,
}

impl Default for HighBushShootsParams {
	fn default() -> Self {
		Self { shape: HighBushShootsShape::default() }
	}
}

impl HighBushShootsParams {
	pub fn new(shape: HighBushShootsShape) -> Self {
		Self { shape }
	}

	pub fn build(&self) -> HighBushShoots {
		HighBushShoots::from_params(self)
	}

	/// Unit-height bush whose chain noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	///
	/// Lengths and radii on [`HighBushShootsShape`] are already height fractions.
	/// Only `height` is world-sized; world size returns for grove [`Placement`] scale.
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let mut shape = self.shape;
		let size = shape.height.max(1e-4);
		shape.height = 1.0;
		shape.chain_noise.seed = num as i32;
		(Self { shape }, size)
	}
}

/// Built high bush: shape plus grown ball-stick chain.
#[derive(Clone)]
pub struct HighBushShoots {
	pub shape: HighBushShootsShape,
	pub chain: BallStickChain<HighBushChain>,
}

impl HighBushShoots {
	pub fn from_params(params: &HighBushShootsParams) -> Self {
		Self { shape: params.shape.clone(), chain: params.shape.build_chain() }
	}

	/// Unit-height bush whose chain noise is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&HighBushShootsParams::unit_from_num(num))
	}

	fn leaf_radius_world(&self) -> f32 {
		self.shape.leaf_radius_world()
	}

	fn footprint_radius(&self) -> f32 {
		self.chain.footprint_radius_at_least(
			(self.shape.height * self.shape.segment_radius_fraction_hi).max(1e-3),
		)
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.shape.height * 0.5, 0.0)
	}

	/// Stick stride: High=1 (all), Medium=2, Low=4.
	fn stick_nodes(&self, stride: usize) -> Vec<StickNode> {
		let stride = stride.max(1);
		self.chain
			.segments()
			.enumerate()
			.filter(|(i, _)| i % stride == 0)
			.filter_map(|(_, seg)| {
				StickNode::from_segment(seg.start.position, seg.end.position, seg.start.radius)
			})
			.collect()
	}

	/// Foliage at RFC ball-selection joints; `stride` thins candidates.
	///
	/// [`HighBushFoliageStyle::LayeredBall`] is layered at every LOD. Cheap / plane-splay /
	/// tuft stay cheap-ball at High/Medium and use a layered-ball Low proxy.
	fn foliage_nodes(&self, stride: usize, low: bool) -> Vec<FoliageNode> {
		let stride = stride.max(1);
		let leaf_r = self.leaf_radius_world().max(1e-4);
		let sites: Vec<Vec3> = self
			.chain
			.nodes_with_hysteresis_enumerated()
			.filter(|(idx, _, hyst)| should_allocate_foliage(*idx, hyst, &self.chain))
			.map(|(_, node, _)| node.position)
			.collect();
		let layered = matches!(self.shape.foliage_style, HighBushFoliageStyle::LayeredBall) || low;
		sites
			.into_iter()
			.enumerate()
			.filter(|(i, _)| i % stride == 0)
			.map(|(_, position)| {
				let placement = Placement::foliage_uniform(position, leaf_r);
				if layered {
					FoliageNode::layered_ball(placement)
				} else {
					FoliageNode::cheap_ball(placement)
				}
			})
			.collect()
	}
}

impl VegetationComponents for HighBushShoots {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		let stride = match level {
			LodSceneLevel::High => 1,
			LodSceneLevel::Medium => 2,
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => 4,
		};
		let nodes: Vec<_> = self
			.stick_nodes(stride)
			.into_iter()
			.map(|n| n.with_material(stick_material_ref()))
			.collect();
		Layers::from_free(merge_kit_sticks(nodes))
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let (stride, low) = match level {
			LodSceneLevel::High => (1, false),
			LodSceneLevel::Medium => (2, false),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => (3, true),
		};
		// PlaneSplay / CheapBall / LayeredBall / Tuft all map to ball kits on the VC path.
		let nodes: Vec<_> = self
			.foliage_nodes(stride, low)
			.into_iter()
			.map(|n| n.with_material(leaf_material_ref()))
			.collect();
		Layers::from_free(merge_cheap_ball_foliage(nodes))
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				self.structural_center(),
				self.footprint_radius(),
				self.shape.height,
			)
			.with_factors(
				HIGH_BUSH_STRUCTURAL_HIGH_FACTOR,
				HIGH_BUSH_STRUCTURAL_MEDIUM_FACTOR,
				HIGH_BUSH_STRUCTURAL_LOW_FACTOR,
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
	fn default_vc_uses_layered_ball_style() -> Result<()> {
		assert_eq!(
			HighBushShootsParams::default().shape.foliage_style,
			HighBushFoliageStyle::LayeredBall
		);
		Ok(())
	}

	#[test]
	fn high_emits_sticks_and_canopy_layered_balls() -> Result<()> {
		let built = HighBushShootsParams::default().build();
		let sticks = built.stick_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(sticks.len(), 1);
		assert!(sticks[0].collection.is_some());
		let foliage = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!foliage.is_empty());
		// RFC ball selection fills more than graph terminals alone.
		let terminal_only = built
			.chain
			.nodes_with_hysteresis_enumerated()
			.filter(|(idx, _, _)| sbs_geometry::high_bush_is_graph_terminal(&built.chain, *idx))
			.count();
		assert!(foliage.len() >= terminal_only);
		assert!(foliage.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = HighBushShoots::unit_from_num(3);
		let b = HighBushShoots::unit_from_num(3);
		let c = HighBushShoots::unit_from_num(4);
		assert!((a.shape.height - 1.0).abs() < 1e-5);
		assert_eq!(a.shape.chain_noise.seed, 3);
		assert_eq!(a.shape.chain_noise.seed, b.shape.chain_noise.seed);
		assert_eq!(a.chain.nodes.len(), b.chain.nodes.len());
		assert_ne!(a.shape.chain_noise.seed, c.shape.chain_noise.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let mut params = HighBushShootsParams::default();
		params.shape.height = 8.0;
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 8.0).abs() < 1e-5);
		assert!((unit.shape.height - 1.0).abs() < 1e-5);
		assert_eq!(unit.shape.chain_noise.seed, 7);
		Ok(())
	}

	#[test]
	fn medium_subsamples_sticks_low_uses_layered() -> Result<()> {
		let built = HighBushShootsParams::default().build();
		let high = built.stick_nodes(1);
		let medium = built.stick_nodes(2);
		let low_sticks = built.stick_nodes(4);
		assert!(medium.len() <= high.len());
		assert!(low_sticks.len() <= medium.len());

		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert!(!low.is_empty());
		assert!(low.iter().all(|n| n.geometry.is_layered_ball()));
		Ok(())
	}

	#[test]
	fn cheap_ball_style_emits_cheap_at_high() -> Result<()> {
		let mut params = HighBushShootsParams::default();
		params.shape.foliage_style = HighBushFoliageStyle::CheapBall;
		let built = params.build();
		let high = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert_eq!(high.len(), 1);
		assert!(matches!(high[0].geometry, FoliageGeometry::CheapBallCollection(_)));
		Ok(())
	}

	#[test]
	fn structural_lod_from_footprint() -> Result<()> {
		let built = HighBushShootsParams::default().build();
		let probe = built.structural_lod().expect("probe");
		assert!(probe.tree_radius > 0.0);
		Ok(())
	}
}
