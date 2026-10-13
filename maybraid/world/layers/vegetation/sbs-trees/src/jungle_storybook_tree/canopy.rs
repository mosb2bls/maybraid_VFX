//! Jungle Storybook foliage allocation ([#235](https://github.com/ramate-io/maybraid/issues/235)).
//!
//! VegetationComponents:
//! - **Growth** — epiphyte clusters (palm fronds + spears).
//! - **Canopy** — cheap balls on remaining foliage-eligible sites.

use bevy::prelude::*;
use lod::scene::LodSceneLevel;
use sbs_geometry::chain::storybook_tree::{
	is_graph_terminal, StorybookTreeChain, StorybookTreePhase,
};
use sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use sbs_geometry::{BallStickChain, BallStickNode};
use vegetation_components::{FoliageNode, Placement};

use crate::jungle_canopy_vc::{
	emit_jungle_canopy_lod, JungleCanopyLodPlan, JungleFoliageCandidate, JungleGrowthEmitMode,
	JungleGrowthScaleParams,
};
use crate::storybook_tree::canopy::{HIGH_FOLIAGE_BANDS, LOW_FOLIAGE_BANDS};

/// Default [`crate::JungleStorybookTreeParams::jungle_growth_radius_scale`].
pub const DEFAULT_JUNGLE_GROWTH_RADIUS_SCALE: f32 = 4.0;

/// Minimum ring parameter `u` before a node may receive jungle growth (keeps trunk base clear).
const MIN_RING_U_FOR_GROWTH: f32 = 0.28;

/// Minimum ring `u` for any foliage when not terminal / high branch-order (RFC §3.1.7.13 inner fill).
const MIN_RING_U_FOR_FOLIAGE: f32 = 0.40;

const FOLIAGE_SCALE_CENTER: f32 = 1.2;
const FULL_CANOPY_PROXY_RADIUS_SCALE: f32 = 0.70;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NodeFoliageKind {
	None,
	Growth,
	Canopy,
}

/// Branch-out nodes with non-zero projection that participate in the canopy pass.
fn qualifies_for_foliage(
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
	node_idx: usize,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	hysteresis.ring_u > MIN_RING_U_FOR_FOLIAGE
		|| is_graph_terminal(chain, node_idx)
		|| hysteresis.branch_order() > 1
}

fn classify_node_foliage(
	growth_spawn_fraction: f32,
	node_idx: usize,
	node: &BallStickNode,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> NodeFoliageKind {
	if !qualifies_for_foliage(hysteresis, chain, node_idx) {
		return NodeFoliageKind::None;
	}
	if hysteresis.ring_u >= MIN_RING_U_FOR_GROWTH
		&& mix_seed_below_fraction(node_idx, node.position, growth_spawn_fraction)
	{
		NodeFoliageKind::Growth
	} else {
		NodeFoliageKind::Canopy
	}
}

fn collect_candidates(
	chain: &BallStickChain<StorybookTreeChain>,
	growth_spawn_fraction: f32,
	leaf_radius_world: f32,
) -> (Vec<JungleFoliageCandidate>, Vec<JungleFoliageCandidate>) {
	let mut growth = Vec::new();
	let mut canopy = Vec::new();
	for (node_idx, node, h) in chain.nodes_with_hysteresis_enumerated() {
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

fn full_canopy_proxy(
	growth: &[JungleFoliageCandidate],
	canopy: &[JungleFoliageCandidate],
) -> Option<FoliageNode> {
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for c in growth.iter().chain(canopy.iter()) {
		let r = c.leaf_radius;
		min = min.min(c.position - Vec3::splat(r));
		max = max.max(c.position + Vec3::splat(r));
		any = true;
	}
	if !any {
		return None;
	}
	let center = (min + max) * 0.5;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.x *= FULL_CANOPY_PROXY_RADIUS_SCALE;
	half_extents.z *= FULL_CANOPY_PROXY_RADIUS_SCALE;
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

/// Emit foliage for a structural LOD level (growth + banded canopy ± full proxy).
pub(crate) fn foliage_nodes_for_level(
	chain: &BallStickChain<StorybookTreeChain>,
	level: LodSceneLevel,
	growth_spawn_fraction: f32,
	jungle_growth_radius_scale: f32,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let plan = lod_plan(level);
	let (growth, canopy) = collect_candidates(chain, growth_spawn_fraction, leaf_radius_world);
	let proxy = if plan.include_proxy { full_canopy_proxy(&growth, &canopy) } else { None };
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
