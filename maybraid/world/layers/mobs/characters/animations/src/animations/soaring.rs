//! Glide pose with occasional wing-flap bursts.
//!
//! `progress` is wall-clock seconds. Bursts are parameterized by flap speed, stroke
//! range, and the pause between bursts.

use std::f32::consts::TAU;

use crate::animations::burst_envelope;

/// Softer handoff than idle scratch bursts (`DEFAULT_BURST_EDGE`).
const SOARING_BURST_EDGE: f32 = 0.2;

/// Held wing-spread with infrequent corrective flaps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Soaring {
	/// Flaps per second during a burst.
	pub flap_speed: f32,
	/// Stroke amplitude scale during a burst (`1.0` = default art range).
	pub flap_range: f32,
	/// Seconds of quiet glide between flap bursts.
	pub pause: f32,
	/// How many flap cycles to play in each burst.
	pub flaps_per_burst: f32,
}

impl Default for Soaring {
	fn default() -> Self {
		Self { flap_speed: 2.0, flap_range: 0.7, pause: 2.75, flaps_per_burst: 2.0 }
	}
}

impl Soaring {
	pub fn with_flap_speed(mut self, flap_speed: f32) -> Self {
		self.flap_speed = flap_speed;
		self
	}

	pub fn with_flap_range(mut self, flap_range: f32) -> Self {
		self.flap_range = flap_range;
		self
	}

	pub fn with_pause(mut self, pause: f32) -> Self {
		self.pause = pause;
		self
	}

	pub fn with_flaps_per_burst(mut self, flaps_per_burst: f32) -> Self {
		self.flaps_per_burst = flaps_per_burst.max(0.0);
		self
	}

	/// Duration of one flap burst in seconds.
	pub fn burst_duration(&self) -> f32 {
		let speed = self.flap_speed.max(f32::EPSILON);
		(self.flaps_per_burst / speed).max(f32::EPSILON)
	}

	/// Full soar cycle: burst then pause.
	pub fn cycle_duration(&self) -> f32 {
		self.burst_duration() + self.pause.max(0.0)
	}

	/// Signed flap contribution; `0` while gliding between bursts.
	pub fn flap_amount(&self, progress: f32) -> f32 {
		let cycle = self.cycle_duration();
		if cycle <= f32::EPSILON {
			return 0.0;
		}
		let t = progress.rem_euclid(cycle);
		let burst = self.burst_duration();
		if t >= burst {
			return 0.0;
		}

		let u = (t / burst).clamp(0.0, 1.0);
		let envelope = burst_envelope(u, SOARING_BURST_EDGE);
		let local = t * self.flap_speed.max(f32::EPSILON);
		envelope * (TAU * local).sin() * self.flap_range
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn soaring_is_quiet_during_pause() -> anyhow::Result<()> {
		let soar = Soaring { flap_speed: 2.0, flap_range: 1.0, pause: 3.0, flaps_per_burst: 2.0 };
		let burst = soar.burst_duration();
		assert!(soar.flap_amount(burst + 0.5).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn soaring_flaps_during_burst() -> anyhow::Result<()> {
		let soar = Soaring::default();
		// Quarter-cycle into the first flap (peak of the sine).
		let peak = 0.25 / soar.flap_speed.max(f32::EPSILON);
		assert!(soar.flap_amount(peak).abs() > 0.05);
		Ok(())
	}

	#[test]
	fn burst_envelope_peaks_in_middle() -> anyhow::Result<()> {
		assert!(burst_envelope(0.0, SOARING_BURST_EDGE) < 0.05);
		assert!((burst_envelope(0.5, SOARING_BURST_EDGE) - 1.0).abs() < 1e-5);
		assert!(burst_envelope(1.0, SOARING_BURST_EDGE) < 0.05);
		Ok(())
	}
}
