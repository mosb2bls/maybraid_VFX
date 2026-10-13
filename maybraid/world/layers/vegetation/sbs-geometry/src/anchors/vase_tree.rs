//! **Vase Tree** stalk anchor rings ([#246](https://github.com/ramate-io/maybraid/issues/246), [RFC §3.1.7.3](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/03-vase-tree/README.md)).

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
use crate::projection::vase_projection_length;
use crate::sbs::scale::{
	branch_base_radius_for_stalk, branch_radius_child_bounds_for_stalk,
	branch_radius_child_scale_for_stalk,
};
use crate::BallStickNode;
use procedural_common::NoiseParams;

pub const DEFAULT_TREE_HEIGHT: f32 = 18.0;
pub const DEFAULT_STALK_HEIGHT_FRACTION: f32 = 0.75;
pub const DEFAULT_STALK_BASE_RADIUS_FRACTION: f32 = 0.035;

/// First ring at `z_min / stalk_height` (RFC `z_min = 0.20 * H`, stalk `0.75 * H`).
pub const DEFAULT_FIRST_RING_UNIT_HEIGHT: f32 = 0.20 / DEFAULT_STALK_HEIGHT_FRACTION;
pub const DEFAULT_LAST_RING_UNIT_HEIGHT: f32 = 1.0;
/// RFC ring spacing `0.08 * H` as a stalk unit fraction.
pub const DEFAULT_RING_SPACING_UNIT_HEIGHT: f32 = 0.08 / DEFAULT_STALK_HEIGHT_FRACTION;

pub const DEFAULT_ANCHORS_PER_RING: u32 = 6;
pub const DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT: f32 = 0.15;
pub const DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT: f32 = 0.50;
pub const DEFAULT_VASE_PROFILE_EPSILON: f32 = 0.08;
pub const DEFAULT_VASE_PROFILE_CENTER: f32 = 0.5;

pub const DEFAULT_BIAS_ELEVATION_LO_DEGREES: f32 = 45.0;
pub const DEFAULT_BIAS_ELEVATION_HI_DEGREES: f32 = 5.0;
pub const DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES: f32 = 15.0;
pub const DEFAULT_BIAS_BLEND: f32 = 1.0;
pub const DEFAULT_BRANCH_DEPTH: usize = 4;
pub const DEFAULT_CHILD_COUNT_MIN: u32 = 1;
pub const DEFAULT_CHILD_COUNT_MAX: u32 = 3;

pub const DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.12;
pub const DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO: f32 = 0.75;
pub const DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI: f32 = 0.82;

pub const DEFAULT_LEAF_RADIUS_FRACTION: f32 = 0.08;
/// Medium crown ball at the stalk tip (`~1.5×` [`DEFAULT_LEAF_RADIUS_FRACTION`]).
pub const DEFAULT_APEX_BALL_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.12;
pub const DEFAULT_UPPER_FOLIAGE_RING_U: f32 = 0.60;
pub const DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION: f32 = 0.60;

pub const BUSH_STALK_HEIGHT_FRACTION: f32 = 0.60;

const RADIAL_OFFSET_FRACTION_OF_STALK_BASE: f32 = 0.05;
const LIMB_BASE_RADIUS_FLOOR: f32 = 0.02;
const STALK_RADIUS_EPSILON: f32 = 1e-4;
const RADIAL_DIRECTION_EPSILON: f32 = 1e-12;
const BRANCH_HYSTERESIS_FREQUENCY_SCALE: f32 = 10.0;
const SEGMENT_LENGTH_JITTER_LO: f32 = 0.97;
const SEGMENT_LENGTH_JITTER_HI: f32 = 1.03;

/// Bias ray: strongly upward at the crown base, nearly horizontal at the rim (RFC `mix(45°, 5°, u)`).
pub fn vase_tree_branch_direction(
	radial_xz: Vec3,
	ring_u: f32,
	elevation_lo_degrees: f32,
	elevation_hi_degrees: f32,
) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < RADIAL_DIRECTION_EPSILON {
		return Vec3::Y;
	}
	let u = ring_u.clamp(0.0, 1.0);
	let elev = elevation_lo_degrees + (elevation_hi_degrees - elevation_lo_degrees) * u;
	let y = elev.to_radians().tan();
	(radial + Vec3::Y * y).normalize_or_zero()
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaseTreeProtoAnchors {
	pub tree_height: f32,
	pub stalk: StrictStalk,
	pub first_ring_unit_height: f32,
	pub last_ring_unit_height: f32,
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	pub projection_min_fraction_of_height: f32,
	pub projection_max_fraction_of_height: f32,
	pub vase_profile_epsilon: f32,
	pub vase_profile_center: f32,
	pub bias_elevation_lo_degrees: f32,
	pub bias_elevation_hi_degrees: f32,
	pub branch_angle_tolerance: f32,
	pub bias_blend: f32,
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub upper_foliage_ring_u: f32,
	pub outer_foliage_distance_fraction: f32,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for VaseTreeProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_TREE_HEIGHT;
		Self {
			tree_height: h,
			stalk: StrictStalk {
				stalk_height: h * DEFAULT_STALK_HEIGHT_FRACTION,
				stalk_base_radius: DEFAULT_STALK_BASE_RADIUS_FRACTION * h,
			},
			first_ring_unit_height: DEFAULT_FIRST_RING_UNIT_HEIGHT,
			last_ring_unit_height: DEFAULT_LAST_RING_UNIT_HEIGHT,
			ring_spacing_unit_height: DEFAULT_RING_SPACING_UNIT_HEIGHT,
			anchors_per_ring: DEFAULT_ANCHORS_PER_RING,
			projection_min_fraction_of_height: DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
			projection_max_fraction_of_height: DEFAULT_PROJECTION_MAX_FRACTION_OF_HEIGHT,
			vase_profile_epsilon: DEFAULT_VASE_PROFILE_EPSILON,
			vase_profile_center: DEFAULT_VASE_PROFILE_CENTER,
			bias_elevation_lo_degrees: DEFAULT_BIAS_ELEVATION_LO_DEGREES,
			bias_elevation_hi_degrees: DEFAULT_BIAS_ELEVATION_HI_DEGREES,
			branch_angle_tolerance: DEFAULT_BRANCH_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: DEFAULT_BIAS_BLEND,
			branch_depth: DEFAULT_BRANCH_DEPTH,
			child_count_min: DEFAULT_CHILD_COUNT_MIN,
			child_count_max: DEFAULT_CHILD_COUNT_MAX,
			upper_foliage_ring_u: DEFAULT_UPPER_FOLIAGE_RING_U,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			branch_base_radius_fraction_of_stalk: DEFAULT_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale: (
				DEFAULT_BRANCH_RADIUS_CHILD_SCALE_LO,
				DEFAULT_BRANCH_RADIUS_CHILD_SCALE_HI,
			),
		}
	}
}

