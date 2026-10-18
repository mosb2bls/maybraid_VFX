//! **Jungle Growth** — frond crown + upward spears as [`VegetationComponents`].
//!
//! Isolated counterpart to the Honu / Jungle Storybook cluster helper in
//! [`crate::jungle_growth_vc`]. Approximates jungle-growth foliage without the inner dirt/wood ball.

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use sbs_geometry::JungleGrowthShape;
use vegetation_components::{FoliageNode, Layers, StickNode, StructuralLod, VegetationComponents};

use crate::jungle_growth_vc::{jungle_growth_foliage_nodes, JungleGrowthVcParams};

const JUNGLE_GROWTH_STRUCTURAL_HIGH_FACTOR: f32 = 30.0;
const JUNGLE_GROWTH_STRUCTURAL_MEDIUM_FACTOR: f32 = 45.0;
const JUNGLE_GROWTH_STRUCTURAL_LOW_FACTOR: f32 = 60.0;

/// Authoring / CLI parameters for a single jungle-growth cluster.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct JungleGrowthParams {
	#[command(flatten, next_help_heading = "Jungle Growth Shape")]
	pub shape: JungleGrowthShape,
}

impl Default for JungleGrowthParams {
	fn default() -> Self {
		Self { shape: JungleGrowthShape::default() }
	}
}

impl JungleGrowthParams {
	pub fn new(shape: JungleGrowthShape) -> Self {
		Self { shape }
	}

	pub fn build(&self) -> JungleGrowth {
		JungleGrowth::from_params(self)
	}
}

/// Built jungle-growth cluster (frond collection at the origin).
#[derive(Clone, Debug)]
pub struct JungleGrowth {
	pub shape: JungleGrowthShape,
	nodes: Vec<FoliageNode>,
}

impl JungleGrowth {
	pub fn from_params(params: &JungleGrowthParams) -> Self {
		let shape = params.shape.clone();
		let vc = JungleGrowthVcParams {
			node_idx: 0,
			position: Vec3::ZERO,
			radius_scale: 1.0,
			foliage_scale: shape.foliage_scale.max(1e-4),
			seed: shape.seed,
		};
		Self { shape, nodes: jungle_growth_foliage_nodes(vc) }
	}

	fn footprint_radius(&self) -> f32 {
		(self.shape.foliage_scale.max(self.shape.inner_ball_scale) * 1.2).max(1e-3)
	}

	fn structural_center(&self) -> Vec3 {
		Vec3::new(0.0, self.footprint_radius() * 0.5, 0.0)
	}
}

impl VegetationComponents for JungleGrowth {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		let nodes = match level {
			LodSceneLevel::High | LodSceneLevel::Medium => self.nodes.clone(),
			LodSceneLevel::Low
			| LodSceneLevel::UltraLow
			| LodSceneLevel::Distance(_)
			| LodSceneLevel::Resolution(_) => self.nodes.iter().take(1).cloned().collect(),
		};
		Layers::from_free(nodes)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(StructuralLod::new(self.structural_center(), self.footprint_radius()).with_factors(
			JUNGLE_GROWTH_STRUCTURAL_HIGH_FACTOR,
			JUNGLE_GROWTH_STRUCTURAL_MEDIUM_FACTOR,
			JUNGLE_GROWTH_STRUCTURAL_LOW_FACTOR,
		))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn default_emits_frond_collection() -> Result<()> {
		let built = JungleGrowthParams::default().build();
		let high = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		assert!(!high.is_empty());
		assert!(high
			.iter()
			.any(|n| n.geometry.is_frond_collection() || n.geometry.is_frond_kit()));
		Ok(())
	}

	#[test]
	fn structural_lod_from_footprint() -> Result<()> {
		let built = JungleGrowthParams::default().build();
		let probe = built.structural_lod().expect("probe");
		assert!(probe.tree_radius > 0.0);
		Ok(())
	}
}
