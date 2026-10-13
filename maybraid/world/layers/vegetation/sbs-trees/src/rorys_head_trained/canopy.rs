//! Joint canopy LOD: cheap balls on graph joints, with Penmarch-style banding.
//!
//! High / Medium / Low outer-sample only — no layered mass proxies.

use bevy::prelude::Vec3;
use sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickChain,
	StorybookTreeChain,
};
use vegetation_components::{FoliageNode, Placement};

/// High foliage: densest azimuth × height outer samples (still drops near-duplicates).
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(24, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

/// Slightly undersized vs node radius so limb gaps stay covered without dominating sticks.
const JOINT_CANOPY_BALL_SCALE: f32 = 0.88;

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
}

fn world_ball_radius(c: &FoliageCandidate, leaf_radius_world: f32) -> f32 {
	let scale = (leaf_radius_world / c.radius.max(1e-4)) * JOINT_CANOPY_BALL_SCALE;
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
		.nodes()
		.map(|node| FoliageCandidate { position: node.position, radius: node.radius })
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

/// Outermost joint foliage per azimuth × height cell.
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

/// Coarse outer samples (no mass proxy).
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<StorybookTreeChain>,
	leaf_radius_world: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(chain, LOW_FOLIAGE_BANDS, leaf_radius_world)
}
