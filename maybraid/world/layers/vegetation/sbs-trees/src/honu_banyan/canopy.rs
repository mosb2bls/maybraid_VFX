//! Honu canopy: jungle-growth clusters + cheap canopy balls (VegetationComponents).

use bevy::prelude::*;
use lod::scene::LodSceneLevel;
use sbs_geometry::chain::honu_banyan::{is_graph_terminal, HonuBanyanChain};
use sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use sbs_geometry::{AzimuthHeightBands, BallStickChain, BallStickNode};
use vegetation_components::{FoliageNode, Placement};

use crate::jungle_canopy_vc::{
	emit_jungle_canopy_lod, JungleCanopyLodPlan, JungleFoliageCandidate, JungleGrowthEmitMode,
	JungleGrowthScaleParams,
};

/// High / Medium outer / inner canopy bands.
const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(27, 8);
/// Low foliage: cheap-ball kit.
const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

/// Default [`crate::HonuBanyanParams::jungle_growth_radius_scale`] for full (~24 m) Honu.
pub const DEFAULT_HONU_GROWTH_RADIUS_SCALE: f32 = 10.0;
/// Reference stalk height that pairs with [`DEFAULT_HONU_GROWTH_RADIUS_SCALE`].
pub const HONU_GROWTH_REFERENCE_HEIGHT: f32 = 24.0;
/// Floor so tiny trees still get a readable growth cluster.
const MIN_HONU_GROWTH_RADIUS_SCALE: f32 = 0.35;

/// Scale jungle-growth assembly with tree height (mini Honu vs full canopy).
pub fn jungle_growth_radius_scale_for_height(tree_height: f32) -> f32 {
	let t = (tree_height.max(1e-3) / HONU_GROWTH_REFERENCE_HEIGHT).clamp(0.12, 1.0);
	(DEFAULT_HONU_GROWTH_RADIUS_SCALE * t).max(MIN_HONU_GROWTH_RADIUS_SCALE)
}

/// Frond scale relative to the assembly.
const FOLIAGE_SCALE_CENTER: f32 = 1.6;

const MIN_HEIGHT_FRACTION_FOR_FOLIAGE: f32 = 0.70;

/// Crown-height window for the Medium/Low proxy (fractions of span above crown floor).
const PROXY_CROWN_Y_START: f32 = 0.15;
const PROXY_CROWN_Y_END: f32 = 0.80;
const PROXY_HEIGHT_SCALE: f32 = 0.80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeFoliageKind {
	None,
	Growth,
	Canopy,
}

fn qualifies_for_foliage(
	hysteresis: &HonuBanyanChain,
	chain: &BallStickChain<HonuBanyanChain>,
	node_idx: usize,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !hysteresis.phase.is_canopy_limb() {
		return false;
	}
	hysteresis.height_fraction() > MIN_HEIGHT_FRACTION_FOR_FOLIAGE
		|| is_graph_terminal(chain, node_idx)
		|| hysteresis.branch_order() > 1
}

fn classify_node_foliage(
	growth_spawn_fraction: f32,
	node_idx: usize,
	node: &BallStickNode,
	hysteresis: &HonuBanyanChain,
	chain: &BallStickChain<HonuBanyanChain>,
) -> NodeFoliageKind {
	if !qualifies_for_foliage(hysteresis, chain, node_idx) {
		return NodeFoliageKind::None;
	}
	// Descenders keep a cheap canopy ball; growth is reserved for canopy limbs.
	if hysteresis.phase.is_descender_limb() {
		return NodeFoliageKind::Canopy;
	}
	if mix_seed_below_fraction(node_idx, node.position, growth_spawn_fraction) {
		NodeFoliageKind::Growth
	} else {
		NodeFoliageKind::Canopy
	}
}

fn collect_candidates(
	chain: &BallStickChain<HonuBanyanChain>,
	growth_spawn_fraction: f32,
	min_height: f32,
	leaf_radius_world: f32,
) -> (Vec<JungleFoliageCandidate>, Vec<JungleFoliageCandidate>) {
	let mut growth = Vec::new();
	let mut canopy = Vec::new();
	for (node_idx, node, h) in chain.nodes_with_hysteresis_enumerated() {
		if node.position.y < min_height {
			continue;
		}
		match classify_node_foliage(growth_spawn_fraction, node_idx, node, h, chain) {
			NodeFoliageKind::None => {}
			NodeFoliageKind::Growth => growth.push(JungleFoliageCandidate {
				position: node.position,
				node_idx,
				leaf_radius: leaf_radius_world,
			}),
			NodeFoliageKind::Canopy => canopy.push(JungleFoliageCandidate {
				position: node.position,
				node_idx,
				leaf_radius: leaf_radius_world,
			}),
		}
	}
	(growth, canopy)
}