impl VaseTreeProtoAnchors {
	pub fn ring_height_fractions(&self) -> Vec<f32> {
		let mut out = Vec::new();
		let mut z = self.first_ring_unit_height;
		while z <= self.last_ring_unit_height + 1e-6 {
			out.push(z);
			z += self.ring_spacing_unit_height;
		}
		out
	}

	pub fn ring_mix_u(&self, z_frac: f32) -> f32 {
		let a = self.first_ring_unit_height;
		let b = self.last_ring_unit_height;
		if (b - a).abs() < 1e-6 {
			return 0.0;
		}
		((z_frac - a) / (b - a)).clamp(0.0, 1.0)
	}

	pub fn projection_length(&self, u: f32) -> f32 {
		vase_projection_length(
			self.tree_height,
			self.projection_min_fraction_of_height,
			self.projection_max_fraction_of_height,
			u,
			self.vase_profile_epsilon,
			self.vase_profile_center,
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
		let child_count =
			self.child_count_min as usize..(self.child_count_max as usize).saturating_add(1);

		let mut out = Vec::new();

		for z_frac in self.ring_height_fractions() {
			let ring_u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(ring_u);

			for i in 0..spokes {
				let theta = TAU * (i as f32) / (spokes as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let bias = vase_tree_branch_direction(
					radial,
					ring_u,
					self.bias_elevation_lo_degrees,
					self.bias_elevation_hi_degrees,
				);
				let pos = self.stalk.centroid_at_height_fraction(z_frac) + radial * radial_eps;
				let first_len = proj * fracs[0];
				let noise = chain_noise.clone();

				let branch =
					BranchOut::radial_out_horizontal(BallStickNode::new(pos, limb_r), radial)
						.with_hysteresis_context(noise.clone(), 0, bias)
						.with_bias_ray(bias)
						.with_bias_blend(self.bias_blend)
						.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
						.with_child_count(child_count.clone())
						.with_radius_range(limb_r..limb_r)
						.with_radius_range_child_scale(child_scale)
						.with_radius_range_child_bounds(child_bounds.clone())
						.with_length(
							first_len * SEGMENT_LENGTH_JITTER_LO
								..first_len * SEGMENT_LENGTH_JITTER_HI,
						);

				out.push(StorybookTreeChain::new(
					noise.clone().with_frequency(
						noise.params().frequency * BRANCH_HYSTERESIS_FREQUENCY_SCALE,
					),
					proj,
					depth,
					0.0,
					ring_u,
					self.outer_foliage_distance_fraction,
					StorybookTreePhase::BranchOut(DepthBudget { inner: branch, remaining: depth }),
				));
			}
		}

		for stalk in self.stalk.point_to_point_anchors() {
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

impl HasStrictStalk for VaseTreeProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaseTreeAnchors {
	pub perturbation: StalkPerturbation<VaseTreeProtoAnchors>,
}

impl VaseTreeAnchors {
	pub fn new(proto: VaseTreeProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: VaseTreeAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &VaseTreeProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		self.perturbation.perturb_anchors(self.proto().hysteresis_seeds(chain_noise))
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct VaseTreeAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for VaseTreeAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl Default for VaseTreeAnchors {
	fn default() -> Self {
		Self::new(VaseTreeProtoAnchors::default())
	}
}

impl HasStrictStalk for VaseTreeAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Anchors<StorybookTreeChain> for VaseTreeAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl Anchors<StorybookTreeChain> for VaseTreeProtoAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vase_projection_widens_toward_rim() {
		let proto = VaseTreeProtoAnchors::default();
		let low = proto.projection_length(proto.ring_mix_u(proto.first_ring_unit_height));
		let high = proto.projection_length(proto.ring_mix_u(proto.last_ring_unit_height));
		assert!(high > low, "rim {high} vs base {low}");
	}

	#[test]
	fn branch_bias_relaxes_with_ring_height() {
		let radial = Vec3::new(1.0, 0.0, 0.0);
		let low = vase_tree_branch_direction(radial, 0.0, 45.0, 5.0);
		let high = vase_tree_branch_direction(radial, 1.0, 45.0, 5.0);
		let elev = |d: Vec3| d.y.atan2(Vec3::new(d.x, 0.0, d.z).length()).to_degrees();
		assert!(elev(low) > elev(high));
	}

	#[test]
	fn anchor_count_matches_rings_times_spokes_plus_stalk() {
		let proto = VaseTreeProtoAnchors::default();
		let rings = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		assert_eq!(VaseTreeAnchors::new(proto).anchors().len(), rings * spokes + 1);
	}
}
