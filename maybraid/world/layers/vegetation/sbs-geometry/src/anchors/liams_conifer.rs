//! **Stalk anchor rings** for **Liam's Conifer** ([RFC-183 §3.1.7.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/02-liam-s-conifer/README.md), [#244](https://github.com/ramate-io/maybraid/issues/244)).
//!
//! # Layout
//!
//! One shared [`crate::BallStickChain`] per tree:
//!
//! - **Stalk** — a single [`crate::chain::point_to_point::PointToPoint`] seed from ground to crown.
//! - **Canopy** — many ring seeds on the stalk radial centroid; each becomes a [`LiamsConiferChain`] limb.
//!
//! Ring density and projection taper are art-directed here; segment growth and radius down-stepping
//! live in [`crate::chain::liams_conifer`].

use std::f32::consts::TAU;

use bevy_math::Vec3;

use super::stalk_perturbation::{
	perturb_branch_out, perturb_node, AnchorPerturbation, HasStrictStalk, PerturbAnchor,
	StalkPerturbation,
};
use super::strict_stalk::StrictStalk;
use super::Anchors;
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

const LIAMS_REFERENCE_STALK_HEIGHT: f32 = 30.0;

/// Tilt horizontal radial slightly toward −Y (RFC ~2° downward bias).
pub(crate) fn downward_biased_radial(radial_xz: Vec3, bias_radians: f32) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::NEG_Y;
	}
	let cos = bias_radians.cos();
	let sin = bias_radians.sin();
	Vec3::new(radial.x * cos, -sin, radial.z * cos).normalize_or_zero()
}

/// Deterministic ring + stalk parameters before optional [`StalkPerturbation`].
#[derive(Clone, Debug, PartialEq)]
pub struct LiamsConiferProtoAnchors {
	pub stalk: StrictStalk,
	/// First ring height as a fraction of [`StrictStalk::stalk_height`] (RFC ~0.10).
	pub first_ring_unit_height: f32,
	/// Last ring height fraction (RFC ~0.98).
	pub last_ring_unit_height: f32,
	/// Vertical spacing between rings as a fraction of stalk height (RFC ~0.04).
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	/// Max projection length as a fraction of stalk height (RFC example `0.05 * H`; playground defaults higher).
	pub max_projection_fraction_of_height: f32,
	/// Floor on [`Self::projection_length`] as a fraction of the ring's max projection.
	pub min_projection_fraction_of_max: f32,
	pub downward_bias_radians: f32,
	/// [`BranchOut::ray_degrees_of_freedom`] at canopy seeds (RFC ~8°).
	pub branch_angle_tolerance: f32,
	/// Limb hops at each ring seed; coerced via [`liams_conifer_branch_depth`](crate::chain::liams_conifer::liams_conifer_branch_depth) (`1..=3`, RFC default `3`).
	pub branch_depth: usize,
	/// Ball radius at the ring anchor and initial [`BranchOut::radius_range`] (both ends).
	///
	/// Limb thinning along the chain comes only from [`Self::branch_radius_child_scale`], which
	/// multiplies `radius_range` on each [`BranchOut`] hop (see `expand_children` in
	/// [`crate::chain::branch_out`]).
	pub branch_base_radius_fraction_of_stalk: f32,
	/// Per-segment multipliers on child [`BranchOut::radius_range`]: `(lo, hi)`.
	pub branch_radius_child_scale: (f32, f32),
	/// When true, [`Self::projection_length`] is \(\ell_{\max}(1-u)\) only (no [`Self::min_projection_fraction_of_max`] floor).
	pub linear_projection_taper: bool,
}

impl Default for LiamsConiferProtoAnchors {
	fn default() -> Self {
		let h = 30.0;
		Self {
			stalk: StrictStalk { stalk_height: h, stalk_base_radius: 0.025 * h },
			first_ring_unit_height: 0.10,
			last_ring_unit_height: 1.0,
			ring_spacing_unit_height: 0.03,
			anchors_per_ring: 6,
			max_projection_fraction_of_height: 0.15,
			min_projection_fraction_of_max: 0.20,
			downward_bias_radians: 2.0_f32.to_radians(),
			branch_angle_tolerance: 8.0_f32.to_radians(),
			branch_depth: 3,
			branch_base_radius_fraction_of_stalk: 0.1,
			branch_radius_child_scale: (0.72, 0.80),
			linear_projection_taper: false,
		}
	}
}

impl HasStrictStalk for LiamsConiferProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

