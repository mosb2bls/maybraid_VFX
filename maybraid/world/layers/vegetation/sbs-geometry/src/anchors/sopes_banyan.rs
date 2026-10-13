//! **Stalk anchor rings** and projection policy for **Sope's Banyan** ([RFC-183 §3.1.7.6](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/06-sope-s-banyan/README.md), [#252](https://github.com/ramate-io/maybraid/issues/252)).
//!
//! # Intent
//!
//! Anchoring follows [§3.1.3 Ball-stick anchors](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/03-ball-stick-anchors/README.md): positions, initial rays, bias directions, and local scale for each canopy chain, usually emitted from the **stalk radial centroid** so limbs read as emerging from trunk mass.
//!
//! Compared to [Honu Banyan](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/05-honu-banyan/README.md), Sope's places rings **much lower**: radial work begins around **40%** of total height \(z_{\min} \approx 0.40 H\), extending to ~**90%** with **5–7** rings at spacing ~**0.08 H**, **6–8** anchors per ring. **Projection length** uses a bounded **logit vase profile** over normalized height \(u\), mixed between min/max projection fractions.
//!
//! # Same ball-stick graph as the stalk
//!
//! Sope's Banyan composes a **[`StrictStalk`](super::strict_stalk::StrictStalk)** (straight vertical centroid) **with** ring seeds at [`StrictStalk::centroid_at_height_fraction`]. Output is deterministic; a composing type can perturb or drop anchors later.

use std::f32::consts::TAU;

use bevy_math::Vec3;

use super::stalk_perturbation::{
	perturb_branch_out, perturb_node, AnchorPerturbation, HasStrictStalk, PerturbAnchor,
	StalkPerturbation,
};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::sopes_banyan::{at_stalk, SopesBanyanChain, SopesBanyanPhase};
use procedural_common::{NoiseConfig, NoiseParams};

use crate::chain::BranchOut;
use crate::projection::vase_projection_length;
use crate::BallStickNode;
use crate::DepthBudget;

/// RFC-style ring band and vase profile over [`StrictStalk::height`].
#[derive(Clone, Debug, PartialEq)]
pub struct SopesBanyanProtoAnchors {
	/// Vertical extent and base for ring placement.
	pub stalk: StrictStalk,
	/// First ring height as a fraction of [`StrictStalk::height`] above the tree-local origin (RFC ~0.4).
	pub first_ring_unit_height: f32,
	/// Last ring height fraction (RFC ~0.9).
	pub last_ring_unit_height: f32,
	pub ring_count: u32,
	pub anchors_per_ring: u32,
	/// Vase mix endpoints as fractions of stalk height: `length ≈ H * mix(min, max, vase_profile(u))`.
	pub projection_min_fraction_of_height: f32,
	pub projection_max_fraction_of_height: f32,
	/// Clamp epsilon for bounded logit vase profile.
	pub vase_profile_epsilon: f32,
	/// Center of the vase profile.
	pub projection_center_fraction: f32,
	/// Initial [`crate::DepthBudget::remaining`] at the first ring (RFC limb depth ~5 segments).
	pub max_depth_first_ring: usize,
	/// Initial depth budget at the last ring (~8).
	pub max_depth_last_ring: usize,
	pub descender_threshold: f32,
}

impl Default for SopesBanyanProtoAnchors {
	fn default() -> Self {
		Self {
			stalk: StrictStalk { stalk_height: 20.0, stalk_base_radius: 0.75 },
			first_ring_unit_height: 0.40,
			last_ring_unit_height: 0.95,
			ring_count: 8,
			anchors_per_ring: 7,
			projection_min_fraction_of_height: 0.10,
			projection_max_fraction_of_height: 0.20,
			vase_profile_epsilon: 0.4,
			projection_center_fraction: 0.5,
			max_depth_first_ring: 4,
			max_depth_last_ring: 8,
			descender_threshold: 0.01,
		}
	}
}

impl HasStrictStalk for SopesBanyanProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

