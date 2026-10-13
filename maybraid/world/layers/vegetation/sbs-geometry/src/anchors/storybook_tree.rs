//! **Storybook Tree** stalk anchor rings ([#230](https://github.com/ramate-io/maybraid/issues/230), [RFC §3.1.7.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/01-storybook-tree/README.md)).

use std::f32::consts::TAU;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::stalk_perturbation::{
	perturb_branch_out, perturb_node, AnchorPerturbation, HasStrictStalk, PerturbAnchor,
	StalkPerturbation,
};
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

/// Default total tree height `H` for playground Storybook trees.
pub const DEFAULT_TREE_HEIGHT: f32 = 18.0;

/// RFC stalk height as a fraction of total tree height.
pub const DEFAULT_STALK_HEIGHT_FRACTION: f32 = 0.80;

/// RFC stalk base radius as a fraction of `H`.
pub const DEFAULT_STALK_BASE_RADIUS_FRACTION: f32 = 0.035;

/// Max projection at the crown belt as a fraction of `H`.
pub const DEFAULT_MAX_PROJECTION_FRACTION: f32 = 0.50;

/// End-ring minimum projection as a fraction of `H` (storybook dome floor; mid-canopy uses [`DEFAULT_MAX_PROJECTION_FRACTION`]).
pub const DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT: f32 = 0.20;

/// RFC outer foliage distance threshold as a fraction of limb projection.
pub const DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION: f32 = 0.65;

/// Lowest ring along the stalk as a unit height fraction (stalk base = 0, tip = 1).
pub const DEFAULT_FIRST_RING_UNIT_HEIGHT: f32 = 0.30;

/// Dome/bell profile: low at the trunk base and tip, longest near mid-canopy.
///
/// \(\ell(u) = \ell_{\min} + (\ell_{\max} - \ell_{\min}) \sin(\pi u)\).
pub fn dome_projection_length(ell_max: f32, ell_min: f32, u: f32) -> f32 {
	let u = u.clamp(0.0, 1.0);
	let ell_max = ell_max.max(ell_min);
	let ell_min = ell_min.min(ell_max);
	let bell = (std::f32::consts::PI * u).sin();
	ell_min + (ell_max - ell_min) * bell
}

/// Storybook dome projection at ring mix `u`, from tree height and min/max fractions of `H`.
pub fn storybook_dome_projection_length(
	tree_height: f32,
	max_fraction_of_height: f32,
	min_fraction_of_height: f32,
	u: f32,
) -> f32 {
	let h = tree_height.max(1e-6);
	let ell_max = h * max_fraction_of_height;
	let ell_min = h * min_fraction_of_height.min(max_fraction_of_height);
	dome_projection_length(ell_max, ell_min, u)
}

/// Tilt horizontal radial: lower rings slightly downward, upper rings slightly upward.
fn ring_biased_radial(radial_xz: Vec3, ring_u: f32, max_tilt_degrees: f32) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::Y;
	}
	let tilt = max_tilt_degrees.to_radians() * (ring_u * 2.0 - 1.0);
	let cos = tilt.cos();
	let sin = tilt.sin();
	Vec3::new(radial.x * cos, sin, radial.z * cos).normalize_or_zero()
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorybookTreeProtoAnchors {
	/// Total tree height `H` including canopy (RFC scale).
	pub tree_height: f32,
	pub stalk: StrictStalk,
	pub first_ring_unit_height: f32,
	pub last_ring_unit_height: f32,
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	pub max_projection_fraction_of_height: f32,
	/// End-ring minimum projection as a fraction of [`Self::tree_height`] (dome floor at `u ∈ {0, 1}`).
	pub projection_min_fraction_of_height: f32,
	pub ring_tilt_degrees: f32,
	pub branch_angle_tolerance: f32,
	pub bias_blend: f32,
	/// Limb hop count; coerced to `3..=5` via [`storybook_branch_depth`](crate::chain::storybook_tree::storybook_branch_depth) when building seeds (must match [`segment_fracs`](crate::segment_fracs)).
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub outer_foliage_distance_fraction: f32,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for StorybookTreeProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_TREE_HEIGHT;
		let stalk_h = h * DEFAULT_STALK_HEIGHT_FRACTION;
		Self {
			tree_height: h,
			stalk: StrictStalk {
				stalk_height: stalk_h,
				stalk_base_radius: DEFAULT_STALK_BASE_RADIUS_FRACTION * h,
			},
			first_ring_unit_height: DEFAULT_FIRST_RING_UNIT_HEIGHT,
			last_ring_unit_height: 1.0,
			ring_spacing_unit_height: 0.08 / DEFAULT_STALK_HEIGHT_FRACTION,
			anchors_per_ring: 6,
			max_projection_fraction_of_height: DEFAULT_MAX_PROJECTION_FRACTION,
			projection_min_fraction_of_height: DEFAULT_PROJECTION_MIN_FRACTION_OF_HEIGHT,
			ring_tilt_degrees: 4.0,
			branch_angle_tolerance: 26.0_f32.to_radians(),
			bias_blend: 0.88,
			branch_depth: 4,
			child_count_min: 1,
			child_count_max: 3,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			branch_base_radius_fraction_of_stalk: 0.12,
			branch_radius_child_scale: (0.75, 0.82),
		}
	}
}

