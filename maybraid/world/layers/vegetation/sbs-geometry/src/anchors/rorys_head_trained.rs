//! **Rory's Head-trained** anchor recipe ([#254](https://github.com/ramate-io/maybraid/issues/254), [RFC §3.1.7.7](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/07-rory-s-head-trained/README.md)).
//!
//! Single canopy ring at the stalk tip; flat projection length along the trained horizontal plane.

use std::f32::consts::TAU;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::stalk_perturbation::{HasStrictStalk, StalkPerturbation};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::storybook_tree::{
	segment_fracs, storybook_branch_depth, StorybookTreeChain, StorybookTreePhase,
};
use crate::chain::{BranchOut, DepthBudget};
use crate::sbs::scale::{
	branch_base_radius_for_stalk, branch_radius_child_bounds_for_stalk,
	branch_radius_child_scale_for_stalk,
};
use crate::BallStickNode;
use procedural_common::NoiseParams;

// --- Scale (RFC § stalk) ---

pub const DEFAULT_TREE_HEIGHT: f32 = 18.0;
pub const DEFAULT_STALK_HEIGHT_FRACTION: f32 = 0.90;
/// Default stalk radius as a fraction of tree height. RFC lists `0.025 * H`;
/// the playground default is thicker so the column reads at orchard / savanna scale.
pub const DEFAULT_STALK_BASE_RADIUS_FRACTION: f32 = 0.105;
pub const DEFAULT_STALK_SECTION_COUNT: u32 = 3;

// --- Canopy ring (RFC § anchor ring) ---

pub const DEFAULT_CANOPY_RING_UNIT_HEIGHT: f32 = 1.0;
/// RFC `6..=8`.
pub const DEFAULT_ANCHORS_PER_RING: u32 = 4;

// --- Flat projection (RFC § projection length) ---

pub const DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT: f32 = 0.22;
pub const DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT: f32 = 0.38;
/// Bush / grape-vine variant: RFC `0.60 * H`.
pub const BUSH_PROJECTION_FRACTION_OF_HEIGHT: f32 = 0.60;
pub const BUSH_STALK_HEIGHT_FRACTION: f32 = 0.60;

// --- Branch growth (RFC § chain growth) ---

/// RFC `segments: 3..=5`.
pub const DEFAULT_BRANCH_DEPTH: usize = 4;
pub const DEFAULT_CHILD_COUNT_MIN: u32 = 1;
pub const DEFAULT_CHILD_COUNT_MAX: u32 = 3;
/// RFC `radians(10.0)`; playground uses a wider fan.
pub const DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES: f32 = 50.0;
pub const DEFAULT_BIAS_BLEND: f32 = 1.0;
/// RFC `normalize(radial + Vec3::Y * 0.02)` ≈ small upward lift.
pub const DEFAULT_BIAS_ELEVATION_DEGREES: f32 = 20.0;

pub const DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.52;
pub const DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO: f32 = 0.90;
pub const DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI: f32 = 0.94;

// --- Foliage (RFC § ball selection) ---

pub const DEFAULT_LEAF_RADIUS_FRACTION: f32 = 0.126;
pub const DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION: f32 = 0.65;

// --- Anchor / limb numerics ---

const RADIAL_OFFSET_FRACTION_OF_STALK_BASE: f32 = 0.05;
const LIMB_BASE_RADIUS_FLOOR: f32 = 0.02;
const STALK_RADIUS_EPSILON: f32 = 1e-4;
const RADIAL_DIRECTION_EPSILON: f32 = 1e-12;
const BRANCH_HYSTERESIS_FREQUENCY_SCALE: f32 = 10.0;
const SEGMENT_LENGTH_JITTER_LO: f32 = 0.97;
const SEGMENT_LENGTH_JITTER_HI: f32 = 1.03;

const ANCHOR_VERTICAL_OFFSET_LO: f32 = -1.0;
const ANCHOR_VERTICAL_OFFSET_HI: f32 = 1.0;
const ANCHOR_ANGULAR_SCALE_LO: f32 = 0.0;
const ANCHOR_ANGULAR_SCALE_HI: f32 = 0.5;
const ANCHOR_RADIUS_OFFSET_LO: f32 = -0.05;
const ANCHOR_RADIUS_OFFSET_HI: f32 = 0.05;

