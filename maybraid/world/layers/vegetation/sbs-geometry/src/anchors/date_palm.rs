//! **Date Palm** trunk anchor ([#256](https://github.com/ramate-io/maybraid/issues/256), [RFC §3.1.7.9](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/09-date-palm/README.md)).
//!
//! One vertical trunk seed at the ground; crown fronds are placed at render time from [`crate::sbs::date_palm::DatePalmSbs`].

use bevy_math::Vec3;
use procedural_common::NoiseConfig;

use super::strict_stalk::StrictStalk;
use super::Anchors;
use crate::chain::date_palm::{DatePalmChain, DatePalmPhase};
use crate::chain::BranchOut;
use crate::chain::DepthBudget;
use crate::palm_crown::vertical_bias_mix;
use crate::BallStickNode;

/// Default stalk height for playground date palms (shorter than tall RFC reference trees).
pub const DEFAULT_STALK_HEIGHT: f32 = 12.0;

/// Default trunk radius as a fraction of stalk height when not set on [`StrictStalk`].
pub const DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT: f32 = 0.10;

/// Trunk and crown layout before [`DatePalmAnchors::hysteresis_seeds`].
#[derive(Clone, Debug, PartialEq)]
pub struct DatePalmProtoAnchors {
	pub stalk: StrictStalk,
	/// Visible trunk height as a fraction of [`StrictStalk::stalk_height`] (short trunk, large crown).
	pub trunk_height_fraction: f32,
	/// Per-segment length as fractions of stalk height (RFC `0.05..0.08`).
	pub segment_length_fraction: (f32, f32),
	/// Vertical growth tolerance in radians (RFC ~2°).
	pub angle_tolerance_radians: f32,
	/// Stacked frond rings at the crown (RFC `6..10`; playground default denser).
	pub ring_count: u32,
	pub fronds_per_ring: u32,
	/// Vertical extent of the crown ring stack below the trunk tip, as a fraction of stalk height.
	pub crown_stack_down_fraction: f32,
	/// Lower-ring vertical bias in crown direction mix (RFC `-0.10`).
	pub crown_vertical_bias_low: f32,
	/// Upper-ring vertical bias (RFC `0.60`).
	pub crown_vertical_bias_high: f32,
}

impl Default for DatePalmProtoAnchors {
	fn default() -> Self {
		let h = DEFAULT_STALK_HEIGHT;
		Self {
			stalk: StrictStalk {
				stalk_height: h,
				stalk_base_radius: DEFAULT_TRUNK_RADIUS_FRACTION_OF_HEIGHT * h,
			},
			trunk_height_fraction: 0.68,
			segment_length_fraction: (0.05, 0.08),
			angle_tolerance_radians: 2.0_f32.to_radians(),
			ring_count: 10,
			fronds_per_ring: 15,
			crown_stack_down_fraction: 0.30,
			crown_vertical_bias_low: -0.10,
			crown_vertical_bias_high: 0.60,
		}
	}
}

impl DatePalmProtoAnchors {
	pub fn trunk_height(&self) -> f32 {
		self.stalk.stalk_height.max(1e-6) * self.trunk_height_fraction
	}

	pub fn trunk_segment_count(&self) -> usize {
		let h = self.stalk.stalk_height.max(1e-6);
		let (lo, hi) = self.segment_length_fraction;
		let mean = (lo + hi) * 0.5 * h;
		(self.trunk_height() / mean.max(1e-4)).ceil().max(1.0) as usize
	}

	pub fn ring_spacing(&self) -> f32 {
		let h = self.stalk.stalk_height.max(1e-6);
		let n = self.ring_count.max(1);
		if n <= 1 {
			return 0.0;
		}
		h * self.crown_stack_down_fraction / (n - 1) as f32
	}

	/// RFC `mix(low, high, u)` for ring index `ring` in `0..ring_count`.
	pub fn ring_vertical_bias(&self, ring: u32) -> f32 {
		vertical_bias_mix(
			ring,
			self.ring_count,
			self.crown_vertical_bias_low,
			self.crown_vertical_bias_high,
		)
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<DatePalmChain> {
		let h = self.stalk.stalk_height.max(1e-6);
		let (len_lo, len_hi) = self.segment_length_fraction;
		let r = self.stalk.stalk_base_radius;
		let base = BallStickNode::new(Vec3::ZERO, r);

		let branch = BranchOut::up(base)
			.with_hysteresis_context(chain_noise.clone(), 0, Vec3::Y)
			.with_bias_blend(1.0)
			.with_ray_degrees_of_freedom(self.angle_tolerance_radians)
			.with_radius_range(r..r)
			.with_radius_range_child_scale((1.0, 1.0))
			.with_length(h * len_lo..h * len_hi)
			.single_child();

		vec![DatePalmChain::new(
			chain_noise,
			DatePalmPhase::Trunk(DepthBudget {
				inner: branch,
				remaining: self.trunk_segment_count(),
			}),
		)]
	}
}

/// Anchor recipe for [`crate::sbs::date_palm::DatePalmSbs`].
#[derive(Clone, Debug, PartialEq)]
pub struct DatePalmAnchors {
	proto: DatePalmProtoAnchors,
}

impl DatePalmAnchors {
	pub fn new(proto: DatePalmProtoAnchors) -> Self {
		Self { proto }
	}

	pub fn proto(&self) -> &DatePalmProtoAnchors {
		&self.proto
	}

	pub fn hysteresis_seeds(&self, chain_noise: NoiseConfig) -> Vec<DatePalmChain> {
		self.proto.hysteresis_seeds(chain_noise)
	}
}

impl Anchors<DatePalmChain> for DatePalmAnchors {
	fn anchors(&self) -> Vec<DatePalmChain> {
		self.hysteresis_seeds(NoiseConfig::new(procedural_common::NoiseParams::default()))
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_trunk_segment_count_in_expected_band() {
		let proto = DatePalmProtoAnchors::default();
		let n = proto.trunk_segment_count();
		assert!((7..=14).contains(&n), "segment count {n}");
	}
}