fn mid_canopy_proxy_ball(
	chain: &BallStickChain<HonuBanyanChain>,
	min_height: f32,
	leaf_radius_world: f32,
) -> Option<FoliageNode> {
	let mut y_max = min_height;
	let mut any = false;
	for node in chain.nodes() {
		if node.position.y < min_height {
			continue;
		}
		any = true;
		y_max = y_max.max(node.position.y);
	}
	if !any {
		return None;
	}
	let span = (y_max - min_height).max(1e-4);
	let y_lo = min_height + span * PROXY_CROWN_Y_START;
	let y_hi = min_height + span * PROXY_CROWN_Y_END;

	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut mid_any = false;
	for node in chain.nodes() {
		let y = node.position.y;
		if y < y_lo || y > y_hi {
			continue;
		}
		let r = leaf_radius_world;
		let p = node.position;
		min = min.min(p - Vec3::splat(r));
		max = max.max(p + Vec3::splat(r));
		mid_any = true;
	}
	if !mid_any {
		return None;
	}
	let center = (min + max) * 0.5;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.y *= PROXY_HEIGHT_SCALE;
	Some(FoliageNode::layered_ball(Placement::new(center, 0.0).with_scale(half_extents)))
}

fn lod_plan(level: LodSceneLevel) -> JungleCanopyLodPlan {
	match level {
		// Medium keeps High growth/canopy topology; FrondCollection fine-phase merge thins.
		LodSceneLevel::High | LodSceneLevel::Medium => JungleCanopyLodPlan {
			growth: JungleGrowthEmitMode::All,
			canopy_bands: HIGH_FOLIAGE_BANDS,
			include_proxy: false,
		},
		LodSceneLevel::Low
		| LodSceneLevel::UltraLow
		| LodSceneLevel::Distance(_)
		| LodSceneLevel::Resolution(_) => JungleCanopyLodPlan {
			growth: JungleGrowthEmitMode::None,
			canopy_bands: LOW_FOLIAGE_BANDS,
			include_proxy: true,
		},
	}
}

/// Emit foliage for a structural LOD level (growth + banded canopy ± mid proxy).
pub(crate) fn foliage_nodes_for_level(
	chain: &BallStickChain<HonuBanyanChain>,
	level: LodSceneLevel,
	growth_spawn_fraction: f32,
	jungle_growth_radius_scale: f32,
	min_height: f32,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let plan = lod_plan(level);
	let (growth, canopy) =
		collect_candidates(chain, growth_spawn_fraction, min_height, leaf_radius_world);
	let proxy = if plan.include_proxy {
		mid_canopy_proxy_ball(chain, min_height, leaf_radius_world)
	} else {
		None
	};
	emit_jungle_canopy_lod(
		&growth,
		&canopy,
		plan,
		JungleGrowthScaleParams {
			radius_scale: jungle_growth_radius_scale,
			foliage_scale_center: FOLIAGE_SCALE_CENTER,
		},
		proxy,
	)
}

#[cfg(test)]
mod vc_growth_tests {
	use super::*;
	use crate::HonuBanyanParams;

	#[test]
	fn high_emits_jungle_growth_nodes() {
		let tree = HonuBanyanParams::default().build();
		let nodes = foliage_nodes_for_level(
			&tree.chain,
			LodSceneLevel::High,
			tree.growth_spawn_fraction,
			tree.jungle_growth_radius_scale,
			tree.geometry.crown_floor_world_y(),
			tree.geometry.leaf_ball_size(),
		);
		let fronds = nodes.iter().filter(|n| n.geometry.is_frond_collection()).count();
		assert!(
			fronds > 0,
			"expected jungle-growth frond collections, got {} foliage nodes",
			nodes.len()
		);
	}

	#[test]
	fn growth_radius_scales_down_for_mini_height() {
		let full = jungle_growth_radius_scale_for_height(HONU_GROWTH_REFERENCE_HEIGHT);
		assert!((full - DEFAULT_HONU_GROWTH_RADIUS_SCALE).abs() < 1e-4);
		let mini = jungle_growth_radius_scale_for_height(3.0);
		assert!(mini < full * 0.25);
		assert!(mini >= MIN_HONU_GROWTH_RADIUS_SCALE);
	}
}