impl LiamsConiferProtoAnchors {
	/// Ring center heights as fractions of [`StrictStalk::stalk_height`], stepping by [`Self::ring_spacing_unit_height`].
	pub fn ring_height_fractions(&self) -> Vec<f32> {
		let mut out = Vec::new();
		let mut z = self.first_ring_unit_height;
		while z <= self.last_ring_unit_height + 1e-6 {
			out.push(z);
			z += self.ring_spacing_unit_height;
		}
		out
	}

	/// Normalized index along the ring band: `0` = lowest ring, `1` = highest.
	fn ring_mix_u(&self, z_frac: f32) -> f32 {
		let a = self.first_ring_unit_height;
		let b = self.last_ring_unit_height;
		if (b - a).abs() < 1e-6 {
			return 0.0;
		}
		((z_frac - a) / (b - a)).clamp(0.0, 1.0)
	}

	/// Ring projection budget: \(\ell_{\max}(1-u)\), optionally floored at [`Self::min_projection_fraction_of_max`].
	pub fn projection_length(&self, u: f32) -> f32 {
		let h = self.stalk.stalk_height.max(1e-6);
		let ell_max = h * self.max_projection_fraction_of_height;
		let raw = ell_max * (1.0 - u.clamp(0.0, 1.0));
		if self.linear_projection_taper {
			raw
		} else {
			raw.max(ell_max * self.min_projection_fraction_of_max)
		}
	}

	/// Joint radius at a ring spoke before any [`BranchOut`] down-stepping.
	fn limb_base_radius(&self) -> f32 {
		let base = self.stalk.stalk_base_radius.max(1e-4);
		branch_base_radius_for_stalk(
			base,
			self.branch_base_radius_fraction_of_stalk,
			0.035,
			self.stalk.stalk_height,
			LIAMS_REFERENCE_STALK_HEIGHT,
		)
	}

	/// Approximate terminal joint radius after `branch_depth` down-steps (for tests / tuning).
	pub fn limb_terminal_radius_estimate(&self) -> f32 {
		let r = self.limb_base_radius();
		let (_s_lo, s_hi) = self.branch_radius_child_scale;
		r * s_hi.powi(self.branch_depth as i32)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<LiamsConiferChain> {
		let mut out = Vec::new();
		let k = self.anchors_per_ring.max(1);
		let radial_eps = (self.stalk.stalk_base_radius * 0.05).max(1e-4);
		let limb_r = self.limb_base_radius();
		let child_scale = branch_radius_child_scale_for_stalk(
			self.branch_radius_child_scale,
			self.stalk.stalk_height,
			LIAMS_REFERENCE_STALK_HEIGHT,
		);
		let child_bounds = branch_radius_child_bounds_for_stalk(
			self.stalk.stalk_base_radius,
			limb_r,
			self.stalk.stalk_height,
			LIAMS_REFERENCE_STALK_HEIGHT,
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
				// Collapsed range at the anchor: thickness is limb_r; child_scale thins each hop.
				let branch = BranchOut::radial_out_horizontal(seed_node, radial)
					.with_hysteresis_context(chain_noise.clone(), 0, bias)
					.with_bias_blend(0.92)
					.with_ray_degrees_of_freedom(self.branch_angle_tolerance)
					.with_child_count(1..2)
					.with_radius_range(limb_r..limb_r)
					.with_radius_range_child_scale(child_scale)
					.with_radius_range_child_bounds(child_bounds.clone())
					.with_length(first_len * 0.97..first_len * 1.03)
					.single_child();

				let depth = liams_conifer_branch_depth(self.branch_depth);
				out.push(LiamsConiferChain::new(
					chain_noise.clone(),
					proj,
					depth,
					LiamsConiferPhase::BranchOut(DepthBudget { inner: branch, remaining: depth }),
				));
			}
		}

		let depth = liams_conifer_branch_depth(self.branch_depth);
		for a in self.stalk.point_to_point_anchors() {
			out.push(LiamsConiferChain::new(
				chain_noise.clone(),
				0.0,
				depth,
				LiamsConiferPhase::Stalk(a),
			));
		}

		out
	}
}

