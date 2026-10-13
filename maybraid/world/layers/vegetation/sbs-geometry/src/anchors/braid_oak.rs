//! **Braid Oak** stalk anchor rings ([#234](https://github.com/ramate-io/maybraid/issues/234), [RFC §3.1.7.13](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/13-braid-oak/README.md)).

use std::f32::consts::TAU;

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::stalk_perturbation::{HasStrictStalk, StalkPerturbation};
use super::storybook_tree::{
	storybook_dome_projection_length, StorybookTreeAnchorPerturbation,
	DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION, DEFAULT_TREE_HEIGHT,
};
use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::storybook_tree::{
	segment_fracs, storybook_branch_depth, StorybookTreeChain, StorybookTreePhase,
};
use crate::chain::{BranchOut, DepthBudget};
use crate::BallStickNode;
use procedural_common::NoiseParams;

/// RFC stalk height as a fraction of total tree height `H`.
pub const BRAID_STALK_HEIGHT_FRACTION: f32 = 0.75;

/// RFC stalk base radius as a fraction of `H` (art-direction: sturdy expressive trunk).
pub const BRAID_STALK_BASE_RADIUS_FRACTION: f32 = 0.06;

/// Limb radius at ring anchors as a fraction of stalk base radius (storybook: `0.12`).
pub const BRAID_BRANCH_BASE_RADIUS_FRACTION_OF_STALK: f32 = 0.20;

/// Minimum radial spokes sampled per ring.
pub const BRAID_ANCHORS_PER_RING_MIN: u32 = 3;

/// Maximum radial spokes sampled per ring (RFC / storybook: up to `6`).
pub const BRAID_ANCHORS_PER_RING_MAX: u32 = 6;

/// [`BranchOut`](crate::chain::BranchOut) hops per limb (storybook: `4`).
pub const BRAID_BRANCH_DEPTH: usize = 3;

/// Lowest ring along the stalk as a unit height fraction.
pub const BRAID_FIRST_RING_UNIT_HEIGHT: f32 = 0.28;

/// Vertical spacing between ring planes as a stalk-unit fraction.
pub const BRAID_RING_SPACING_UNIT_HEIGHT: f32 = 0.11;

/// Max projection at the crown belt as a fraction of `H`.
pub const BRAID_MAX_PROJECTION_FRACTION: f32 = 0.60;

/// End-ring minimum projection as a fraction of `H` (RFC `0.15 * H`).
pub const BRAID_PROJECTION_MIN_FRACTION: f32 = 0.15;

/// Stalk segments along the centroid (multi-hop [`PointToPoint`]).
pub const BRAID_STALK_SECTION_COUNT: u32 = 5;

/// Vertical bias at the lowest rings (RFC `mix(-0.35, …, u)`).
pub const BRAID_VERTICAL_BIAS_LOW: f32 = -0.35;
/// Vertical bias at the highest rings (RFC `mix(…, 0.45, u)`).
pub const BRAID_VERTICAL_BIAS_HIGH: f32 = 0.45;

/// [`BranchOut::with_bias_blend`] for braid limbs.
pub const BRAID_BIAS_BLEND: f32 = 0.88;

/// Fixed child count at ring seeds (RFC `2..=3`; art-direction: always `2`).
pub const BRAID_CHILD_COUNT: u32 = 2;

/// Per-hop radius scale range on canopy limbs.
pub const BRAID_BRANCH_RADIUS_CHILD_SCALE_LO: f32 = 0.82;
pub const BRAID_BRANCH_RADIUS_CHILD_SCALE_HI: f32 = 0.90;

/// Wider branch fan-out than RFC `18°` for expressive braiding.
pub const BRAID_ANGLE_TOLERANCE_DEGREES: f32 = 32.0;

/// Noise lane for per-ring spoke sampling ([`NoiseConfig::sample_range_usize_4d`]).
const BRAID_SPOKE_SAMPLE_LANE: f32 = 11.0;

/// World leaf radius fraction of `H`.
pub const BRAID_LEAF_RADIUS_FRACTION: f32 = 0.085;

/// Height-dependent branch bias: drooping lower rings, rising upper rings (RFC §3.1.7.13).
pub fn braid_vertical_bias_radial(radial_xz: Vec3, ring_u: f32) -> Vec3 {
	let radial = Vec3::new(radial_xz.x, 0.0, radial_xz.z).normalize_or_zero();
	if radial.length_squared() < 1e-12 {
		return Vec3::Y;
	}
	let u = ring_u.clamp(0.0, 1.0);
	let vertical_bias =
		BRAID_VERTICAL_BIAS_LOW + (BRAID_VERTICAL_BIAS_HIGH - BRAID_VERTICAL_BIAS_LOW) * u;
	(radial + Vec3::Y * vertical_bias).normalize_or_zero()
}

