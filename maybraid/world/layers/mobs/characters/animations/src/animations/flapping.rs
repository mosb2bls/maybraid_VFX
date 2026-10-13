//! Continuous wing-flap cycle for biped / bird humanoids.
//!
//! `progress` is wall-clock seconds; [`Flapping::speed`] sets flaps per second and
//! [`Flapping::range`] scales stroke amplitude around a held soar-spread pose.

use std::f32::consts::TAU;

/// Repeating in-phase wing flap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Flapping {
	/// Flaps per second when `progress` is elapsed time in seconds.
	pub speed: f32,
	/// Stroke amplitude scale (`1.0` = default art range).
	pub range: f32,
}

impl Default for Flapping {
	fn default() -> Self {
		Self { speed: 2.5, range: 1.0 }
	}
}

impl Flapping {
	pub fn with_speed(mut self, speed: f32) -> Self {
		self.speed = speed;
		self
	}

	pub fn with_range(mut self, range: f32) -> Self {
		self.range = range;
		self
	}

	/// Signed flap amount in roughly `[-range, range]` (downstroke negative).
	pub fn flap_amount(&self, progress: f32) -> f32 {
		let speed = self.speed.max(f32::EPSILON);
		(TAU * progress * speed).sin() * self.range
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flapping_amount_scales_with_range() -> anyhow::Result<()> {
		let base = Flapping { speed: 1.0, range: 1.0 }.flap_amount(0.25);
		let wide = Flapping { speed: 1.0, range: 2.0 }.flap_amount(0.25);
		assert!((wide - 2.0 * base).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn flapping_is_periodic_in_speed() -> anyhow::Result<()> {
		let flap = Flapping { speed: 2.0, range: 1.0 };
		let a = flap.flap_amount(0.1);
		let b = flap.flap_amount(0.1 + 0.5);
		assert!((a - b).abs() < 1e-4);
		Ok(())
	}
}
