//! Crook-cylinder stick helpers for Braid Oak VegetationComponents.
//!
//! High and Medium share the same crook trunk polylines. High crooks banded outer branches;
//! Medium draws those branches as straight sticks. Low is stalk-only (straight).

use std::f32::consts::PI;

use bevy::prelude::*;
use procedural_common::{NoiseConfig, NoiseParams};
use sbs_geometry::{
	horizontal_radius_from_y_axis, sample_max_horizontal_radius_by_azimuth_height,
	AzimuthHeightBands, BallStickChain, BallStickSegment, StorybookTreeChain, StorybookTreePhase,
};
use sdf_common::CrookCylinder;
use vegetation_components::{StickGeometry, StickNode};

/// Base crook strength on the stalk (maps to ~`0.10` SDF radius via the crook cylinder).
const STALK_BEND_STRENGTH: f32 = 10.0;
/// Branch base strength at the lowest ring; rises with [`StorybookTreeChain::ring_u`].
const BRANCH_BEND_STRENGTH_BASE: f32 = 14.0;
const BRANCH_BEND_STRENGTH_RING_GAIN: f32 = 10.0;
/// Multiplier on signed stick-surface noise sample.
const BEND_STRENGTH_NOISE_GAIN: f32 = 0.40;
const MIN_BEND_STRENGTH: f32 = 4.0;
/// Unit-stick target base radius used by crook XZ scale (`0.5 / sdf_base_radius`).
const UNIT_TARGET_BASE_RADIUS: f32 = 0.5;
const UNIT_TARGET_TOP_RADIUS: f32 = 0.42;
const SDF_BEND_X: f32 = 0.12;
const SDF_BEND_Z: f32 = 0.08;
const MIN_CROOK_BEND_STRENGTH: f32 = 1e-4;
const MIN_BOUNDS_MARGIN: f32 = 0.06;
const PHASE_Z_KEY_ROTATE: u32 = 7;
/// Samples along the crook centerline for High stick polylines (`t = 0..=1`).
/// Three samples → two stick segments (enough crook read up close, fewer sticks).
const CROOK_POLYLINE_SAMPLES: usize = 3;

fn is_stalk(parent: &StorybookTreeChain) -> bool {
	matches!(parent.phase, StorybookTreePhase::Stalk(_))
}

/// Deterministic phase key for crook bend axes.
pub(crate) fn segment_key(segment: &BallStickSegment<'_>) -> u32 {
	(segment.start.position.x.to_bits() as u32)
		.wrapping_add(segment.end.position.y.to_bits().rotate_left(3))
		.wrapping_add(segment.end.position.z.to_bits().rotate_left(7))
}

/// Bend strength from stalk / ring height plus stick-surface noise.
pub(crate) fn bend_strength(
	segment: &BallStickSegment<'_>,
	parent_hysteresis: &StorybookTreeChain,
	stick_surface_noise: NoiseParams,
) -> f32 {
	let base = if matches!(parent_hysteresis.phase, StorybookTreePhase::Stalk(_)) {
		STALK_BEND_STRENGTH
	} else {
		let u = parent_hysteresis.ring_u.clamp(0.0, 1.0);
		BRANCH_BEND_STRENGTH_BASE + BRANCH_BEND_STRENGTH_RING_GAIN * u
	};

	let mid = (segment.start.position + segment.end.position) * 0.5;
	let seed = stick_surface_noise.seed
		+ segment.start.position.length() as i32
		+ segment.end.position.length() as i32;
	let noise = NoiseConfig::new(stick_surface_noise.with_seed(seed));
	let n = noise.sample_3d(mid).clamp(-1.0, 1.0);
	(base * (1.0 + BEND_STRENGTH_NOISE_GAIN * n)).max(MIN_BEND_STRENGTH)
}

