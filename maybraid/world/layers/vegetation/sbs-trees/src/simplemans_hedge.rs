//! **Simpleman's Hedge** — a dense, low ball-and-plane-splay mat over a small ground area
//! ([#320](https://github.com/ramate-io/maybraid/issues/320), [RFC §3.1.7.16](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/16-simpleman-s-hedge/README.md)).
//!
//! [`SimplemansHedgeParams::build`] grows clump anchors once into [`SimplemansHedge`], which
//! implements [`VegetationComponents`]: empty sticks; foliage is one layered ball plus one
//! cheap ball (splay silhouette) per clump.
//!
//! [`SimplemansHedge::unit_from_num`] / [`SimplemansHedgeParams::into_unit_from_num`]
//! normalize to unit height (footprint scales with it) and key clump scatter by a
//! variant index. Emission folds cheap-ball splays into one collection; layered
//! cores stay separate.

use bevy::prelude::*;
use clap::Args;
use lod::scene::LodSceneLevel;
use procedural_common::{NoiseConfig, NoiseParams};
use vegetation_components::{
	leaf_material_ref, FoliageNode, Layers, Placement, StickNode, StructuralLod,
	VegetationComponents,
};

use crate::storybook_tree::merge_cheap_ball_foliage;

/// Structural band edges as `distance / tree_radius` (High / Medium / Low).
const STRUCTURAL_HIGH_FACTOR: f32 = 8.0;
const STRUCTURAL_MEDIUM_FACTOR: f32 = 20.0;
const STRUCTURAL_LOW_FACTOR: f32 = 32.0;

/// Authoring / CLI parameters for Simpleman's Hedge.
#[derive(Component, Clone, Args, Debug, PartialEq)]
#[command(rename_all = "kebab-case")]
pub struct SimplemansHedgeParams {
	/// Number of ball-and-splay clumps scattered over the hedge.
	#[arg(long, default_value_t = 9)]
	pub clump_count: u32,

	/// Hedge band height `H` (m).
	#[arg(long, default_value_t = 1.2)]
	pub height: f32,

	/// Square hedge footprint side length (m) the clumps scatter within.
	#[arg(long, default_value_t = 1.2)]
	pub footprint_xz: f32,

	/// Fill density in `0.0..1.0`; modulates clump size and plate reach.
	#[arg(long, default_value_t = 0.5)]
	pub density: f32,

	/// Deterministic surface seed.
	#[arg(long, default_value_t = 0)]
	pub seed: u32,
}

impl Default for SimplemansHedgeParams {
	fn default() -> Self {
		Self { clump_count: 9, height: 1.2, footprint_xz: 1.2, density: 0.5, seed: 0 }
	}
}

impl SimplemansHedgeParams {
	pub fn new(height: f32, footprint_xz: f32, density: f32, seed: u32) -> Self {
		Self { clump_count: 9, height, footprint_xz, density, seed }
	}

	/// RFC `hedge_radius = 0.08 * H`, widened slightly by authored density.
	pub fn hedge_radius(&self) -> f32 {
		let density = self.density.clamp(0.0, 1.0);
		0.08 * self.height.max(0.1) * (0.9 + 0.35 * density)
	}

	pub fn clump_height(&self) -> f32 {
		self.height.max(0.1) * 0.42
	}

	pub fn clump_radius(&self) -> f32 {
		let density = self.density.clamp(0.0, 1.0);
		let footprint_radius = self.footprint_xz.max(0.1) * (0.20 + 0.08 * density);
		footprint_radius.max(self.hedge_radius() * 2.0)
	}

