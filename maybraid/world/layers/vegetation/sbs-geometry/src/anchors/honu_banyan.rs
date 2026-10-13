//! **Honu Banyan** stalk anchor rings ([#250](https://github.com/ramate-io/maybraid/issues/250), [RFC §3.1.7.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md)).
//!
//! High canopy band on **total tree height** `H`, broad horizontal spread, and stochastic descenders (see [`crate::chain::honu_banyan`]).
//! Compared to [Sope's Banyan](super::sopes_banyan), rings sit around **80–95%** of `H` with **2–3** rings and longer projections.

use std::f32::consts::TAU;

use bevy_math::Vec3;

use super::stalk_perturbation::{
	perturb_branch_out, perturb_node, AnchorPerturbation, HasStrictStalk, PerturbAnchor,
	StalkPerturbation,
};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::honu_banyan::{HonuBanyanChain, HonuBanyanPhase, HONU_CANOPY_RAY_DOF};
use procedural_common::{NoiseConfig, NoiseParams};

use crate::chain::BranchOut;
use crate::BallStickNode;
use crate::DepthBudget;

pub const DEFAULT_TREE_HEIGHT: f32 = 24.0;
pub const DEFAULT_STALK_HEIGHT_FRACTION: f32 = 0.80;
pub const DEFAULT_STALK_RADIUS_FRACTION: f32 = 0.08;
pub const DEFAULT_FIRST_RING_HEIGHT_FRACTION: f32 = 0.80;
pub const DEFAULT_LAST_RING_HEIGHT_FRACTION: f32 = 0.95;
pub const DEFAULT_PROJECTION_MIN_FRACTION: f32 = 0.55;
pub const DEFAULT_PROJECTION_MAX_FRACTION: f32 = 1.05;
pub const DEFAULT_PROJECTION_MIX_SCALE: f32 = 0.30;
pub const DEFAULT_MAX_DEPTH_FIRST_RING: usize = 3;
pub const DEFAULT_MAX_DEPTH_LAST_RING: usize = 5;
/// Noise threshold for descender candidacy (higher ⇒ more aerial roots).
pub const DEFAULT_DESCENDER_THRESHOLD: f32 = 0.22;
pub const DEFAULT_STALK_SECTION_COUNT: u32 = 6;

const LIMB_RADIUS_FRACTION_LO: f32 = 0.2;
const LIMB_RADIUS_FRACTION_HI: f32 = 0.25;
const LIMB_RADIUS_SPAN_FRACTION: f32 = 0.08;

const RADIAL_OFFSET_FRACTION_OF_STALK_BASE: f32 = 0.05;
/// Upward lift mixed into radial bias (~12° elevation vs RFC's ~3°).
const CANOPY_BIAS_UP: f32 = 0.20;

/// Canopy bias: broad spread with a clearer upward component than RFC baseline.
pub fn honu_canopy_bias(radial_xz: Vec3) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::Y;
	}
	(radial + Vec3::Y * CANOPY_BIAS_UP).normalize_or_zero()
}

/// `mix(max, min, u * mix_scale)` in world units for total height `H`.
pub fn honu_projection_length(
	tree_height: f32,
	u: f32,
	min_fraction: f32,
	max_fraction: f32,
	mix_scale: f32,
) -> f32 {
	let t = (u * mix_scale).clamp(0.0, 1.0);
	let frac = max_fraction + (min_fraction - max_fraction) * t;
	tree_height * frac
}

/// Joint ball radius at a ring height fraction of total `H` (thicker toward the crown).
pub fn honu_limb_joint_radius(height_fraction: f32, stalk_base_radius: f32) -> f32 {
	let u = height_fraction.clamp(0.0, 1.0);
	let scale = LIMB_RADIUS_FRACTION_LO + (LIMB_RADIUS_FRACTION_HI - LIMB_RADIUS_FRACTION_LO) * u;
	(stalk_base_radius * scale).max(0.04)
}

/// [`BranchOut::radius_range`] for a ring spoke from [`honu_limb_joint_radius`].
pub fn honu_limb_radius_range(
	height_fraction: f32,
	stalk_base_radius: f32,
) -> std::ops::Range<f32> {
	let r = honu_limb_joint_radius(height_fraction, stalk_base_radius);
	let span = (r * LIMB_RADIUS_SPAN_FRACTION).max(0.01);
	(r - span)..(r + span)
}