fn align_positive_y_to(dir: Vec3) -> Quat {
	let d = dir.normalize_or_zero();
	if d.length_squared() < 1e-12 {
		return Quat::IDENTITY;
	}
	Quat::from_rotation_arc(Vec3::Y, d)
}

/// Point on the segment that contributes the silhouette (farther from the Y axis).
fn outermost_horizontal_point(start: Vec3, end: Vec3) -> Vec3 {
	if horizontal_radius_from_y_axis(end) >= horizontal_radius_from_y_axis(start) {
		end
	} else {
		start
	}
}

/// Emit a crook-centerline polyline of StickNodes for one ball-stick segment.
pub(crate) fn crook_polyline_stick_nodes(
	segment: &BallStickSegment<'_>,
	parent: &StorybookTreeChain,
	stick_surface_noise: NoiseParams,
) -> Vec<StickNode> {
	let start = segment.start.position;
	let end = segment.end.position;
	let ray = end - start;
	let len_sq = ray.length_squared();
	if len_sq < 1e-12 {
		return Vec::new();
	}
	let length = len_sq.sqrt();
	let dir = ray / length;
	let rotation = align_positive_y_to(dir);
	let midpoint = start + ray * 0.5;
	let radius = segment.start.radius;
	let radius_end = segment.end.radius;

	let strength = bend_strength(segment, parent, stick_surface_noise);
	let cyl = crook_cylinder(strength, segment_key(segment));
	let xz_scale = UNIT_TARGET_BASE_RADIUS / cyl.base_radius.max(1e-6);
	let s = Vec3::new(radius * xz_scale, length, radius * xz_scale);

	let geometry = if is_stalk(parent) { StickGeometry::Trunk } else { StickGeometry::Segment };

	let mut points = Vec::with_capacity(CROOK_POLYLINE_SAMPLES);
	let mut radii = Vec::with_capacity(CROOK_POLYLINE_SAMPLES);
	for i in 0..CROOK_POLYLINE_SAMPLES {
		let t = i as f32 / (CROOK_POLYLINE_SAMPLES - 1) as f32;
		let local = cyl.centerline(t);
		let p = midpoint + rotation * (local * s - s * 0.5);
		points.push(p);
		radii.push(radius + (radius_end - radius) * t);
	}

	let mut nodes = Vec::new();
	for i in 0..points.len() - 1 {
		if let Some(node) =
			StickNode::from_segment_geometry(points[i], points[i + 1], radii[i], geometry)
		{
			nodes.push(node);
		}
	}
	nodes
}

#[derive(Clone)]
struct BranchCandidate {
	sample_at: Vec3,
	start: Vec3,
	end: Vec3,
	start_radius: f32,
	end_radius: f32,
	parent: StorybookTreeChain,
}

/// Crook trunk hops plus unbanded branch candidates. High and Medium share the trunk so the
/// axis does not hop from the crook path to the ball-stick chord.
fn crook_trunk_and_branch_candidates(
	chain: &BallStickChain<StorybookTreeChain>,
	stick_surface_noise: NoiseParams,
) -> (Vec<StickNode>, Vec<BranchCandidate>) {
	let mut trunk = Vec::new();
	let mut branch_candidates = Vec::new();
	for (segment, parent, _) in chain.segments_with_hysteresis() {
		if is_stalk(parent) {
			trunk.extend(crook_polyline_stick_nodes(&segment, parent, stick_surface_noise));
			continue;
		}
		let start = segment.start.position;
		let end = segment.end.position;
		branch_candidates.push(BranchCandidate {
			sample_at: outermost_horizontal_point(start, end),
			start,
			end,
			start_radius: segment.start.radius,
			end_radius: segment.end.radius,
			parent: parent.clone(),
		});
	}
	(trunk, branch_candidates)
}

fn banded_branches(
	candidates: &[BranchCandidate],
	bands: AzimuthHeightBands,
) -> Vec<&BranchCandidate> {
	sample_max_horizontal_radius_by_azimuth_height(candidates, |c| c.sample_at, bands)
		.into_iter()
		.map(|s| s.item)
		.collect()
}