impl SopesBanyanProtoAnchors {
	/// Normalized index along rings in `[0, 1]` (0 = lowest ring, 1 = highest).
	fn ring_mix_u(ring_index: u32, ring_count: u32) -> f32 {
		if ring_count <= 1 {
			return 0.0;
		}
		(ring_index as f32 / (ring_count - 1) as f32).clamp(0.0, 1.0)
	}

	/// Vase-shaped projection length for this ring (world units).
	fn projection_length(&self, u: f32) -> f32 {
		vase_projection_length(
			self.stalk.stalk_height,
			self.projection_min_fraction_of_height,
			self.projection_max_fraction_of_height,
			u,
			self.vase_profile_epsilon,
			self.projection_center_fraction,
		)
	}

	fn max_depth_for_ring(&self, u: f32) -> usize {
		let a = self.max_depth_first_ring as f32;
		let b = self.max_depth_last_ring as f32;
		(a + (b - a) * u).round().max(0.0) as usize
	}

	fn height_fraction_for_ring(&self, u: f32) -> f32 {
		let a = self.first_ring_unit_height;
		let b = self.last_ring_unit_height;
		a + (b - a) * u
	}
}

impl SopesBanyanProtoAnchors {
	/// Ring spokes as [`SopesBanyanChain`] seeds with explicit chain noise and vase/descender tuning.
	pub fn hysteresis_seeds(
		&self,
		chain_noise: NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> Vec<SopesBanyanChain> {
		let mut out = Vec::new();
		let n = self.ring_count.max(1);
		let k = self.anchors_per_ring.max(1);
		let stalk_h = self.stalk.stalk_height;
		let radial_eps = (self.stalk.stalk_base_radius * 0.08).max(1e-4);

		for r in 0..n {
			let u = Self::ring_mix_u(r, n);
			let y_frac = self.height_fraction_for_ring(u);
			let proj = self.projection_length(u);
			let max_depth = self.max_depth_for_ring(u);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let offset = radial * radial_eps;
				let pos = self.stalk.centroid_at_height_fraction(y_frac) + offset;

				let seed_node = BallStickNode::new(pos, at_stalk(stalk_h, 0.1));
				let noise = chain_noise.clone();
				let mut h = SopesBanyanChain::new(
					noise.clone().with_frequency(noise.params().frequency * 10.0),
					stalk_h,
					banyan_height,
					descender_threshold,
					SopesBanyanPhase::BranchOut(DepthBudget {
						inner: BranchOut::radial_out_horizontal(seed_node, radial)
							.with_hysteresis_context(noise, 0, radial)
							.with_ball_radius(at_stalk(stalk_h, 0.25))
							.with_radius_range(at_stalk(stalk_h, 0.24)..at_stalk(stalk_h, 0.28))
							.with_radius_range_child_scale((0.9, 0.95))
							.with_child_count(1..2)
							.with_ray_degrees_of_freedom(0.3),
						remaining: max_depth,
					}),
				);
				let lo = proj * 0.97;
				let hi = proj * 1.03;
				if let SopesBanyanPhase::BranchOut(ref mut w) = &mut h.phase {
					w.inner.length = lo..hi;
				}

				out.push(h);
			}
		}

		// add in the stalk anchor
		let stalk_anchors = self.stalk.point_to_point_anchors();
		out.extend(stalk_anchors.into_iter().map(|a| {
			SopesBanyanChain::new(
				chain_noise.clone(),
				stalk_h,
				banyan_height,
				descender_threshold,
				SopesBanyanPhase::Stalk(a),
			)
		}));

		out
	}
}

impl Anchors<SopesBanyanChain> for SopesBanyanProtoAnchors {
	fn anchors(&self) -> Vec<SopesBanyanChain> {
		self.hysteresis_seeds(
			NoiseConfig::new(NoiseParams::default()),
			self.stalk.stalk_height,
			self.descender_threshold,
		)
	}
}

/// Perturbing Sope's Banyan anchor recipe used by the public SBS front-end.
#[derive(Clone, Debug, PartialEq)]
pub struct SopesBanyanAnchors {
	pub perturbation: StalkPerturbation<SopesBanyanProtoAnchors>,
}

