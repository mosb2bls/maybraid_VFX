//! Terminal canopy: layered / cheap ball foliage nodes (with structural LOD filters).

use bevy::prelude::Vec3;
use sbs_geometry::{AzimuthHeightBands, BallStickChain, BallStickNode, SopesBanyanChain};
use vegetation_components::{FoliageNode, Placement};

/// High foliage: dense azimuth × height outer samples (~+10% cells vs prior 24×8).
pub(crate) const HIGH_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(27, 8);
/// Medium foliage: ~40% more cells than the prior 12×4 grid.
pub(crate) const MEDIUM_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(17, 4);
/// Low foliage: cheap-ball kit; ~20% fewer cells than the prior 8×2 grid.
pub(crate) const LOW_FOLIAGE_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

/// Crown-height window for the Medium/Low proxy (fractions of span above crown floor).
const PROXY_CROWN_Y_START: f32 = 0.15;
const PROXY_CROWN_Y_END: f32 = 0.80;
/// Scale proxy half-height about the mid-canopy center.
const PROXY_HEIGHT_SCALE: f32 = 0.80;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CanopyBallKit {
	Layered,
	Cheap,
}

fn world_leaf_radius(node: &BallStickNode, leaf_radius_world: f32) -> f32 {
	let scale = leaf_radius_world / node.radius.max(1e-4);
	node.radius * scale
}

pub(crate) fn foliage_node_for_terminal(
	node: &BallStickNode,
	min_height: f32,
	leaf_radius_world: f32,
	kit: CanopyBallKit,
) -> Option<FoliageNode> {
	if node.position.y < min_height {
		return None;
	}
	let radius = world_leaf_radius(node, leaf_radius_world);
	let placement = Placement::foliage_uniform(node.position, radius);
	Some(match kit {
		CanopyBallKit::Layered => FoliageNode::layered_ball(placement),
		CanopyBallKit::Cheap => FoliageNode::cheap_ball(placement),
	})
}

/// Outermost chain nodes per azimuth × height cell → canopy balls (crown floor filtered).
pub(crate) fn banded_outer_canopy_balls(
	chain: &BallStickChain<SopesBanyanChain>,
	bands: AzimuthHeightBands,
	min_height: f32,
	leaf_radius_world: f32,
	kit: CanopyBallKit,
) -> Vec<FoliageNode> {
	chain
		.sample_radius_azimuth(bands)
		.into_iter()
		.filter_map(|sample| {
			foliage_node_for_terminal(sample.item, min_height, leaf_radius_world, kit)
		})
		.collect()
}

/// One layered ball covering the mid-crown AABB (fills Medium/Low mass).
pub(crate) fn mid_canopy_proxy_ball(
	chain: &BallStickChain<SopesBanyanChain>,
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
		let r = world_leaf_radius(node, leaf_radius_world);
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

/// Banded samples plus one mid-canopy layered proxy.
pub(crate) fn banded_outer_canopy_with_proxy(
	chain: &BallStickChain<SopesBanyanChain>,
	bands: AzimuthHeightBands,
	min_height: f32,
	leaf_radius_world: f32,
	kit: CanopyBallKit,
) -> Vec<FoliageNode> {
	let mut nodes = banded_outer_canopy_balls(chain, bands, min_height, leaf_radius_world, kit);
	if let Some(proxy) = mid_canopy_proxy_ball(chain, min_height, leaf_radius_world) {
		nodes.push(proxy);
	}
	nodes
}