/// High: crook trunk + crook'd outermost branches per azimuth × height cell.
pub(crate) fn stick_nodes_high_crook(
	chain: &BallStickChain<StorybookTreeChain>,
	stick_surface_noise: NoiseParams,
	bands: AzimuthHeightBands,
) -> Vec<StickNode> {
	let (mut nodes, candidates) = crook_trunk_and_branch_candidates(chain, stick_surface_noise);
	for owned in banded_branches(&candidates, bands) {
		let start_node = sbs_geometry::BallStickNode::new(owned.start, owned.start_radius);
		let end_node = sbs_geometry::BallStickNode::new(owned.end, owned.end_radius);
		let segment = BallStickSegment { start: &start_node, end: &end_node };
		nodes.extend(crook_polyline_stick_nodes(&segment, &owned.parent, stick_surface_noise));
	}
	nodes
}

/// Medium: the same crook trunk as High, plus straight outermost branches (thinner bands).
pub(crate) fn stick_nodes_medium_crook_trunk(
	chain: &BallStickChain<StorybookTreeChain>,
	stick_surface_noise: NoiseParams,
	bands: AzimuthHeightBands,
) -> Vec<StickNode> {
	let (mut nodes, candidates) = crook_trunk_and_branch_candidates(chain, stick_surface_noise);
	for owned in banded_branches(&candidates, bands) {
		if let Some(node) = StickNode::from_segment(owned.start, owned.end, owned.start_radius) {
			nodes.push(node);
		}
	}
	nodes
}

fn crook_cylinder(bend_strength: f32, segment_key: u32) -> CrookCylinder {
	let strength = bend_strength.max(MIN_CROOK_BEND_STRENGTH);
	let base_r = 1.0 / strength;
	let top_r = base_r * (UNIT_TARGET_TOP_RADIUS / UNIT_TARGET_BASE_RADIUS);
	let phase_x = if segment_key & 1 == 0 { 0.0 } else { PI };
	let phase_z = if segment_key.rotate_left(PHASE_Z_KEY_ROTATE) & 1 == 0 { 0.0 } else { PI };
	let mut cyl = CrookCylinder {
		base_radius: base_r,
		top_radius: top_r,
		y_min: 0.0,
		height: 1.0,
		bounds_margin: 0.0,
		bend_x: SDF_BEND_X,
		bend_z: SDF_BEND_Z,
		phase_x,
		phase_z,
	};
	cyl.bounds_margin = cyl.chunk_mu_xz_pad().max(MIN_BOUNDS_MARGIN);
	cyl
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use sbs_geometry::BallStickNode;

	fn branch_chain(ring_u: f32) -> StorybookTreeChain {
		StorybookTreeChain::new(
			procedural_common::NoiseConfig::new(NoiseParams::default()),
			6.0,
			3,
			0.0,
			ring_u,
			0.65,
			StorybookTreePhase::BranchOut(sbs_geometry::DepthBudget {
				inner: sbs_geometry::BranchOut::radial_out_horizontal(
					BallStickNode::new(Vec3::ZERO, 0.04),
					Vec3::X,
				),
				remaining: 3,
			}),
		)
	}

	#[test]
	fn branch_bend_strength_grows_with_ring_height() {
		let noise = NoiseParams::from_scalar(42.0, 1.0, 0.05, 1);
		let segment = BallStickSegment {
			start: &BallStickNode::new(Vec3::ZERO, 0.4),
			end: &BallStickNode::new(Vec3::new(0.0, 2.0, 0.0), 0.35),
		};
		let s_lo = bend_strength(&segment, &branch_chain(0.1), noise);
		let s_hi = bend_strength(&segment, &branch_chain(0.9), noise);
		assert!(s_hi > s_lo);
	}
}
