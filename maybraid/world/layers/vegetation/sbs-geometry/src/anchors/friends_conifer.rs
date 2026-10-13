//! **Stalk anchor rings** for **Friend's Conifer** ([RFC-183 §3.1.7.14](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/14-friend-s-conifer/README.md)).
//!
//! Same layout as [`super::liams_conifer`], with logarithmic projection taper and RFC ring defaults.
//! Friend's Conifer ([#236](https://github.com/ramate-io/maybraid/issues/236)) uses [`FriendsConiferProtoAnchors::default`];
//! Temperate Conifer ([#238](https://github.com/ramate-io/maybraid/issues/238)) applies a shorter-limb preset via [`crate::sbs::friends_conifer::FriendsConiferSbs::apply_temperate_preset`].

use std::f32::consts::TAU;

use bevy_math::Vec3;

use super::liams_conifer::downward_biased_radial;
use super::stalk_perturbation::{HasStrictStalk, StalkPerturbation};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::projection::logarithmic_rounding_projection;
use crate::sbs::scale::{
	branch_base_radius_for_stalk, branch_radius_child_bounds_for_stalk,
	branch_radius_child_scale_for_stalk,
};
use procedural_common::{NoiseConfig, NoiseParams};

use crate::chain::liams_conifer::{
	liams_conifer_branch_depth, LiamsConiferChain, LiamsConiferPhase, SEGMENT_FRACS,
};
use crate::chain::BranchOut;
use crate::chain::DepthBudget;
use crate::BallStickNode;
use std::ops::Range;

/// Hysteresis type shared with Liam's Conifer chain growth ([`crate::chain::liams_conifer`]).
pub type FriendsConiferChain = LiamsConiferChain;

/// Downward tilt on ring radial seeds (playground-tuned; RFC §3.1.7.14 cites `2°`).
pub const FRIENDS_DOWNWARD_BIAS_RADIANS: f32 = 18.0_f32.to_radians();
/// [`BranchOut::ray_degrees_of_freedom`] at four spokes per ring ([#236](https://github.com/ramate-io/maybraid/issues/236)).
pub const FRIENDS_BRANCH_ANGLE_TOLERANCE_RADIANS: f32 = 37.0_f32.to_radians();
/// Blend toward [`downward_biased_radial`] at canopy seeds (`1.0` = fully biased ray).
pub const FRIENDS_BIAS_BLEND: f32 = 0.96;
/// Max / min projection length as fractions of stalk height (longer than Temperate `0.10` / `0.025`).
pub const FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT: f32 = 0.16;
pub const FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT: f32 = 0.03;
/// Temperate Conifer ([#238](https://github.com/ramate-io/maybraid/issues/238)) uses shorter limbs and wider ray DOF.
pub const TEMPERATE_MAX_PROJECTION_FRACTION_OF_HEIGHT: f32 = 0.10;
pub const TEMPERATE_MIN_PROJECTION_FRACTION_OF_HEIGHT: f32 = 0.025;
pub const TEMPERATE_BRANCH_ANGLE_TOLERANCE_RADIANS: f32 = 40.0_f32.to_radians();
/// Limb joint radius at ring anchors relative to stalk base radius.
pub const FRIENDS_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.20;
/// Per-hop thinning on child [`BranchOut::radius_range`]: `(lo, hi)`.
pub const FRIENDS_BRANCH_RADIUS_CHILD_SCALE: (f32, f32) = (0.84, 0.92);
const FRIENDS_REFERENCE_STALK_HEIGHT: f32 = 30.0;

/// Deterministic ring + stalk parameters before optional [`StalkPerturbation`].
#[derive(Clone, Debug, PartialEq)]
pub struct FriendsConiferProtoAnchors {
	pub stalk: StrictStalk,
	pub first_ring_unit_height: f32,
	pub last_ring_unit_height: f32,
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	pub max_projection_fraction_of_height: f32,
	pub min_projection_fraction_of_height: f32,
	pub projection_alpha: f32,
	pub projection_beta: f32,
	pub downward_bias_radians: f32,
	pub branch_angle_tolerance: f32,
	pub branch_depth: usize,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
	pub child_count_range: Range<usize>,
}

