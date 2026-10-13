//! Cheap-ball joint canopy plus optional stalk-tip apex ([#232](https://github.com/ramate-io/maybraid/issues/232)).
//!
//! Medium uses ~30% fewer band cells. Medium and Low share a thin layered proxy that
//! stretches full canopy height (top-anchored) with reduced XZ extent. Low emits the
//! proxy twice at a 90° yaw offset for cross-layered thickness.

use bevy::prelude::Vec3;
use sbs_geometry::render::mix_seed::mix_seed_below_fraction;
use sbs_geometry::{
	liams_stalk_tip_from_chain, sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands,
	BallStickChain, LiamsConiferChain,
};
use vegetation_components::{FoliageNode, Placement};

/// Needle-cluster world radius as a fraction of stalk height (RFC `0.018 * H`; playground default wider).
pub const NORTHERN_SPLAY_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.048;

/// Kept for Params/CLI defaults (historical plane-splay sizing; samples are cheap balls now).
pub const NORTHERN_SPLAY_CORE_RADIUS: f32 = 0.75;
pub const NORTHERN_SPLAY_LEAF_DISC_RADIUS: f32 = 0.95;

/// High foliage: densest azimuth × height outer samples.
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium foliage: ~30% fewer cells than the prior 24×8 grid.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(17, 8);
/// Low foliage: coarser outer samples.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(8, 3);

/// Liam's Medium foliage: Northern Medium cell count × ~1.1 (17×8 → 19×8).
pub(crate) const LIAMS_MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(19, 8);
/// Liam's Low foliage: Northern Low cell count × ~1.1 (8×3 → 9×3).
pub(crate) const LIAMS_LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(9, 3);

/// Horizontal (XZ) scale of the thin full-height canopy proxy.
const THIN_PROXY_XZ_SCALE: f32 = 0.40;
/// Extra vertical stretch beyond the candidate AABB (top held at canopy tip).
const PROXY_HEIGHT_STRETCH: f32 = 1.20;

#[derive(Clone, Copy)]
struct FoliageCandidate {
	position: Vec3,
	radius: f32,
}

fn world_ball_radius(c: &FoliageCandidate, splay_radius_world: f32) -> f32 {
	let scale = splay_radius_world / c.radius.max(1e-4);
	c.radius * scale
}

fn foliage_node_from_candidate(c: &FoliageCandidate, splay_radius_world: f32) -> FoliageNode {
	FoliageNode::cheap_ball(Placement::foliage_uniform(
		c.position,
		world_ball_radius(c, splay_radius_world),
	))
}

fn collect_candidates(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_spawn_fraction: f32,
) -> Vec<FoliageCandidate> {
	chain
		.nodes_with_hysteresis_enumerated()
		.filter_map(|(idx, node, _)| {
			if !mix_seed_below_fraction(idx, node.position, splay_spawn_fraction) {
				return None;
			}
			Some(FoliageCandidate { position: node.position, radius: node.radius })
		})
		.collect()
}

fn banded_from_candidates(
	candidates: &[FoliageCandidate],
	bands: AzimuthHeightBands,
	splay_radius_world: f32,
) -> Vec<FoliageNode> {
	let sampled = sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.position, bands);
	sampled
		.into_iter()
		.map(|s| foliage_node_from_candidate(s.item, splay_radius_world))
		.collect()
}