#[derive(Clone, Debug, PartialEq)]
pub struct BraidOakTreeProtoAnchors {
	pub tree_height: f32,
	pub stalk: StrictStalk,
	pub first_ring_unit_height: f32,
	pub last_ring_unit_height: f32,
	pub ring_spacing_unit_height: f32,
	pub anchors_per_ring: u32,
	pub max_projection_fraction_of_height: f32,
	pub projection_min_fraction_of_height: f32,
	pub branch_angle_tolerance: f32,
	pub bias_blend: f32,
	pub branch_depth: usize,
	pub child_count_min: u32,
	pub child_count_max: u32,
	pub outer_foliage_distance_fraction: f32,
	pub branch_base_radius_fraction_of_stalk: f32,
	pub branch_radius_child_scale: (f32, f32),
}

impl Default for BraidOakTreeProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_TREE_HEIGHT;
		let stalk_h = h * BRAID_STALK_HEIGHT_FRACTION;
		Self {
			tree_height: h,
			stalk: StrictStalk {
				stalk_height: stalk_h,
				stalk_base_radius: BRAID_STALK_BASE_RADIUS_FRACTION * h,
			},
			first_ring_unit_height: BRAID_FIRST_RING_UNIT_HEIGHT,
			last_ring_unit_height: 1.0,
			ring_spacing_unit_height: BRAID_RING_SPACING_UNIT_HEIGHT,
			anchors_per_ring: BRAID_ANCHORS_PER_RING_MAX,
			max_projection_fraction_of_height: BRAID_MAX_PROJECTION_FRACTION,
			projection_min_fraction_of_height: BRAID_PROJECTION_MIN_FRACTION,
			branch_angle_tolerance: BRAID_ANGLE_TOLERANCE_DEGREES.to_radians(),
			bias_blend: BRAID_BIAS_BLEND,
			branch_depth: BRAID_BRANCH_DEPTH,
			child_count_min: BRAID_CHILD_COUNT,
			child_count_max: BRAID_CHILD_COUNT,
			outer_foliage_distance_fraction: DEFAULT_OUTER_FOLIAGE_DISTANCE_FRACTION,
			branch_base_radius_fraction_of_stalk: BRAID_BRANCH_BASE_RADIUS_FRACTION_OF_STALK,
			branch_radius_child_scale: (
				BRAID_BRANCH_RADIUS_CHILD_SCALE_LO,
				BRAID_BRANCH_RADIUS_CHILD_SCALE_HI,
			),
		}
	}
}

impl BraidOakTreeProtoAnchors {
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

	fn limb_base_radius(&self) -> f32 {
		let base = self.stalk.stalk_base_radius.max(1e-4);
		(base * self.branch_base_radius_fraction_of_stalk).max(0.02)
	}