impl Default for FriendsConiferProtoAnchors {
	fn default() -> Self {
		let h = 30.0;
		Self {
			stalk: StrictStalk { stalk_height: h, stalk_base_radius: 0.025 * h },
			first_ring_unit_height: 0.10,
			last_ring_unit_height: 1.0,
			ring_spacing_unit_height: 0.04,
			anchors_per_ring: 6,
			max_projection_fraction_of_height: FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT,
			min_projection_fraction_of_height: FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT,
			projection_alpha: 8.0,
			projection_beta: 3.0,
			downward_bias_radians: FRIENDS_DOWNWARD_BIAS_RADIANS,
			branch_angle_tolerance: FRIENDS_BRANCH_ANGLE_TOLERANCE_RADIANS,
			branch_depth: 3,
			branch_base_radius_fraction_of_stalk: FRIENDS_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale: FRIENDS_BRANCH_RADIUS_CHILD_SCALE,
			child_count_range: 0..2,
		}
	}
}

impl HasStrictStalk for FriendsConiferProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

impl FriendsConiferProtoAnchors {
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
		let h = self.stalk.stalk_height.max(1e-6);
		let ell_max = h * self.max_projection_fraction_of_height;
		let ell_min = h * self.min_projection_fraction_of_height;
		logarithmic_rounding_projection(
			ell_max,
			ell_min,
			u,
			self.projection_alpha,
			self.projection_beta,
		)
	}

	fn limb_base_radius(&self) -> f32 {
		let base = self.stalk.stalk_base_radius.max(1e-4);
		branch_base_radius_for_stalk(
			base,
			self.branch_base_radius_fraction_of_stalk,
			0.05,
			self.stalk.stalk_height,
			FRIENDS_REFERENCE_STALK_HEIGHT,
		)
	}

	pub fn limb_terminal_radius_estimate(&self) -> f32 {
		let r = self.limb_base_radius();
		let (_s_lo, s_hi) = self.branch_radius_child_scale;
		r * s_hi.powi(self.branch_depth as i32)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<FriendsConiferChain> {
		let mut out = Vec::new();
		let k = self.anchors_per_ring.max(1);
		let radial_eps = (self.stalk.stalk_base_radius * 0.05).max(1e-4);
		let limb_r = self.limb_base_radius();
		let child_scale = branch_radius_child_scale_for_stalk(
			self.branch_radius_child_scale,
			self.stalk.stalk_height,
			FRIENDS_REFERENCE_STALK_HEIGHT,
		);
		let child_bounds = branch_radius_child_bounds_for_stalk(
			self.stalk.stalk_base_radius,
			limb_r,
			self.stalk.stalk_height,
			FRIENDS_REFERENCE_STALK_HEIGHT,
		);

		for z_frac in self.ring_height_fractions() {
			let u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let bias = downward_biased_radial(radial, self.downward_bias_radians);
				let pos = self.stalk.centroid_at_height_fraction(z_frac) + radial * radial_eps;

				let seed_node = BallStickNode::new(pos, limb_r);
				let first_len = proj * SEGMENT_FRACS[0];
				let branch = BranchOut::radial_out_horizontal(seed_node, radial)
					.with_hysteresis_context(chain_noise.clone(), 0, bias)
					.with_bias_blend(FRIENDS_BIAS_BLEND)
					.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
					.with_child_count(self.child_count_range.clone())
					.with_radius_range(limb_r..limb_r)
					.with_radius_range_child_scale(child_scale)
					.with_radius_range_child_bounds(child_bounds.clone())
					.with_length(first_len * 0.97..first_len * 1.03);

				let depth = liams_conifer_branch_depth(self.branch_depth);
				out.push(FriendsConiferChain::new(
					chain_noise.clone(),
					proj,
					depth,
					LiamsConiferPhase::BranchOut(DepthBudget { inner: branch, remaining: depth }),
				));
			}
		}

		let depth = liams_conifer_branch_depth(self.branch_depth);
		for a in self.stalk.point_to_point_anchors() {
			out.push(FriendsConiferChain::new(
				chain_noise.clone(),
				0.0,
				depth,
				LiamsConiferPhase::Stalk(a),
			));
		}

		out
	}
}