/// Flat projection length at ring mix `u` in `[0, 1]`.
pub fn rorys_flat_projection_length(
	tree_height: f32,
	min_fraction: f32,
	max_fraction: f32,
	u: f32,
) -> f32 {
	let h = tree_height.max(1e-6);
	let u = u.clamp(0.0, 1.0);
	let lo = h * min_fraction.min(max_fraction);
	let hi = h * min_fraction.max(max_fraction);
	lo + (hi - lo) * u
}

/// Bias ray with slight upward tilt from horizontal.
pub fn rorys_head_trained_branch_direction(radial_xz: Vec3, elevation_degrees: f32) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < RADIAL_DIRECTION_EPSILON {
		return Vec3::Y;
	}
	let y = elevation_degrees.to_radians().tan();
	(radial + Vec3::Y * y).normalize_or_zero()
}

#[derive(Clone, Debug, PartialEq)]
pub struct RorysHeadTrainedProtoAnchors {
	pub tree_height: f32,
	pub stalk: StrictStalk,
	pub canopy_ring_unit_height: f32,
	pub anchors_per_ring: u32,
	pub projection_min_fraction_of_height: f32,
	pub projection_max_fraction_of_height: f32,
	pub bias_elevation_degrees: f32,
	pub branch_angle_tolerance: f32,
	pub bias_blend: f32,
	pub stalk_section_count: u32,
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub outer_foliage_distance_fraction: f32,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for RorysHeadTrainedProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_TREE_HEIGHT;
		Self {
			tree_height: h,
			stalk: StrictStalk {
				stalk_height: h * DEFAULT_STALK_HEIGHT_FRACTION,
				stalk_base_radius: DEFAULT_STALK_BASE_RADIUS_FRACTION * h,
			},
			canopy_ring_unit_height: DEFAULT_CANOPY_RING_UNIT_HEIGHT,
			anchors_per_ring: DEFAULT_ANCHORS_PER_RING,
			projection_min_fraction_of_height: DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
			projection_max_fraction_of_height: DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			bias_elevation_degrees: DEFAULT_BIAS_ELEVATION_DEGREES,
			branch_angle_tolerance: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: DEFAULT_BIAS_BLEND,
			stalk_section_count: DEFAULT_STALK_SECTION_COUNT,
			branch_depth: DEFAULT_BRANCH_DEPTH,
			child_count_min: DEFAULT_CHILD_COUNT_MIN,
			child_count_max: DEFAULT_CHILD_COUNT_MAX,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			branch_base_radius_fraction_of_stalk: DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale: (
				DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO,
				DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI,
			),
		}
	}
}

impl RorysHeadTrainedProtoAnchors {
	pub fn projection_length_at_ring(&self) -> f32 {
		rorys_flat_projection_length(
			self.tree_height,
			self.projection_min_fraction_of_height,
			self.projection_max_fraction_of_height,
			1.0,
		)
	}

	fn limb_base_radius(&self) -> f32 {
		let base = self.stalk.stalk_base_radius.max(STALK_RADIUS_EPSILON);
		branch_base_radius_for_stalk(
			base,
			self.branch_base_radius_fraction_of_stalk,
			LIMB_BASE_RADIUS_FLOOR,
			self.stalk.stalk_height,
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
		)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let spokes = self.anchors_per_ring.max(1) as usize;
		let radial_eps = (self.stalk.stalk_base_radius * RADIAL_OFFSET_FRACTION_OF_STALK_BASE)
			.max(STALK_RADIUS_EPSILON);
		let limb_r = self.limb_base_radius();
		let depth = storybook_branch_depth(self.branch_depth);
		let fracs = segment_fracs(depth);
		let proj = self.projection_length_at_ring();
		let child_scale = branch_radius_child_scale_for_stalk(
			self.branch_radius_child_scale,
			self.stalk.stalk_height,
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
		);
		let child_bounds = branch_radius_child_bounds_for_stalk(
			self.stalk.stalk_base_radius,
			limb_r,
			self.stalk.stalk_height,
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
		);
		let ring_z = self.canopy_ring_unit_height;
		let child_count =
			self.child_count_min as usize..(self.child_count_max as usize).saturating_add(1);

		let mut out = Vec::with_capacity(spokes + 1);

		for i in 0..spokes {
			let theta = TAU * (i as f32) / (spokes as f32);
			let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
			let dir = rorys_head_trained_branch_direction(radial, self.bias_elevation_degrees);
			let pos = self.stalk.centroid_at_height_fraction(ring_z) + radial * radial_eps;
			let first_len = proj * fracs[0];
			let noise = chain_noise.clone();

			let branch = BranchOut::radial_out_horizontal(BallStickNode::new(pos, limb_r), radial)
				.with_hysteresis_context(noise.clone(), 0, dir)
				.with_bias_ray(dir)
				.with_bias_blend(self.bias_blend)
				.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
				.with_child_count(child_count.clone())
				.with_radius_range(limb_r..limb_r)
				.with_radius_range_child_scale(child_scale)
				.with_radius_range_child_bounds(child_bounds.clone())
				.with_length(
					first_len * SEGMENT_LENGTH_JITTER_LO..first_len * SEGMENT_LENGTH_JITTER_HI,
				);

			out.push(StorybookTreeChain::new(
				noise
					.clone()
					.with_frequency(noise.params().frequency * BRANCH_HYSTERESIS_FREQUENCY_SCALE),
				proj,
				depth,
				0.0,
				1.0,
				self.outer_foliage_distance_fraction,
				StorybookTreePhase::BranchOut(DepthBudget { inner: branch, remaining: depth }),
			));
		}

		for stalk in self.stalk.segmented_point_to_point_anchors(self.stalk_section_count) {
			out.push(StorybookTreeChain::new(
				chain_noise.clone(),
				0.0,
				depth,
				0.0,
				0.0,
				self.outer_foliage_distance_fraction,
				StorybookTreePhase::Stalk(stalk),
			));
		}

		out
	}
}