	fn sample_anchors_per_ring(
		&self,
		chain_noise: &NoiseConfig,
		z_frac: f32,
		ring_index: u32,
	) -> u32 {
		let min = BRAID_ANCHORS_PER_RING_MIN;
		let max = self.anchors_per_ring.clamp(min, BRAID_ANCHORS_PER_RING_MAX);
		if max <= min {
			return max.max(1);
		}
		chain_noise.sample_range_usize_4d(
			min as usize,
			(max as usize).saturating_add(1),
			z_frac,
			ring_index as f32,
			self.tree_height,
			BRAID_SPOKE_SAMPLE_LANE,
		) as u32
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let mut out = Vec::new();
		let radial_eps = (self.stalk.stalk_base_radius * 0.05).max(1e-4);
		let limb_r = self.limb_base_radius();
		let depth = storybook_branch_depth(self.branch_depth);
		let fracs = segment_fracs(depth);

		for (ring_index, z_frac) in self.ring_height_fractions().into_iter().enumerate() {
			let u = self.ring_mix_u(z_frac);
			let proj = self.projection_length(u);
			let k = self.sample_anchors_per_ring(&chain_noise, z_frac, ring_index as u32).max(1);

			for i in 0..k {
				let theta = TAU * (i as f32) / (k as f32);
				let radial = Vec3::new(theta.cos(), 0.0, theta.sin());
				let bias = braid_vertical_bias_radial(radial, u);
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
					.with_radius_range_child_scale(self.branch_radius_child_scale)
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

		for a in self.stalk.segmented_point_to_point_anchors(BRAID_STALK_SECTION_COUNT) {
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

impl HasStrictStalk for BraidOakTreeProtoAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		&self.stalk
	}
}

/// Perturbing wrapper used by [`crate::sbs::braid_oak_tree::BraidOakTreeSbs`].
#[derive(Clone, Debug, PartialEq)]
pub struct BraidOakTreeAnchors {
	pub perturbation: StalkPerturbation<BraidOakTreeProtoAnchors>,
}

impl BraidOakTreeAnchors {
	pub fn new(proto: BraidOakTreeProtoAnchors) -> Self {
		Self { perturbation: StalkPerturbation::new(proto) }
	}

	pub fn with_perturbation(mut self, perturbation: StorybookTreeAnchorPerturbation) -> Self {
		self.perturbation.noise = perturbation.noise;
		self.perturbation.vertical_offset = perturbation.vertical_offset;
		self.perturbation.angular_scale = perturbation.angular_scale;
		self.perturbation.radius_offset = perturbation.radius_offset;
		self
	}

	pub fn proto(&self) -> &BraidOakTreeProtoAnchors {
		&self.perturbation.inner
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<StorybookTreeChain> {
		let seeds = self.proto().hysteresis_seeds(chain_noise);
		self.perturbation.perturb_anchors(seeds)
	}
}

impl HasStrictStalk for BraidOakTreeAnchors {
	fn strict_stalk(&self) -> &StrictStalk {
		self.proto().strict_stalk()
	}
}

impl Default for BraidOakTreeAnchors {
	fn default() -> Self {
		Self::new(BraidOakTreeProtoAnchors::default())
	}
}

impl Anchors<StorybookTreeChain> for BraidOakTreeAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

impl Anchors<StorybookTreeChain> for BraidOakTreeProtoAnchors {
	fn anchors(&self) -> Vec<StorybookTreeChain> {
		self.hysteresis_seeds(NoiseConfig::new(NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;

	use crate::anchors::strict_stalk::StrictStalk;

	#[test]
	fn vertical_bias_droops_low_rises_high() {
		let radial = Vec3::new(1.0, 0.0, 0.0);
		let low = braid_vertical_bias_radial(radial, 0.0);
		let high = braid_vertical_bias_radial(radial, 1.0);
		assert!(low.y < 0.0, "low y {}", low.y);
		assert!(high.y > 0.0, "high y {}", high.y);
	}

	#[test]
	fn projection_length_peaks_near_mid_canopy() {
		let a = BraidOakTreeProtoAnchors::default();
		let l_low = a.projection_length(a.ring_mix_u(a.first_ring_unit_height));
		let l_high = a.projection_length(a.ring_mix_u(a.last_ring_unit_height));
		let l_mid = a.projection_length(0.5);
		assert!(l_mid > l_low);
		assert!(l_mid > l_high);
	}

	#[test]
	fn ring_count_in_expected_band() {
		let a = BraidOakTreeProtoAnchors::default();
		let n = a.ring_height_fractions().len();
		assert!((4..=8).contains(&n), "ring count {n}");
	}

	#[test]
	fn segmented_stalk_builds_multiple_hops() {
		let stalk = StrictStalk { stalk_height: 10.0, stalk_base_radius: 0.5 };
		let seed = stalk.segmented_point_to_point(BRAID_STALK_SECTION_COUNT);
		let chain = crate::BallStickChain::build(vec![seed]);
		assert_eq!(chain.nodes.len(), BRAID_STALK_SECTION_COUNT as usize + 1);
	}

	#[test]
	fn anchors_count_within_sampled_spoke_range() {
		let proto =
			BraidOakTreeProtoAnchors { ring_spacing_unit_height: 0.20, ..Default::default() };
		let ring_count = proto.ring_height_fractions().len();
		let a = BraidOakTreeAnchors::new(proto);
		let n = a.anchors().len();
		let min_anchors = ring_count * BRAID_ANCHORS_PER_RING_MIN as usize + 1;
		let max_anchors = ring_count * BRAID_ANCHORS_PER_RING_MAX as usize + 1;
		assert!(
			(n >= min_anchors) && (n <= max_anchors),
			"anchor count {n} not in [{min_anchors}, {max_anchors}]"
		);
	}
}