impl Anchors<LiamsConiferChain> for LiamsConiferProtoAnchors {
	fn anchors(&self) -> Vec<LiamsConiferChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

/// Perturbing wrapper used by [`crate::sbs::liams_conifer::LiamsConiferSbs`].
#[derive(Clone, Debug, PartialEq)]
pub struct LiamsConiferAnchors {
	pub perturbation: StalkPerturbation<LiamsConiferProtoAnchors>,
}

impl LiamsConiferAnchors {
	pub fn new(proto: LiamsConiferProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: LiamsConiferAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &LiamsConiferProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<LiamsConiferChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct LiamsConiferAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for LiamsConiferAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for LiamsConiferAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for LiamsConiferAnchors {
	fn default() -> Self {
		Self::new(LiamsConiferProtoAnchors::default())
	}
}

impl Anchors<LiamsConiferChain> for LiamsConiferAnchors {
	fn anchors(&self) -> Vec<LiamsConiferChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl PerturbAnchor for LiamsConiferChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = match self.phase {
			LiamsConiferPhase::Stalk(mut p) => {
				p.start = perturb_node(p.start, perturbation);
				LiamsConiferPhase::Stalk(p)
			}
			LiamsConiferPhase::BranchOut(mut b) => {
				b.inner = perturb_branch_out(b.inner, perturbation);
				LiamsConiferPhase::BranchOut(b)
			}
		};
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	fn first_branch_radius(seeds: &[LiamsConiferChain]) -> Option<f32> {
		seeds
			.iter()
			.find_map(|seed| seed.active_branch_profile().map(|branch| branch.node.radius))
	}

	#[test]
	fn projection_length_shrinks_with_ring_height() {
		let a = LiamsConiferProtoAnchors::default();
		let z_low = a.first_ring_unit_height;
		let z_high = a.last_ring_unit_height;
		let l0 = a.projection_length(a.ring_mix_u(z_low));
		let l1 = a.projection_length(a.ring_mix_u(z_high));
		assert!(l0 > l1, "upper rings should get shorter projections");
	}

	#[test]
	fn ring_count_matches_spacing_band() {
		let a = LiamsConiferProtoAnchors::default();
		let rings = a.ring_height_fractions();
		let expected = ((a.last_ring_unit_height - a.first_ring_unit_height)
			/ a.ring_spacing_unit_height)
			.floor() as usize
			+ 1;
		assert_eq!(rings.len(), expected);
		assert!((rings[0] - 0.10).abs() < 1e-6);
		assert!(
			rings[expected - 1] <= a.last_ring_unit_height + 1e-6,
			"last ring should reach the top band"
		);
	}

	#[test]
	fn base_ring_projection_scales_with_stalk_height() {
		let a = LiamsConiferProtoAnchors::default();
		let proj = a.projection_length(a.ring_mix_u(a.first_ring_unit_height));
		assert!(
			(proj - a.stalk.stalk_height * a.max_projection_fraction_of_height).abs() < 1e-4,
			"expected max projection at lowest ring"
		);
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let proto =
			LiamsConiferProtoAnchors { ring_spacing_unit_height: 0.20, ..Default::default() };
		let ring_count = proto.ring_height_fractions().len();
		let spokes = proto.anchors_per_ring as usize;
		let a = LiamsConiferAnchors::new(proto);
		assert_eq!(a.anchors().len(), ring_count * spokes + 1);
	}

	#[test]
	fn limb_radius_tapers_via_child_scale_only() {
		let a = LiamsConiferProtoAnchors::default();
		let base = a.limb_base_radius();
		let terminal = a.limb_terminal_radius_estimate();
		assert!(terminal < base, "down-stepping should thin limbs toward tips");
	}

	#[test]
	fn mini_branch_radii_stay_within_stalk_relative_bounds() -> Result<()> {
		let proto = LiamsConiferProtoAnchors {
			stalk: StrictStalk { stalk_height: 3.0, stalk_base_radius: 0.025 * 3.0 },
			ring_spacing_unit_height: 0.20,
			..Default::default()
		};
		let chain = crate::anchors::AnchorsToChain::build_chain(&LiamsConiferAnchors::new(proto));
		let stalk = 0.025 * 3.0;
		let radii: Vec<f32> = chain
			.nodes_with_hysteresis()
			.filter_map(|(node, hysteresis)| {
				hysteresis.active_branch_profile().map(|_| node.radius)
			})
			.collect();
		let max = radii.iter().copied().fold(0.0_f32, f32::max);
		assert!(max <= stalk * 0.36, "liams max branch radius {max} vs stalk {stalk}");
		Ok(())
	}

	#[test]
	fn mini_limb_radius_scales_below_full_size_floor() -> Result<()> {
		let proto = LiamsConiferProtoAnchors {
			stalk: StrictStalk { stalk_height: 2.0, stalk_base_radius: 0.025 * 2.0 },
			..Default::default()
		};
		let branch_radius =
			first_branch_radius(&proto.hysteresis_seeds(NoiseConfig::new(NoiseParams::default())))
				.ok_or_else(|| anyhow::anyhow!("expected a mini Liam's Conifer branch seed"))?;
		assert!(branch_radius < 0.035, "mini branch radius should not hit full-size floor");
		assert!(
			branch_radius >= proto.stalk.stalk_base_radius * 0.08,
			"mini branch radius should remain readable relative to stalk"
		);
		Ok(())
	}
}
