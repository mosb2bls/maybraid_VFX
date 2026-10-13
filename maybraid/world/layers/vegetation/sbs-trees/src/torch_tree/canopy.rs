//! Selective torch canopy: cheap balls on upper/outer BranchOut nodes.
//!
//! High outer-samples densely. Medium samples plus a trunk-biased layered proxy
//! (smaller than Low's full-extent proxy). Low uses a coarser sample plus a
//! full-extent layered proxy.

use bevy::prelude::Vec3;
use sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	StorybookTreeChain, StorybookTreePhase,
};
use vegetation_components::{FoliageNode, Placement};

/// High foliage: densest azimuth × height outer samples (still drops near-duplicates).
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage: coarser sample (proxy ball supplies the bulk silhouette).
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

/// Medium proxy: fraction of full canopy AABB half-extents (Low uses 1.0).
const MEDIUM_PROXY_EXTENT_SCALE: f32 = 0.65;
/// Medium proxy: how far to pull the AABB center toward the trunk in XZ (`0` = AABB
/// center, `1` = on the Y axis).
const MEDIUM_PROXY_TRUNKWARD: f32 = 0.45;

/// RFC §3.1.7.4 ball selection: terminal, upper belt, or far along limb.
fn should_allocate_foliage(
	node_idx: usize,
	hysteresis: &StorybookTreeChain,
	chain: &BallStickChain<StorybookTreeChain>,
) -> bool {
	if hysteresis.projection_length < 1e-6 {
		return false;
	}
	if !matches!(hysteresis.phase, StorybookTreePhase::BranchOut(_)) {
		return false;
	}
	let is_terminal = chain.children.get(node_idx).is_some_and(|c| c.is_empty());
	let upper = hysteresis.ring_u > 0.55;
	let outer = hysteresis.distance_from_anchor > 0.70 * hysteresis.projection_length;
	is_terminal || upper || outer
}

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
}

fn world_ball_radius(c: &FoliageCandidate, leaf_radius_world: f32) -> f32 {
	let scale = leaf_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, leaf_radius_world: f32) -> FoliageNode {
	FoliageNode::cheap_ball(Placement::foliage_uniform(
		c.position,
		world_ball_radius(c, leaf_radius_world),
	))
}

fn collect_candidates(chain: &BallStickChain<StorybookTreeChain>) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, h)| {
			if !should_allocate_foliage(idx, h, chain) {
				return None;
			}
			Some(FoliageCandidate { position: node.position, radius: node.radius })
		})
		.collect()
}

/// Outermost foliage candidates per azimuth × height cell.
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	banded_from_candidates(&candidates, bands, leaf_radius_world)
}

fn banded_from_candidates(
	candidates: &[FoliageCandidate],
	bands: AzimuthHeightBands,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let sampled = sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, leaf_radius_world))
		.collect()
}

/// Layered ball from canopy AABB, optionally shrunken and pulled toward the trunk.
fn canopy_extent_proxy_ball(
	candidates: &[FoliageCandidate],
	leaf_radius_world: f32,
	extent_scale: f32,
	trunkward: f32,
) -> Option<FoliageNode> {
	if candidates.is_empty() {
		return None;
	}
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for c in candidates {
		let r = world_ball_radius(c, leaf_radius_world);
		let p = c.position;
		min = min.min(p - Vec3::splat(r));
		max = max.max(p + Vec3::splat(r));
	}
	let mut center = (min + max) * 0.5;
	let trunkward = trunkward.clamp(0.0, 1.0);
	center.x *= 1.0 - trunkward;
	center.z *= 1.0 - trunkward;
	let half_extents = ((max - min) * 0.5 * extent_scale.max(1e-3)).max(Vec3::splat(1e-4));
	Some(FoliageNode::layered_ball(Placement::new(center, 0.0).with_scale(half_extents)))
}

/// Medium samples plus a trunk-biased, reduced-extent layered proxy.
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let mut nodes = banded_from_candidates(&candidates, MEDIUM_FOLIAGE_BANDS, leaf_radius_world);
	if let Some(proxy) = canopy_extent_proxy_ball(
		&candidates,
		leaf_radius_world,
		MEDIUM_PROXY_EXTENT_SCALE,
		MEDIUM_PROXY_TRUNKWARD,
	) {
		nodes.push(proxy);
	}
	nodes
}

/// Coarse outer samples plus one full-extent canopy proxy ball.
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let mut nodes = banded_from_candidates(&candidates, LOW_FOLIAGE_BANDS, leaf_radius_world);
	if let Some(proxy) = canopy_extent_proxy_ball(&candidates, leaf_radius_world, 1.0, 0.0) {
		nodes.push(proxy);
	}
	nodes
}
