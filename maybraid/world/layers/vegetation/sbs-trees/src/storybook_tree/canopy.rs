//! Cheap-ball canopy on outer / terminal BranchOut joints, with torch-like LOD banding.
//!
//! Low adds a full-canopy layered proxy inset to 70% of the canopy horizontal radius.

use bevy::prelude::Vec3;
use sbs_geometry::chain::storybook_tree::is_graph_terminal;
use sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	StorybookTreeChain, StorybookTreePhase,
};
use vegetation_components::{FoliageNode, Placement};

/// High foliage: densest azimuth × height outer samples.
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

/// Medium sticks: ~20% more cells than shared torch medium (10×4 → 12×4).
pub(crate) const MEDIUM_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(12, 4);
/// Braid Oak Medium: another ~20% denser branch samples than storybook Medium (12×4 → 15×4).
pub(crate) const BRAID_MEDIUM_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(15, 4);

/// Full-canopy proxy sits inside the foliage AABB at this fraction of XZ half-extents.
const FULL_CANOPY_PROXY_RADIUS_SCALE: f32 = 0.70;

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
	let is_terminal = is_graph_terminal(chain, node_idx);
	let outer = hysteresis.distance_from_anchor
		> hysteresis.outer_foliage_distance_fraction * hysteresis.projection_length;
	is_terminal || outer
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

/// Full-canopy layered AABB proxy at 70% of horizontal canopy radius (full height).
fn full_canopy_proxy_ball(
	candidates: &[FoliageCandidate],
	leaf_radius_world: f32,
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
	let center = (min + max) * 0.5;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.x *= FULL_CANOPY_PROXY_RADIUS_SCALE;
	half_extents.z *= FULL_CANOPY_PROXY_RADIUS_SCALE;
	Some(FoliageNode::layered_ball(Placement::new(center, 0.0).with_scale(half_extents)))
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

/// Medium outer samples (no mass proxy).
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(chain, MEDIUM_FOLIAGE_BANDS, leaf_radius_world)
}

/// Medium outer samples plus a full-canopy layered proxy (used by Braid Oak).
pub(crate) fn foliage_nodes_medium_with_proxy(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let mut nodes = banded_from_candidates(&candidates, MEDIUM_FOLIAGE_BANDS, leaf_radius_world);
	if let Some(proxy) = full_canopy_proxy_ball(&candidates, leaf_radius_world) {
		nodes.push(proxy);
	}
	nodes
}

/// Coarse outer samples plus a full-canopy layered proxy (70% inset radius).
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain);
	let mut nodes = banded_from_candidates(&candidates, LOW_FOLIAGE_BANDS, leaf_radius_world);
	if let Some(proxy) = full_canopy_proxy_ball(&candidates, leaf_radius_world) {
		nodes.push(proxy);
	}
	nodes
}