impl StorybookTreeProtoAnchors {
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
		storybook_dome_projection_length(
			self.tree_height,
			self.max_projection_fraction_of_height,
			self.projection_min_fraction_of_height,
			u,
		)
	}

	/// Limb radius at ring anchors; floored so degenerate SBS fractions still produce visible sticks.
	fn limb_base_radius(&self) -> f32 {
		let base = self.stalk.stalk_base_radius.max(1e-4);
		branch_base_radius_for_stalk(
			base,
			self.branch_base_radius_fraction_of_stalk,
			0.02,
			self.stalk.stalk_height,
			DEFAULT_TREE_HEIGHT * DEFAULT_STALK_HEIGHT_FRACTION,
		)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let mut out = Vec::new();
		let k = self.anchors_per_ring.max(1);
		let radial_eps = (self.stalk.stalk_base_radius * 0.05).max(1e-4);
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

		for z_frac in self.ring_height_fractions() {
			let u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let bias = ring_biased_radial(radial, u, self.ring_tilt_degrees);
				let pos = self.stalk.centroid_at_height_fraction(z_frac) + radial * radial_eps;

				let seed_node = BallStickNode::new(pos, limb_r);
				let first_len = proj * fracs[0];
				let noise = chain_noise.clone();
				let branch = BranchOut::radial_out_horizontal(seed_node, radial)
					.with_hysteresis_context(noise.clone(), 0, bias)
					.with_bias_blend(self.bias_blend)
					.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
					.with_child_count(
						self.child_count_min as usize
							..(self.child_count_max as usize).saturating_add(1),
					)
					.with_radius_range(limb_r..limb_r)
					.with_radius_range_child_scale(child_scale)
					.with_radius_range_child_bounds(child_bounds.clone())
					.with_length(first_len * 0.97..first_len * 1.03);

				out.push(StorybookTreeChain::new(
					noise.clone().with_frequency(noise.params().frequency * 10.0),
					proj,
					depth,
					0.0,
					u,
					self.outer_foliage_distance_fraction,
					StorybookTreePhase::BranchOut(DepthBudget { inner: branch, remaining: depth }),
				));
			}
		}

		for a in self.stalk.point_to_point_anchors() {
			out.push(StorybookTreeChain::new(
				chain_noise.clone(),
				0.0,
				depth,
				0.0,
				0.0,
				self.outer_foliage_distance_fraction,
				StorybookTreePhase::Stalk(a),
			));
		}

		out
	}
}

impl HasStrictStalk for StorybookTreeProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

/// Perturbing wrapper used by [`crate::sbs::storybook_tree::StorybookTreeSbs`].
#[derive(Clone, Debug, PartialEq)]
pub struct StorybookTreeAnchors {
	pub perturbation: StalkPerturbation<StorybookTreeProtoAnchors>,
}

impl StorybookTreeAnchors {
	pub fn new(proto: StorybookTreeProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: StorybookTreeAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &StorybookTreeProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct StorybookTreeAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for StorybookTreeAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for StorybookTreeAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for StorybookTreeAnchors {
	fn default() -> Self {
		Self::new(StorybookTreeProtoAnchors::default())
	}
}

impl Anchors<StorybookTreeChain> for StorybookTreeAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl Anchors<StorybookTreeChain> for StorybookTreeProtoAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams::default()))
	}
}

impl PerturbAnchor for StorybookTreeChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = match self.phase {
			StorybookTreePhase::Stalk(mut p) => {
				p.start = perturb_node(p.start, perturbation);
				StorybookTreePhase::Stalk(p)
			}
			StorybookTreePhase::BranchOut(mut b) => {
				b.inner = perturb_branch_out(b.inner, perturbation);
				StorybookTreePhase::BranchOut(b)
			}
		};
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::chain::storybook_tree::STORYBOOK_BRANCH_DEPTH_MAX;

	#[test]
	fn dome_projection_low_at_ends_high_at_mid() {
		let ell_max = 10.0;
		let ell_min = 4.0;
		let l0 = dome_projection_length(ell_max, ell_min, 0.0);
		let l1 = dome_projection_length(ell_max, ell_min, 1.0);
		let mid = dome_projection_length(ell_max, ell_min, 0.5);
		assert!((l0 - ell_min).abs() < 1e-4);
		assert!((l1 - ell_min).abs() < 1e-4);
		assert!(mid > l0 * 1.5);
		assert!(mid > l1 * 1.5);
	}

	#[test]
	fn projection_length_peaks_near_mid_canopy() {
		let a = StorybookTreeProtoAnchors::default();
		let l_low = a.projection_length(a.ring_mix_u(a.first_ring_unit_height));
		let l_high = a.projection_length(a.ring_mix_u(a.last_ring_unit_height));
		let l_mid = a.projection_length(0.5);
		assert!(l_mid > l_low);
		assert!(l_mid > l_high);
	}

	#[test]
	fn ring_count_in_expected_band() {
		let a = StorybookTreeProtoAnchors::default();
		let n = a.ring_height_fractions().len();
		assert!((7..=10).contains(&n), "ring count {n}");
	}

	#[test]
	fn branch_depth_coerced_at_hysteresis_seed() {
		let mut proto = StorybookTreeProtoAnchors::default();
		proto.branch_depth = 99;
		let seeds = proto.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()));
		let branch = seeds
			.iter()
			.find(|s| matches!(s.phase, StorybookTreePhase::BranchOut(_)))
			.expect("branch seed");
		assert_eq!(branch.branch_depth, STORYBOOK_BRANCH_DEPTH_MAX);
		if let StorybookTreePhase::BranchOut(b) = &branch.phase {
			assert_eq!(b.remaining, STORYBOOK_BRANCH_DEPTH_MAX);
		}
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let proto =
			StorybookTreeProtoAnchors { ring_spacing_unit_height: 0.20, ..Default::default() };
		let ring_count = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		let a = StorybookTreeAnchors::new(proto);
		assert_eq!(a.anchors().len(), ring_count * spokes + 1);
	}
}