fn maybe_apex_ball(
	chain: &BallStickChain<LiamsConiferChain>,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Option<FoliageNode> {
	let tip = liams_stalk_tip_from_chain(chain);
	if !mix_seed_below_fraction(0, tip.position, apex_spawn_fraction) {
		return None;
	}
	Some(FoliageNode::cheap_ball(Placement::foliage_uniform(tip.position, apex_radius_world)))
}

/// Thin layered proxy: full canopy height (top-anchored at stalk tip), reduced XZ.
fn thin_full_height_proxy_ball(
	candidates: &[FoliageCandidate],
	splay_radius_world: f32,
	chain: &BallStickChain<LiamsConiferChain>,
	yaw: f32,
) -> Option<FoliageNode> {
	if candidates.is_empty() {
		return None;
	}
	let tip = liams_stalk_tip_from_chain(chain);
	let mut min = Vec3::splat(f32::INFINITY);
	let mut max = Vec3::splat(f32::NEG_INFINITY);
	for c in candidates {
		let r = world_ball_radius(c, splay_radius_world);
		let p = c.position;
		min = min.min(p - Vec3::splat(r));
		max = max.max(p + Vec3::splat(r));
	}
	// Ensure the proxy envelope includes the crown tip.
	max.y = max.y.max(tip.position.y + apex_pad(splay_radius_world));
	let top_y = max.y;
	let mut half_extents = ((max - min) * 0.5).max(Vec3::splat(1e-4));
	half_extents.x *= THIN_PROXY_XZ_SCALE;
	half_extents.z *= THIN_PROXY_XZ_SCALE;
	half_extents.y *= PROXY_HEIGHT_STRETCH;
	let mut center = (min + max) * 0.5;
	// Hold the top of the stretched proxy at the canopy tip.
	center.y = top_y - half_extents.y;
	Some(FoliageNode::layered_ball(Placement::new(center, yaw).with_scale(half_extents)))
}

fn apex_pad(splay_radius_world: f32) -> f32 {
	splay_radius_world.max(1e-4) * 0.25
}

fn with_proxy_and_apex(
	candidates: &[FoliageCandidate],
	bands: AzimuthHeightBands,
	splay_radius_world: f32,
	chain: &BallStickChain<LiamsConiferChain>,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
	// When 2+, emit proxies at 0, π/2, … for cross-layered thickness.
	proxy_count: usize,
) -> Vec<FoliageNode> {
	let mut nodes = banded_from_candidates(candidates, bands, splay_radius_world);
	let n = proxy_count.max(1);
	for i in 0..n {
		let yaw = if n > 1 { std::f32::consts::FRAC_PI_2 * (i as f32) } else { 0.0 };
		if let Some(proxy) = thin_full_height_proxy_ball(candidates, splay_radius_world, chain, yaw)
		{
			nodes.push(proxy);
		}
	}
	if let Some(apex) = maybe_apex_ball(chain, apex_spawn_fraction, apex_radius_world) {
		nodes.push(apex);
	}
	nodes
}

/// Banded joint cheap-ball foliage; apex when spawn fraction passes (High: no mass proxy).
pub(crate) fn foliage_nodes_banded(
	chain: &BallStickChain<LiamsConiferChain>,
	bands: AzimuthHeightBands,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	let mut nodes = banded_from_candidates(&candidates, bands, splay_radius_world);
	if let Some(apex) = maybe_apex_ball(chain, apex_spawn_fraction, apex_radius_world) {
		nodes.push(apex);
	}
	nodes
}

/// Medium samples (~30% fewer cells) plus thin top-anchored proxy and optional apex.
pub(crate) fn foliage_nodes_medium(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	with_proxy_and_apex(
		&candidates,
		MEDIUM_FOLIAGE_BANDS,
		splay_radius_world,
		chain,
		apex_spawn_fraction,
		apex_radius_world,
		1,
	)
}

/// Medium banded samples only (no mass proxy) — arid / Liam thinning.
pub(crate) fn foliage_nodes_medium_no_proxy(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
) -> Vec<FoliageNode> {
	foliage_nodes_banded(
		chain,
		LIAMS_MEDIUM_FOLIAGE_BANDS,
		splay_radius_world,
		splay_spawn_fraction,
		0.0,
		0.0,
	)
}

/// Coarse samples plus the same thin proxy doubled, and optional apex.
pub(crate) fn foliage_nodes_low(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
	apex_spawn_fraction: f32,
	apex_radius_world: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	with_proxy_and_apex(
		&candidates,
		LOW_FOLIAGE_BANDS,
		splay_radius_world,
		chain,
		apex_spawn_fraction,
		apex_radius_world,
		2,
	)
}

/// Coarse samples plus a single thin full-height proxy (arid / Liam Low).
pub(crate) fn foliage_nodes_low_single_proxy(
	chain: &BallStickChain<LiamsConiferChain>,
	splay_radius_world: f32,
	splay_spawn_fraction: f32,
) -> Vec<FoliageNode> {
	let candidates = collect_candidates(chain, splay_spawn_fraction);
	with_proxy_and_apex(
		&candidates,
		LIAMS_LOW_FOLIAGE_BANDS,
		splay_radius_world,
		chain,
		0.0,
		0.0,
		1,
	)
}
