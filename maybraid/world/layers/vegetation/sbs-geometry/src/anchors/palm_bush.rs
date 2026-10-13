//! **Palm Bush** ground-anchored frond crown ([#231](https://github.com/ramate-io/maybraid/issues/231), [RFC §3.1.7.10](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/07-well-known-tree-constructions/10-palm-bush/README.md)).
//!
//! Trunkless: stacked frond rings only; no ball-stick hysteresis graph.

use bevy_math::Vec3;

use crate::palm_crown::{ring_spacing_world, vertical_bias_mix};

/// Default total height `H` for playground previews.
pub const DEFAULT_HEIGHT: f32 = 8.0;

/// Crown anchor lift above ground as a fraction of `H` (RFC `0.02`).
pub const DEFAULT_CROWN_LIFT_FRACTION: f32 = 0.02;

/// RFC ring count band `6..=10`.
pub const DEFAULT_RING_COUNT: u32 = 8;

/// RFC fronds per ring band `10..=16`.
pub const DEFAULT_FRONDS_PER_RING: u32 = 12;

/// Vertical spacing between rings as a fraction of `H` (RFC `0.01`).
pub const DEFAULT_RING_SPACING_FRACTION: f32 = 0.01;

/// Lower-ring vertical bias in direction mix (RFC `-0.20`).
pub const DEFAULT_VERTICAL_BIAS_LOW: f32 = -0.20;

/// Upper-ring vertical bias (RFC `0.35`).
pub const DEFAULT_VERTICAL_BIAS_HIGH: f32 = 0.35;

/// RFC optional tuft scale as a fraction of `H`.
pub const DEFAULT_CROWN_TUFT_SCALE_FRACTION: f32 = 0.2;

/// Trunkless palm-bush crown parameters.
#[derive(Clone, Debug, PartialEq)]
pub struct PalmBushProtoAnchors {
	pub height: f32,
	pub crown_lift_fraction: f32,
	pub ring_count: u32,
	pub fronds_per_ring: u32,
	pub ring_spacing_fraction: f32,
	pub vertical_bias_low: f32,
	pub vertical_bias_high: f32,
	pub crown_tuft_scale_fraction: f32,
}

impl Default for PalmBushProtoAnchors {
	fn default() -> Self {
		Self {
			height: DEFAULT_HEIGHT,
			crown_lift_fraction: DEFAULT_CROWN_LIFT_FRACTION,
			ring_count: DEFAULT_RING_COUNT,
			fronds_per_ring: DEFAULT_FRONDS_PER_RING,
			ring_spacing_fraction: DEFAULT_RING_SPACING_FRACTION,
			vertical_bias_low: DEFAULT_VERTICAL_BIAS_LOW,
			vertical_bias_high: DEFAULT_VERTICAL_BIAS_HIGH,
			crown_tuft_scale_fraction: DEFAULT_CROWN_TUFT_SCALE_FRACTION,
		}
	}
}

impl PalmBushProtoAnchors {
	/// Tree-local crown origin (pure Y lift above the origin; the root entity owns world placement).
	pub fn crown_origin(&self) -> Vec3 {
		let h = self.height.max(1e-6);
		Vec3::Y * (h * self.crown_lift_fraction)
	}

	pub fn ring_spacing(&self) -> f32 {
		ring_spacing_world(self.height, self.ring_spacing_fraction)
	}

	/// RFC `mix(low, high, u)` for ring index `ring` in `0..ring_count`.
	pub fn ring_vertical_bias(&self, ring: u32) -> f32 {
		vertical_bias_mix(ring, self.ring_count, self.vertical_bias_low, self.vertical_bias_high)
	}

	/// Ground-up stacked ring anchor (RFC palm bush).
	pub fn crown_ring_position(&self, ring: u32) -> Vec3 {
		self.crown_origin() + Vec3::Y * self.ring_spacing() * ring as f32
	}

	pub fn crown_tuft_world_scale(&self) -> f32 {
		self.height.max(1e-6) * self.crown_tuft_scale_fraction
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_counts_in_rfc_bands() {
		let proto = PalmBushProtoAnchors::default();
		assert!((6..=10).contains(&proto.ring_count));
		assert!((10..=16).contains(&proto.fronds_per_ring));
	}

	#[test]
	fn crown_ring_positions_increase_with_height() -> anyhow::Result<()> {
		let proto = PalmBushProtoAnchors::default();
		let mut prev_y = proto.crown_origin().y;
		for ring in 0..proto.ring_count {
			let pos = proto.crown_ring_position(ring);
			assert!(pos.y >= prev_y - 1e-5, "ring {ring} y {}", pos.y);
			prev_y = pos.y;
		}
		Ok(())
	}

	#[test]
	fn ring_vertical_bias_endpoints_match_rfc() {
		let proto = PalmBushProtoAnchors::default();
		assert!((proto.ring_vertical_bias(0) - DEFAULT_VERTICAL_BIAS_LOW).abs() < 1e-5);
		let last = proto.ring_count.saturating_sub(1);
		assert!((proto.ring_vertical_bias(last) - DEFAULT_VERTICAL_BIAS_HIGH).abs() < 1e-5);
	}
}