impl Anchors<FriendsConiferChain> for FriendsConiferProtoAnchors {
	fn anchors(&self) -> Vec<FriendsConiferChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendsConiferAnchors {
	pub perturbation: StalkPerturbation<FriendsConiferProtoAnchors>,
}

impl FriendsConiferAnchors {
	pub fn new(proto: FriendsConiferProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: FriendsConiferAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &FriendsConiferProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<FriendsConiferChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct FriendsConiferAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for FriendsConiferAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for FriendsConiferAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for FriendsConiferAnchors {
	fn default() -> Self {
		Self::new(FriendsConiferProtoAnchors::default())
	}
}

impl Anchors<FriendsConiferChain> for FriendsConiferAnchors {
	fn anchors(&self) -> Vec<FriendsConiferChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::sbs::friends_conifer::FriendsConiferSbs;
	use anyhow::Result;

	fn first_branch_radius(seeds: &[FriendsConiferChain]) -> Option<f32> {
		seeds
			.iter()
			.find_map(|seed| seed.active_branch_profile().map(|branch| branch.node.radius))
	}

	#[test]
	fn projection_endpoints_match_default_fractions() {
		let a = FriendsConiferProtoAnchors::default();
		let h = a.stalk.stalk_height;
		let l0 = a.projection_length(0.0);
		let l1 = a.projection_length(1.0);
		assert!((l0 - h * FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT).abs() < 1e-3);
		assert!((l1 - h * FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT).abs() < 1e-3);
	}

	#[test]
	fn lower_mid_canopy_projection_stays_near_max() {
		let a = FriendsConiferProtoAnchors::default();
		let h = a.stalk.stalk_height;
		let ell_max = h * FRIENDS_MAX_PROJECTION_FRACTION_OF_HEIGHT;
		let ell_min = h * FRIENDS_MIN_PROJECTION_FRACTION_OF_HEIGHT;
		let u = 0.25;
		let log = a.projection_length(u);
		let linear = ell_max + (ell_min - ell_max) * u;
		assert!(log > linear, "log profile should delay falloff: log={log} linear={linear}");
		assert!(log > ell_max * 0.9, "lower-mid canopy should stay near max: {log}");
	}

	#[test]
	fn ring_count_matches_rfc_spacing() {
		let a = FriendsConiferProtoAnchors::default();
		let rings = a.ring_height_fractions();
		let expected = ((a.last_ring_unit_height - a.first_ring_unit_height)
			/ a.ring_spacing_unit_height)
			.floor() as usize
			+ 1;
		assert_eq!(rings.len(), expected);
		assert!((rings[0] - 0.10).abs() < 1e-6);
		assert_eq!(a.anchors_per_ring, 6);
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let proto =
			FriendsConiferProtoAnchors { ring_spacing_unit_height: 0.20, ..Default::default() };
		let ring_count = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		let a = FriendsConiferAnchors::new(proto);
		assert_eq!(a.anchors().len(), ring_count * spokes + 1);
	}

	#[test]
	fn mini_limb_radius_scales_below_full_size_floor() -> Result<()> {
		let proto = FriendsConiferProtoAnchors {
			stalk: StrictStalk { stalk_height: 2.0, stalk_base_radius: 0.025 * 2.0 },
			..Default::default()
		};
		let branch_radius =
			first_branch_radius(&proto.hysteresis_seeds(NoiseConfig::new(NoiseParams::default())))
				.ok_or_else(|| anyhow::anyhow!("expected a mini Friend's Conifer branch seed"))?;
		assert!(branch_radius < 0.05, "mini branch radius should not hit full-size floor");
		assert!(
			branch_radius >= proto.stalk.stalk_base_radius * 0.10,
			"mini branch radius should remain readable relative to stalk"
		);
		Ok(())
	}

	#[test]
	fn mini_branch_radii_stay_within_stalk_relative_bounds() -> Result<()> {
		let proto = FriendsConiferProtoAnchors {
			stalk: StrictStalk { stalk_height: 3.0, stalk_base_radius: 0.025 * 3.0 },
			ring_spacing_unit_height: 0.20,
			..Default::default()
		};
		let chain = crate::anchors::AnchorsToChain::build_chain(&FriendsConiferAnchors::new(proto));
		let stalk = 0.025 * 3.0;
		let radii: Vec<f32> = chain
			.nodes_with_hysteresis()
			.filter_map(|(node, hysteresis)| {
				hysteresis.active_branch_profile().map(|_| node.radius)
			})
			.collect();
		let max = radii.iter().copied().fold(0.0_f32, f32::max);
		assert!(max <= stalk * 0.36, "friend max branch radius {max} vs stalk {stalk}");
		Ok(())
	}

	#[test]
	fn mini_sapling_branch_radii_stay_stalk_proportional_after_perturbation() -> Result<()> {
		let mut sbs = FriendsConiferSbs::default();
		sbs.scale.stalk_height = 3.0;
		sbs.canopy_noise = NoiseParams::from_scalar(5.0, 1.0, 1.0, 1);
		let chain = sbs.build_chain();
		let stalk = sbs.scale.stalk_base_radius_or_default();
		let branch_radii: Vec<f32> = chain
			.nodes_with_hysteresis()
			.filter_map(|(node, hysteresis)| {
				hysteresis.active_branch_profile().map(|_| node.radius)
			})
			.collect();
		let max = branch_radii.iter().copied().fold(0.0_f32, f32::max);
		assert!(
			max <= stalk * 0.35,
			"perturbed max branch radius {max} too large for stalk {stalk}"
		);
		Ok(())
	}
}
