//! Storybook / torch stick → [`StickNode`] emission (with structural LOD filters).
//!
//! High / Medium keep the stalk plus outermost branch sticks per azimuth × height
//! cell. Low keeps the stalk only.
//!
//! Sample position is the segment endpoint with larger horizontal radius (not the
//! midpoint). Steep / high-elevation sticks flare out along their length; midpoints
//! sit inward and lose the outer-radius contest.

use bevy::prelude::Vec3;
use sbs_geometry::{
	horizontal_radius_from_y_axis, sample_max_horizontal_radius_by_azimuth_height,
	AzimuthHeightBands, BallStickChain, BallStickSegment, StorybookTreeChain, StorybookTreePhase,
};
use vegetation_components::{Placement, StickNode};

/// High sticks: densest azimuth × height outer samples.
pub(crate) const HIGH_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(48, 16);
/// Medium sticks: another ~40% fewer cells than 13×5 (10×4).
pub(crate) const MEDIUM_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(10, 4);

fn is_stalk(parent: &StorybookTreeChain) -> bool {
	matches!(parent.phase, StorybookTreePhase::Stalk(_))
}

/// Point on the segment that contributes the silhouette (farther from the Y axis).
fn outermost_horizontal_point(start: Vec3, end: Vec3) -> Vec3 {
	if horizontal_radius_from_y_axis(end) >= horizontal_radius_from_y_axis(start) {
		end
	} else {
		start
	}
}

pub(crate) fn stick_node_for_segment(
	segment: &BallStickSegment<'_>,
	parent: &StorybookTreeChain,
) -> Option<StickNode> {
	let start = segment.start.position;
	let end = segment.end.position;
	let radius = segment.start.radius;
	let ray = end - start;
	let len_sq = ray.length_squared();
	if len_sq < 1e-12 {
		return None;
	}
	let length = len_sq.sqrt();
	let placement = Placement::stick_segment(start, ray, length, radius)?;
	if is_stalk(parent) {
		Some(StickNode::trunk(placement))
	} else {
		Some(StickNode::segment(placement))
	}
}

pub(crate) fn stick_nodes_high(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	stick_nodes_banded(chain, HIGH_STICK_BANDS)
}

#[derive(Clone, Copy)]
struct StickBandCandidate {
	/// Endpoint with larger horizontal radius — used for azimuth × height binning.
	sample_at: Vec3,
	start: Vec3,
	end: Vec3,
	radius: f32,
}

/// Stalk always + outermost non-stalk sticks per azimuth × height cell.
pub(crate) fn stick_nodes_banded(
	chain: &BallStickChain<StorybookTreeChain>,
	bands: AzimuthHeightBands,
) -> Vec<StickNode> {
	let mut trunk = Vec::new();
	let mut candidates = Vec::new();
	for (segment, parent, _) in chain.segments_with_hysteresis() {
		if is_stalk(parent) {
			if let Some(node) = stick_node_for_segment(&segment, parent) {
				trunk.push(node);
			}
			continue;
		}
		let start = segment.start.position;
		let end = segment.end.position;
		candidates.push(StickBandCandidate {
			sample_at: outermost_horizontal_point(start, end),
			start,
			end,
			radius: segment.start.radius,
		});
	}
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.sample_at, bands);
	trunk.extend(
		sampled
			.into_iter()
			.filter_map(|s| StickNode::from_segment(s.item.start, s.item.end, s.item.radius)),
	);
	trunk
}

pub(crate) fn stick_nodes_medium(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	stick_nodes_banded(chain, MEDIUM_STICK_BANDS)
}

/// Stalk / trunk segments only (no branch sticks).
pub(crate) fn stick_nodes_low(chain: &BallStickChain<StorybookTreeChain>) -> Vec<StickNode> {
	chain
		.segments_with_hysteresis()
		.filter_map(|(segment, parent, _)| {
			if !is_stalk(parent) {
				return None;
			}
			stick_node_for_segment(&segment, parent)
		})
		.collect()
}
