//! Stick segment → [`StickNode`] emission (with structural LOD phase filters).

use bevy::prelude::Vec3;
use sbs_geometry::{
	sample_max_horizontal_radius_by_azimuth_height, AzimuthHeightBands, BallStickSegment,
	SopesBanyanChain, SopesBanyanPhase,
};
use vegetation_components::{StickGeometry, StickNode};

/// Medium sticks: coarser azimuth × height outer samples than foliage (aggressive drop-off).
pub(crate) const MEDIUM_STICK_BANDS: AzimuthHeightBands = AzimuthHeightBands::new(6, 2);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StickLodRole {
	/// Strict stalk / trunk.
	Trunk,
	/// Descender aerial roots.
	Descender,
	/// Branch-out / flair sticks that connect successive ball-stick nodes.
	BetweenNodes,
}

impl StickLodRole {
	pub(crate) fn from_parent_phase(phase: &SopesBanyanPhase) -> Self {
		match phase {
			SopesBanyanPhase::Stalk(_) => Self::Trunk,
			SopesBanyanPhase::StartDescender(_) | SopesBanyanPhase::EndDescender(_) => {
				Self::Descender
			}
			SopesBanyanPhase::BranchOut(_)
			| SopesBanyanPhase::StartFlairUp(_)
			| SopesBanyanPhase::EndFlairUp(_) => Self::BetweenNodes,
		}
	}

	pub(crate) fn is_trunk(self) -> bool {
		matches!(self, Self::Trunk)
	}

	pub(crate) fn is_descender(self) -> bool {
		matches!(self, Self::Descender)
	}

	/// Trunk + descenders use trunk mesh LOD (length-biased) so they stay in frame longer.
	pub(crate) fn stick_geometry(self) -> StickGeometry {
		match self {
			Self::Trunk | Self::Descender => StickGeometry::Trunk,
			Self::BetweenNodes => StickGeometry::Segment,
		}
	}
}

/// Keep roughly this fraction of descender sticks on Low (stable every-Nth sample).
pub(crate) const LOW_DESCENDER_KEEP_EVERY: usize = 4;

pub(crate) fn stick_role_for_segment(
	_segment: &BallStickSegment<'_>,
	parent: &SopesBanyanChain,
) -> StickLodRole {
	StickLodRole::from_parent_phase(&parent.phase)
}

pub(crate) fn stick_node_for_segment(
	segment: &BallStickSegment<'_>,
	parent: &SopesBanyanChain,
) -> Option<StickNode> {
	let role = stick_role_for_segment(segment, parent);
	StickNode::from_segment_geometry(
		segment.start.position,
		segment.end.position,
		segment.start.radius,
		role.stick_geometry(),
	)
}

#[derive(Clone, Copy)]
struct StickBandCandidate {
	mid: Vec3,
	start: Vec3,
	end: Vec3,
	radius: f32,
	geometry: StickGeometry,
}

/// Trunk always + outermost non-trunk sticks per azimuth × height cell.
pub(crate) fn stick_nodes_medium_banded<'a, I>(segments: I) -> Vec<StickNode>
where
	I: IntoIterator<Item = (BallStickSegment<'a>, &'a SopesBanyanChain)>,
{
	let mut trunk = Vec::new();
	let mut candidates = Vec::new();
	for (segment, parent) in segments {
		let role = stick_role_for_segment(&segment, parent);
		if role.is_trunk() {
			if let Some(node) = stick_node_for_segment(&segment, parent) {
				trunk.push(node);
			}
			continue;
		}
		candidates.push(StickBandCandidate {
			mid: segment.midpoint(),
			start: segment.start.position,
			end: segment.end.position,
			radius: segment.start.radius,
			geometry: role.stick_geometry(),
		});
	}
	let sampled =
		sample_max_horizontal_radius_by_azimuth_height(&candidates, |c| c.mid, MEDIUM_STICK_BANDS);
	trunk.extend(sampled.into_iter().filter_map(|s| {
		StickNode::from_segment_geometry(s.item.start, s.item.end, s.item.radius, s.item.geometry)
	}));
	trunk
}

/// Low: trunk + a thinned subset of descenders (no branch / flair sticks).
pub(crate) fn keep_stick_on_low(role: StickLodRole, descender_index: &mut usize) -> bool {
	if role.is_trunk() {
		return true;
	}
	if !role.is_descender() {
		return false;
	}
	let keep = *descender_index % LOW_DESCENDER_KEEP_EVERY == 0;
	*descender_index += 1;
	keep
}
