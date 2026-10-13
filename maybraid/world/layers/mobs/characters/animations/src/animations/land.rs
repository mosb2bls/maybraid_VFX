use character_rigs::humanoid::LegSegmentLengths;

use crate::animations::{vertical_drop, Squat};

/// Peak landing flex as a fraction of a full pre-jump squat.
const DEFAULT_LAND_SCALE: f32 = 0.35;

#[derive(Debug, Clone)]
pub struct Land {
	/// Timed squat envelope for landing compression and recovery.
	pub squat: Squat,
	/// Scales peak landing joint flex relative to a full squat.
	pub scale: f32,
}

impl Land {
	/// Landing squat with explicit compression and recovery half-cycle speeds.
	pub fn with_speeds(descent_speed: f32, ascent_speed: f32, squat: Squat) -> Self {
		let mut timed = Squat::with_speeds(descent_speed, ascent_speed);
		timed.femur_peak = squat.femur_peak;
		timed.shin_peak = squat.shin_peak;
		timed.root_peak = squat.root_peak;
		Self { squat: timed, scale: DEFAULT_LAND_SCALE }
	}

	/// Normalized landing depth before scale: 0 at touch-down extension, 1 at peak compression.
	pub fn depth(&self, progress: f32) -> f32 {
		self.squat.depth(progress)
	}

	/// Duration of impact compression (touch-down to peak flex) in seconds.
	pub fn descent_duration(&self) -> f32 {
		self.squat.descent_duration()
	}

	/// Duration of stand-up recovery after peak compression in seconds.
	pub fn ascent_duration(&self) -> f32 {
		self.squat.ascent_duration()
	}

	pub fn cycle_duration(&self) -> f32 {
		self.squat.cycle_duration()
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.depth(progress) * self.scale * self.squat.femur_peak
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.depth(progress) * self.scale * self.squat.shin_peak
	}

	pub fn root_swing(&self, progress: f32) -> f32 {
		self.depth(progress) * self.scale * self.squat.root_peak
	}

	pub fn vertical_drop(&self, progress: f32, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_swing(progress), self.shin_flex(progress), lengths)
	}

	/// Vertical drop at peak landing compression.
	pub fn peak_vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(
			self.scale * self.squat.femur_peak,
			self.scale * self.squat.shin_peak,
			lengths,
		)
	}
}

impl Default for Land {
	fn default() -> Self {
		Self::with_speeds(1.0, 1.0, Squat::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::Squat;

	#[test]
	fn land_peak_flex_below_full_squat() -> anyhow::Result<()> {
		let squat = Squat::for_loop(1.0, 1.0);
		let land = Land::default();
		let peak = land.descent_duration() / land.cycle_duration();
		assert!(land.femur_swing(peak).abs() < squat.femur_swing(0.5).abs());
		assert!(land.shin_flex(peak).abs() < squat.shin_flex(0.5).abs());
		Ok(())
	}

	#[test]
	fn land_starts_at_stand() -> anyhow::Result<()> {
		let land = Land::default();
		assert!(land.depth(0.0).abs() < 1e-5);
		assert!(land.femur_swing(0.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn land_compresses_gradually_after_touchdown() -> anyhow::Result<()> {
		let land = Land::with_speeds(10.0, 1.0, Squat::default());
		let early = land.descent_duration() * 0.5 / land.cycle_duration();
		assert!(land.depth(early) > 0.0);
		Ok(())
	}
}