	/// Deterministic patch-local clump anchors, scattered within the XZ footprint.
	pub fn clump_anchors(&self) -> Vec<Vec3> {
		let config = NoiseConfig::new(NoiseParams::from_scalar(self.seed as f32, 1.0, 1.0, 1));
		let half = (self.footprint_xz * 0.18).max(0.0);
		(0..self.clump_count)
			.map(|i| {
				// Non-integer lanes keep samples off gradient-noise lattice points.
				let lane = (i as f32 + 0.5) * 3.7;
				let x = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 1.0);
				let z = config.sample_range_f32_4d(-half, half, lane, 0.0, 0.0, 2.0);
				Vec3::new(x, 0.0, z)
			})
			.collect()
	}

	/// Grounded layered-ball transform for clump `index` at `anchor`.
	pub fn clump_ball_transform(&self, index: u32, anchor: Vec3) -> Transform {
		let radius = self.clump_radius();
		let height = self.clump_height();
		let jitter = 0.95 + 0.06 * (index % 3) as f32;
		Transform {
			translation: anchor + Vec3::new(0.0, height, 0.0),
			rotation: Quat::IDENTITY,
			scale: Vec3::new(radius * 2.4 * jitter, height, radius * 1.9 * jitter),
		}
	}

	/// Grounded cheap-ball (splay silhouette) transform for clump `index` at `anchor`.
	///
	/// Scale is relative to splay core radius (`clump_radius * 0.55`); VegetationComponents
	/// multiplies by that core when placing the cheap-ball silhouette.
	pub fn clump_splay_transform(&self, index: u32, anchor: Vec3) -> Transform {
		let radius = self.clump_radius();
		let core_radius = radius * 0.55;
		let vertical_scale = (self.clump_height() / (core_radius * 2.0)).max(0.2);
		let horizontal_scale = 1.45 + 0.10 * (index % 2) as f32;
		Transform {
			translation: anchor + Vec3::new(0.0, core_radius * vertical_scale, 0.0),
			rotation: Quat::IDENTITY,
			scale: Vec3::new(horizontal_scale, vertical_scale, horizontal_scale * 0.9),
		}
	}

	/// Grow clump anchors once for presentation / LOD emission.
	pub fn build(&self) -> SimplemansHedge {
		SimplemansHedge::from_params(self)
	}

	/// Unit-height hedge whose scatter seed is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::default().into_unit_from_num(num).0
	}

	/// Normalize this params set to unit height keyed by `num`.
	///
	/// Returns `(unit_params, world_size)` where `world_size` is the pre-normalize
	/// height to apply on the plant [`Placement`](vegetation_components::Placement).
	pub fn into_unit_from_num(self, num: u32) -> (Self, f32) {
		let size = self.height.max(1e-4);
		let inv = 1.0 / size;
		(
			Self {
				clump_count: self.clump_count,
				height: 1.0,
				footprint_xz: (self.footprint_xz * inv).max(1e-4),
				density: self.density,
				seed: num,
			},
			size,
		)
	}
}

/// Built Simpleman's Hedge: params plus resolved clump anchors.
#[derive(Clone, Debug, PartialEq)]
pub struct SimplemansHedge {
	pub clump_count: u32,
	pub height: f32,
	pub footprint_xz: f32,
	pub density: f32,
	pub seed: u32,
	pub anchors: Vec<Vec3>,
}

impl SimplemansHedge {
	pub fn from_params(params: &SimplemansHedgeParams) -> Self {
		Self {
			clump_count: params.clump_count,
			height: params.height,
			footprint_xz: params.footprint_xz,
			density: params.density,
			seed: params.seed,
			anchors: params.clump_anchors(),
		}
	}

	/// Unit-height hedge whose scatter seed is keyed solely by `num`.
	pub fn unit_from_num(num: u32) -> Self {
		Self::from_params(&SimplemansHedgeParams::unit_from_num(num))
	}

	fn as_params(&self) -> SimplemansHedgeParams {
		SimplemansHedgeParams {
			clump_count: self.clump_count,
			height: self.height,
			footprint_xz: self.footprint_xz,
			density: self.density,
			seed: self.seed,
		}
	}

	/// RFC `hedge_radius = 0.08 * H`, widened slightly by authored density.
	pub fn hedge_radius(&self) -> f32 {
		self.as_params().hedge_radius()
	}

