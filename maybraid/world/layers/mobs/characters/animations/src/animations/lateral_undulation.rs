//! Lateral (side-to-side) caudal undulation — typical teleost swimming.

use std::f32::consts::TAU;

/// Traveling lateral wave along the post-cranial axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LateralUndulation {
	/// Wave cycles per second when `progress` is elapsed time.
	pub speed: f32,
	/// Peak yaw amplitude at the caudal tip (radians scale).
	pub amplitude: f32,
}

impl Default for LateralUndulation {
	fn default() -> Self {
		Self { speed: 1.4, amplitude: 1.0 }
	}
}

impl LateralUndulation {
	pub fn with_speed(mut self, speed: f32) -> Self {
		self.speed = speed;
		self
	}

	pub fn with_amplitude(mut self, amplitude: f32) -> Self {
		self.amplitude = amplitude;
		self
	}

	/// Phase in cycles for a traveling wave at time `progress`.
	pub fn wave_phase(&self, progress: f32) -> f32 {
		progress * self.speed.max(f32::EPSILON)
	}

	/// Signed yaw contribution for caudal segment `index` (0 = cranial … n = tip).
	pub fn segment_yaw(&self, progress: f32, index: usize, segment_count: usize) -> f32 {
		let count = segment_count.max(1) as f32;
		let lag = index as f32 / count;
		let phase = self.wave_phase(progress) - lag;
		let envelope = 0.35 + 0.65 * lag;
		(TAU * phase).sin() * self.amplitude * envelope
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lateral_wave_grows_toward_tail() -> anyhow::Result<()> {
		let swim = LateralUndulation::default();
		let cranial = swim.segment_yaw(0.1, 0, 4).abs();
		let caudal = swim.segment_yaw(0.1, 3, 4).abs();
		assert!(caudal + 1e-4 >= cranial);
		Ok(())
	}
}