/// Ring band and projection over total tree height `H`.
#[derive(Clone, Debug, PartialEq)]
pub struct HonuBanyanProtoAnchors {
	pub tree_height: f32,
	pub stalk: StrictStalk,
	pub first_ring_height_fraction: f32,
	pub last_ring_height_fraction: f32,
	pub ring_count: u32,
	pub anchors_per_ring: u32,
	pub projection_min_fraction: f32,
	pub projection_max_fraction: f32,
	pub projection_mix_scale: f32,
	pub max_depth_first_ring: usize,
	pub max_depth_last_ring: usize,
	pub descender_threshold: f32,
	pub stalk_section_count: u32,
}

impl Default for HonuBanyanProtoAnchors {
	fn default() -> Self {
		let tree_height = DEFAULT_TREE_HEIGHT;
		Self {
			tree_height,
			stalk: StrictStalk {
				stalk_height: tree_height * DEFAULT_STALK_HEIGHT_FRACTION,
				stalk_base_radius: tree_height * DEFAULT_STALK_RADIUS_FRACTION,
			},
			first_ring_height_fraction: DEFAULT_FIRST_RING_HEIGHT_FRACTION,
			last_ring_height_fraction: DEFAULT_LAST_RING_HEIGHT_FRACTION,
			ring_count: 3,
			anchors_per_ring: 4,
			projection_min_fraction: DEFAULT_PROJECTION_MIN_FRACTION,
			projection_max_fraction: DEFAULT_PROJECTION_MAX_FRACTION,
			projection_mix_scale: DEFAULT_PROJECTION_MIX_SCALE,
			max_depth_first_ring: DEFAULT_MAX_DEPTH_FIRST_RING,
			max_depth_last_ring: DEFAULT_MAX_DEPTH_LAST_RING,
			descender_threshold: DEFAULT_DESCENDER_THRESHOLD,
			stalk_section_count: DEFAULT_STALK_SECTION_COUNT,
		}
	}
}

impl HasStrictStalk for HonuBanyanProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

impl HonuBanyanProtoAnchors {
	fn ring_mix_u(ring_index: u32, ring_count: u32) -> f32 {
		if ring_count <= 1 {
			return 0.0;
		}
		(ring_index as f32 / (ring_count - 1) as f32).clamp(0.0, 1.0)
	}

	fn projection_length(&self, u: f32) -> f32 {
		honu_projection_length(
			self.tree_height,
			u,
			self.projection_min_fraction,
			self.projection_max_fraction,
			self.projection_mix_scale,
		)
	}

	fn max_depth_for_ring(&self, u: f32) -> usize {
		let a = self.max_depth_first_ring as f32;
		let b = self.max_depth_last_ring as f32;
		(a + (b - a) * u).round().max(1.0) as usize
	}

	fn ring_height_fraction(&self, u: f32) -> f32 {
		let a = self.first_ring_height_fraction;
		let b = self.last_ring_height_fraction;
		a + (b - a) * u
	}