	pub fn clump_height(&self) -> f32 {
		self.as_params().clump_height()
	}

	pub fn clump_radius(&self) -> f32 {
		self.as_params().clump_radius()
	}

	pub fn clump_ball_transform(&self, index: u32, anchor: Vec3) -> Transform {
		self.as_params().clump_ball_transform(index, anchor)
	}

	pub fn clump_splay_transform(&self, index: u32, anchor: Vec3) -> Transform {
		self.as_params().clump_splay_transform(index, anchor)
	}

	fn clump_foliage_nodes(&self, index: u32, anchor: Vec3) -> [FoliageNode; 2] {
		let ball = self.clump_ball_transform(index, anchor);
		let splay = self.clump_splay_transform(index, anchor);
		let core_radius = self.clump_radius() * 0.55;
		[
			FoliageNode::layered_ball(Placement::new(ball.translation, 0.0).with_scale(ball.scale)),
			FoliageNode::cheap_ball(
				Placement::new(splay.translation, 0.0).with_scale(splay.scale * core_radius),
			),
		]
	}

	fn foliage_nodes_all_clumps(&self) -> Vec<FoliageNode> {
		self.anchors
			.iter()
			.enumerate()
			.flat_map(|(i, anchor)| self.clump_foliage_nodes(i as u32, *anchor))
			.collect()
	}

	/// Low: every other clump, balls only (no splay cheap-ball).
	fn foliage_nodes_low(&self) -> Vec<FoliageNode> {
		self.anchors
			.iter()
			.enumerate()
			.filter(|(i, _)| i % 2 == 0)
			.map(|(i, anchor)| {
				let ball = self.clump_ball_transform(i as u32, *anchor);
				FoliageNode::layered_ball(
					Placement::new(ball.translation, 0.0).with_scale(ball.scale),
				)
			})
			.collect()
	}
}