impl HasStrictStalk for RorysHeadTrainedProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct RorysHeadTrainedAnchors {
	pub perturbation: StalkPerturbation<RorysHeadTrainedProtoAnchors>,
}

impl RorysHeadTrainedAnchors {
	pub fn new(proto: RorysHeadTrainedProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: RorysHeadTrainedAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &RorysHeadTrainedProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		self.perturbation.perturb_anchors(self.proto().hysteresis_seeds(chain_noise))
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct RorysHeadTrainedAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for RorysHeadTrainedAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: ANCHOR_VERTICAL_OFFSET_LO..ANCHOR_VERTICAL_OFFSET_HI,
			angular_scale: ANCHOR_ANGULAR_SCALE_LO..ANCHOR_ANGULAR_SCALE_HI,
			radius_offset: ANCHOR_RADIUS_OFFSET_LO..ANCHOR_RADIUS_OFFSET_HI,
		}
	}
}

impl Default for RorysHeadTrainedAnchors {
	fn default() -> Self {
		Self::new(RorysHeadTrainedProtoAnchors::default())
	}
}

impl HasStrictStalk for RorysHeadTrainedAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Anchors<StorybookTreeChain> for RorysHeadTrainedAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl Anchors<StorybookTreeChain> for RorysHeadTrainedProtoAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn single_canopy_ring_at_stalk_tip() {
		let proto = RorysHeadTrainedProtoAnchors::default();
		assert_eq!(proto.canopy_ring_unit_height, DEFAULT_CANOPY_RING_UNIT_HEIGHT);
		assert_eq!(proto.anchors_per_ring, DEFAULT_ANCHORS_PER_RING);
	}

	#[test]
	fn branch_direction_has_configured_upward_bias() {
		let radial = Vec3::new(1.0, 0.0, 0.0);
		let dir = rorys_head_trained_branch_direction(radial, DEFAULT_BIAS_ELEVATION_DEGREES);
		let elev = dir.y.atan2(Vec3::new(dir.x, 0.0, dir.z).length()).to_degrees();
		assert!((elev - DEFAULT_BIAS_ELEVATION_DEGREES).abs() < 2.0);
	}

	#[test]
	fn flat_projection_span_orders_endpoints() {
		let h = DEFAULT_TREE_HEIGHT;
		let lo = rorys_flat_projection_length(
			h,
			DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
			DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			0.0,
		);
		let hi = rorys_flat_projection_length(
			h,
			DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
			DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			1.0,
		);
		assert!(hi > lo);
	}

	#[test]
	fn branch_projection_applies_length_scale() {
		let proto = RorysHeadTrainedProtoAnchors::default();
		let base = rorys_flat_projection_length(
			proto.tree_height,
			proto.projection_min_fraction_of_height,
			proto.projection_max_fraction_of_height,
			1.0,
		);
		assert!((proto.projection_length_at_ring() - base).abs() < 1e-4);
	}

	#[test]
	fn anchor_count_is_spokes_plus_stalk() {
		let proto = RorysHeadTrainedProtoAnchors::default();
		assert_eq!(
			RorysHeadTrainedAnchors::new(proto.clone()).anchors().len(),
			proto.anchors_per_ring as usize + 1
		);
	}
}