impl SopesBanyanAnchors {
	pub fn new(proto: SopesBanyanProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: SopesBanyanAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &SopesBanyanProtoAnchors {
		&self.perturbation.inner
	}

	pub fn proto_mut(&mut self) -> &mut SopesBanyanProtoAnchors {
		&mut self.perturbation.inner
	}

	/// Ring spokes as [`SopesBanyanChain`] seeds with explicit chain noise and stalk perturbation.
	pub fn hysteresis_seeds(
		&self,
		chain_noise: NoiseConfig,
		banyan_height: f32,
		descender_threshold: f32,
	) -> Vec<SopesBanyanChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise, banyan_height, descender_threshold);
		self.perturbation.perturb_anchors(seeds)
	}
}

/// Public Sope-specific knobs for non-stalk anchor perturbation.
#[derive(Clone, Debug, PartialEq)]
pub struct SopesBanyanAnchorPerturbation {
	pub noise: NoiseParams,
	pub vertical_offset: std::ops::Range<f32>,
	pub angular_scale: std::ops::Range<f32>,
	pub radius_offset: std::ops::Range<f32>,
}

impl Default for SopesBanyanAnchorPerturbation {
	fn default() -> Self {
		Self {
			noise: NoiseParams::default(),
			vertical_offset: -1.0..1.0,
			angular_scale: 0.0..0.5,
			radius_offset: -0.05..0.05,
		}
	}
}

impl HasStrictStalk for SopesBanyanAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for SopesBanyanAnchors {
	fn default() -> Self {
		Self::new(SopesBanyanProtoAnchors::default())
	}
}

impl Anchors<SopesBanyanChain> for SopesBanyanAnchors {
	fn anchors(&self) -> Vec<SopesBanyanChain> {
		self.hysteresis_seeds(
			NoiseConfig::new(NoiseParams::default()),
			self.proto().stalk.stalk_height,
			self.proto().descender_threshold,
		)
	}
}

impl PerturbAnchor for SopesBanyanChain {
	fn perturb_anchor(mut self, perturbation: AnchorPerturbation) -> Self {
		self.phase = match self.phase {
			SopesBanyanPhase::Stalk(mut p) => {
				p.start = perturb_node(p.start, perturbation);
				SopesBanyanPhase::Stalk(p)
			}
			SopesBanyanPhase::BranchOut(mut b) => {
				b.inner = perturb_branch_out(b.inner, perturbation);
				SopesBanyanPhase::BranchOut(b)
			}
			SopesBanyanPhase::StartFlairUp(mut s) => {
				s.projection = perturb_branch_out(s.projection, perturbation);
				SopesBanyanPhase::StartFlairUp(s)
			}
			SopesBanyanPhase::EndFlairUp(mut e) => {
				e.node = perturb_node(e.node, perturbation);
				SopesBanyanPhase::EndFlairUp(e)
			}
			SopesBanyanPhase::StartDescender(mut s) => {
				s.projection = perturb_branch_out(s.projection, perturbation);
				SopesBanyanPhase::StartDescender(s)
			}
			SopesBanyanPhase::EndDescender(mut e) => {
				e.node = perturb_node(e.node, perturbation);
				SopesBanyanPhase::EndDescender(e)
			}
		};
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vase_projection_grows_with_ring_height() {
		let a = SopesBanyanProtoAnchors {
			stalk: StrictStalk { stalk_height: 10.0, stalk_base_radius: 0.5 },
			ring_count: 5,
			..Default::default()
		};
		let l0 = a.projection_length(SopesBanyanProtoAnchors::ring_mix_u(0, 5));
		let l1 = a.projection_length(SopesBanyanProtoAnchors::ring_mix_u(4, 5));
		assert!(l1 > l0, "upper rings should get longer vase projections");

		let d0 = a.max_depth_for_ring(0.0);
		let d1 = a.max_depth_for_ring(1.0);
		assert!(d1 >= d0, "segment budget should not shrink upward by default");
	}

	#[test]
	fn anchors_count_matches_rings_times_spokes() {
		let a = SopesBanyanAnchors::new(SopesBanyanProtoAnchors {
			ring_count: 3,
			anchors_per_ring: 4,
			..Default::default()
		});
		assert_eq!(a.anchors().len(), 13);
	}
}