	/// Tree-local ring anchor (the spawned root entity owns world placement).
	fn ring_local_position(&self, height_fraction: f32, radial_offset: Vec3) -> Vec3 {
		Vec3::Y * (height_fraction * self.tree_height) + radial_offset
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<HonuBanyanChain> {
		let mut out = Vec::new();
		let n = self.ring_count.max(1);
		let k = self.anchors_per_ring.max(1);
		let radial_eps =
			(self.stalk.stalk_base_radius * RADIAL_OFFSET_FRACTION_OF_STALK_BASE).max(1e-4);

		for r in 0..n {
			let u = Self::ring_mix_u(r, n);
			let y_frac = self.ring_height_fraction(u);
			let proj = self.projection_length(u);
			let max_depth = self.max_depth_for_ring(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let pos = self.ring_local_position(y_frac, radial * radial_eps);
				let bias = honu_canopy_bias(radial);
				let joint_r = honu_limb_joint_radius(y_frac, self.stalk.stalk_base_radius);
				let radius_range = honu_limb_radius_range(y_frac, self.stalk.stalk_base_radius);

				let seed_node = BallStickNode::new(pos, joint_r);
				let noise = chain_noise.clone();
				let mut h = HonuBanyanChain::new(
					noise.clone().with_frequency(noise.params().frequency * 10.0),
					self.tree_height,
					u,
					proj,
					max_depth,
					0.0,
					self.descender_threshold,
					HonuBanyanPhase::BranchOut(DepthBudget {
						inner: BranchOut::radial_out_horizontal(seed_node, radial)
							.with_hysteresis_context(noise, 0, radial)
							.with_bias_ray(bias)
							.with_bias_blend(0.9)
							.with_ball_radius(joint_r)
							.with_radius_range(radius_range)
							.with_radius_range_child_scale((0.82, 0.88))
							.with_child_count(1..3)
							.with_ray_degrees_of_freedom(HONU_CANOPY_RAY_DOF),
						remaining: max_depth,
					}),
				);
				// Per-hop length (not the full projection) so the limb grows in even segments.
				let hop = (proj / (max_depth as f32).max(1.0)).max(0.1);
				let lo = hop * 0.97;
				let hi = hop * 1.03;
				if let HonuBanyanPhase::BranchOut(ref mut w) = &mut h.phase {
					w.inner.length = lo..hi;
				}

				out.push(h);
			}
		}

		let stalk_anchors = self.stalk.segmented_point_to_point_anchors(self.stalk_section_count);
		out.extend(stalk_anchors.into_iter().map(|a| {
			HonuBanyanChain::new(
				chain_noise.clone(),
				self.tree_height,
				0.0,
				0.0,
				0,
				0.0,
				self.descender_threshold,
				HonuBanyanPhase::Stalk(a),
			)
		}));

		out
	}
}

impl Anchors<HonuBanyanChain> for HonuBanyanProtoAnchors {
	fn anchors(&self) -> Vec<HonuBanyanChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct HonuBanyanAnchors {
	pub perturbation: StalkPerturbation<HonuBanyanProtoAnchors>,
}

impl HonuBanyanAnchors {
	pub fn new(proto: HonuBanyanProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: HonuBanyanAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &HonuBanyanProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<HonuBanyanChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

#[derive(Clone, Debug, PartialEq)]
pub struct HonuBanyanAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for HonuBanyanAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -0.5..0.5,
			angular_scale: 0.0..0.25,
			radius_offset: -0.04..0.04,
		}
	}
}

impl HasStrictStalk for HonuBanyanAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for HonuBanyanAnchors {
	fn default() -> Self {
		Self::new(HonuBanyanProtoAnchors::default())
	}
}

impl Anchors<HonuBanyanChain> for HonuBanyanAnchors {
	fn anchors(&self) -> Vec<HonuBanyanChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl PerturbAnchor for HonuBanyanChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = match self.phase {
			HonuBanyanPhase::Stalk(mut p) => {
				p.start = perturb_node(p.start, perturbation);
				HonuBanyanPhase::Stalk(p)
			}
			HonuBanyanPhase::BranchOut(mut b) => {
				b.inner = perturb_branch_out(b.inner, perturbation);
				HonuBanyanPhase::BranchOut(b)
			}
			HonuBanyanPhase::StartDescender(mut s) => {
				s.projection = perturb_branch_out(s.projection, perturbation);
				HonuBanyanPhase::StartDescender(s)
			}
			HonuBanyanPhase::EndDescender(mut e) => {
				e.node = perturb_node(e.node, perturbation);
				HonuBanyanPhase::EndDescender(e)
			}
		};
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn projection_shortens_toward_top_ring() {
		let a = HonuBanyanProtoAnchors::default();
		let l0 = a.projection_length(HonuBanyanProtoAnchors::ring_mix_u(0, 3));
		let l2 = a.projection_length(HonuBanyanProtoAnchors::ring_mix_u(2, 3));
		assert!(l0 > l2, "upper rings mix toward shorter projections");
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes_plus_stalk() {
		let a = HonuBanyanAnchors::new(HonuBanyanProtoAnchors {
			ring_count: 3,
			anchors_per_ring: 4,
			..Default::default()
		});
		assert_eq!(a.anchors().len(), 13);
	}

	#[test]
	fn limb_joint_radius_grows_with_ring_height() {
		let stalk_r = 2.0;
		let lo = honu_limb_joint_radius(0.80, stalk_r);
		let hi = honu_limb_joint_radius(0.95, stalk_r);
		assert!(hi > lo);
	}

	#[test]
	fn honu_canopy_bias_is_mostly_horizontal() {
		let b = honu_canopy_bias(Vec3::X);
		assert!(b.y > 0.1 && b.y < 0.25, "upward lift {b:?}");
		assert!(b.x.abs() > 0.85);
	}
}