impl VegetationComponents for SimplemansHedge {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		match level {
			LodSceneLevel::High | LodSceneLevel::Medium => {
				let nodes: Vec<_> = self
					.foliage_nodes_all_clumps()
					.into_iter()
					.map(|n| n.with_material(leaf_material_ref()))
					.collect();
				Layers::from_free(merge_cheap_ball_foliage(nodes))
			}
			LodSceneLevel::Low => Layers::from_free(self.foliage_nodes_low())
				.map(|n| n.with_material(leaf_material_ref())),
			LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
				Layers::new()
			}
		}
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		Some(
			StructuralLod::from_extent(
				Vec3::new(0.0, self.height * 0.5, 0.0),
				self.footprint_xz * 0.5,
				self.height,
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

	fn hedge(seed: u32) -> SimplemansHedge {
		SimplemansHedgeParams { seed, ..SimplemansHedgeParams::default() }.build()
	}

	#[test]
	fn hedge_radius_follows_rfc() -> Result<()> {
		let h =
			SimplemansHedgeParams { height: 2.0, density: 0.0, ..SimplemansHedgeParams::default() }
				.build();
		assert!((h.hedge_radius() - 0.144).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn anchors_stay_within_patch_footprint() -> Result<()> {
		let hedge = hedge(7);
		assert_eq!(hedge.anchors.len(), hedge.clump_count as usize);
		let half = hedge.footprint_xz * 0.5;
		for anchor in &hedge.anchors {
			assert!(anchor.x.abs() <= half);
			assert!(anchor.z.abs() <= half);
			assert_eq!(anchor.y, 0.0);
		}
		Ok(())
	}

	#[test]
	fn anchors_are_deterministic_per_seed_and_scattered() -> Result<()> {
		let anchors = hedge(7).anchors;
		assert_eq!(anchors, hedge(7).anchors);
		let distinct = anchors
			.iter()
			.enumerate()
			.all(|(i, a)| anchors.iter().skip(i + 1).all(|b| a.distance(*b) > 1e-4));
		assert!(distinct, "expected scattered anchors, got {anchors:?}");
		Ok(())
	}

	#[test]
	fn clump_shapes_vary_by_size() -> Result<()> {
		let hedge = hedge(7);
		assert_ne!(
			hedge.clump_ball_transform(0, Vec3::ZERO).scale.x,
			hedge.clump_ball_transform(1, Vec3::ZERO).scale.x
		);
		Ok(())
	}

	#[test]
	fn clumps_sit_on_ground() -> Result<()> {
		let hedge = hedge(7);
		let anchor = Vec3::new(0.2, 0.0, -0.1);
		let ball = hedge.clump_ball_transform(0, anchor);
		let center_y = ball.translation.y - ball.scale.y * 0.5;
		let ball_base_y = center_y - ball.scale.y * 0.5;
		let splay = hedge.clump_splay_transform(0, anchor);
		let core_radius = hedge.clump_radius() * 0.55;
		let splay_base_y = splay.translation.y - core_radius * splay.scale.y;
		assert!(ball_base_y.abs() < 1e-4, "expected ball base at y=0, got {ball_base_y}");
		assert!(splay_base_y.abs() < 1e-4, "expected splay base at y=0, got {splay_base_y}");
		Ok(())
	}

	#[test]
	fn clumps_are_low_dense_and_overlapping() -> Result<()> {
		let hedge = hedge(7);
		assert_eq!(hedge.clump_count, 9);
		assert!(hedge.clump_height() < hedge.height * 0.5);

		let max_anchor_distance = hedge.anchors.iter().fold(0.0_f32, |max_distance, a| {
			hedge
				.anchors
				.iter()
				.fold(max_distance, |inner_max, b| inner_max.max(a.distance(*b)))
		});
		let min_ball_width = (0..hedge.clump_count)
			.map(|index| hedge.clump_ball_transform(index, Vec3::ZERO).scale.x)
			.fold(f32::INFINITY, f32::min);
		assert!(
			max_anchor_distance < min_ball_width,
			"expected dense overlapping hedge clumps, max anchor distance {max_anchor_distance} >= min ball width {min_ball_width}"
		);
		Ok(())
	}

	#[test]
	fn high_emits_merged_cheap_balls_and_layered_cores() -> Result<()> {
		let built = SimplemansHedge::unit_from_num(1);
		let high = built.foliage_nodes_for_level(LodSceneLevel::High).flatten();
		let cheap = high
			.iter()
			.filter(|n| matches!(n.geometry, FoliageGeometry::CheapBallCollection(_)))
			.count();
		let layered = high.iter().filter(|n| n.geometry.is_layered_ball()).count();
		assert_eq!(cheap, 1);
		assert_eq!(layered, built.clump_count as usize);
		let low = built.foliage_nodes_for_level(LodSceneLevel::Low).flatten();
		assert_eq!(low.len(), (built.clump_count as usize + 1) / 2);
		assert!(built.foliage_nodes_for_level(LodSceneLevel::UltraLow).flatten().is_empty());
		Ok(())
	}

	#[test]
	fn unit_from_num_is_unit_height_and_stable() -> Result<()> {
		let a = SimplemansHedge::unit_from_num(3);
		let b = SimplemansHedge::unit_from_num(3);
		let c = SimplemansHedge::unit_from_num(4);
		assert!((a.height - 1.0).abs() < 1e-5);
		assert_eq!(a.seed, 3);
		assert_eq!(a.anchors, b.anchors);
		assert_ne!(a.seed, c.seed);
		Ok(())
	}

	#[test]
	fn into_unit_from_num_returns_world_size() -> Result<()> {
		let params = SimplemansHedgeParams {
			height: 2.4,
			footprint_xz: 3.6,
			..SimplemansHedgeParams::default()
		};
		let (unit, size) = params.into_unit_from_num(7);
		assert!((size - 2.4).abs() < 1e-5);
		assert!((unit.height - 1.0).abs() < 1e-5);
		assert!((unit.footprint_xz - 1.5).abs() < 1e-5);
		assert_eq!(unit.seed, 7);
		Ok(())
	}
}
