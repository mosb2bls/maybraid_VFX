use std::f32::consts::{FRAC_PI_2, FRAC_PI_4};

use character_rigs::humanoid::LegSegmentLengths;

use crate::Progress;

const ROOT_SQUAT_DEG: f32 = 15.0;

#[derive(Debug, Clone)]
pub struct Squat {
	/// Stand-to-bottom rate: full descent takes `1/descent_speed` seconds.
	pub descent_speed: f32,
	/// Bottom-to-stand rate: full ascent takes `1/ascent_speed` seconds.
	pub ascent_speed: f32,
	/// When true, progress clamps to one down-up cycle; when false, it wraps.
	pub one_shot: bool,
	/// Held stance: `progress` is depth (0 = stand, 1 = bottom), not a cycle.
	pub held: bool,
	/// Skip [`crate::Effects::move`] so physics can own the origin snap.
	pub bones_only: bool,
	/// Peak femur forward swing at full depth (radians).
	pub femur_peak: f32,
	/// Peak shin flex relative to femur at full depth (radians).
	pub shin_peak: f32,
	/// Peak root forward swing at full depth (radians).
	pub root_peak: f32,
	/// Held-stance pelvis fold (radians). Jump windup leaves this at 0.
	pub hip_peak: f32,
}

impl Squat {
	/// Looping squat with independent descent and ascent half-cycle speeds.
	pub fn for_loop(descent_speed: f32, ascent_speed: f32) -> Self {
		Self { descent_speed, ascent_speed, one_shot: false, ..Self::default() }
	}

	/// One-shot down-up envelope with the given half-cycle speeds.
	pub fn with_speeds(descent_speed: f32, ascent_speed: f32) -> Self {
		Self { descent_speed, ascent_speed, one_shot: true, ..Self::default() }
	}

	/// Held stance pose. `progress` is squat depth; no armature `move`.
	pub fn held() -> Self {
		Self {
			held: true,
			bones_only: true,
			femur_peak: -1.35,
			shin_peak: 1.85,
			root_peak: 35.0_f32.to_radians(),
			hip_peak: 0.85,
			..Self::default()
		}
	}

	/// Duration of one full down-up cycle in seconds.
	pub fn cycle_duration(&self) -> f32 {
		self.descent_duration() + self.ascent_duration()
	}

	/// Duration of the stand-to-bottom half in seconds.
	pub fn descent_duration(&self) -> f32 {
		(1.0 / self.descent_speed).max(f32::EPSILON)
	}

	/// Duration of the bottom-to-stand half in seconds.
	pub fn ascent_duration(&self) -> f32 {
		(1.0 / self.ascent_speed).max(f32::EPSILON)
	}

	/// Normalized position in `[0, 1)` within one full down-up cycle at `progress`.
	pub fn cycle_phase(&self, progress: f32) -> f32 {
		let cycle = self.cycle_duration();
		if cycle <= f32::EPSILON {
			return 0.0;
		}
		(self.envelope_time(progress) / cycle).fract()
	}

	/// Vertical drop at full depth (`depth` = 1).
	pub fn peak_vertical_drop(&self, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_peak, self.shin_peak, lengths)
	}

	fn envelope_time(&self, progress: f32) -> f32 {
		let cycle = self.cycle_duration();
		let t = if self.one_shot {
			Progress(progress).clamp() * cycle
		} else {
			Progress(progress).cycle() * cycle
		};
		if self.one_shot && Progress(progress).is_complete() {
			return cycle;
		}
		t
	}

	/// Squat depth: 0 at stand, 1 at deepest flex.
	pub fn depth(&self, progress: f32) -> f32 {
		if self.held {
			return progress.clamp(0.0, 1.0);
		}
		let desc_d = self.descent_duration();
		let asc_d = self.ascent_duration();
		let t = self.envelope_time(progress);
		if self.one_shot && Progress(progress).is_complete() {
			return 0.0;
		}
		if t <= desc_d {
			let u = (t / desc_d).clamp(0.0, 1.0);
			(u * FRAC_PI_2).sin()
		} else {
			let u = ((t - desc_d) / asc_d).clamp(0.0, 1.0);
			(u * FRAC_PI_2).cos()
		}
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.depth(progress) * self.femur_peak
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.depth(progress) * self.shin_peak
	}

	pub fn root_swing(&self, progress: f32) -> f32 {
		self.depth(progress) * self.root_peak
	}

	pub fn hip_fold(&self, progress: f32) -> f32 {
		self.depth(progress) * self.hip_peak
	}

	pub fn vertical_drop(&self, progress: f32, lengths: LegSegmentLengths) -> f32 {
		vertical_drop(self.femur_swing(progress), self.shin_flex(progress), lengths)
	}
}

impl Default for Squat {
	fn default() -> Self {
		Self {
			descent_speed: 1.0,
			ascent_speed: 1.0,
			one_shot: false,
			held: false,
			bones_only: false,
			femur_peak: -FRAC_PI_4,
			shin_peak: FRAC_PI_2,
			root_peak: ROOT_SQUAT_DEG.to_radians(),
			hip_peak: 0.0,
		}
	}
}

/// Two-link leg height loss from hip to ankle in the sagittal plane.
pub fn vertical_drop(femur_swing: f32, shin_flex: f32, lengths: LegSegmentLengths) -> f32 {
	let standing = lengths.femur + lengths.shin;
	let bent = lengths.femur * femur_swing.cos() + lengths.shin * (femur_swing + shin_flex).cos();
	(standing - bent).max(0.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn independent_half_speeds_stretch_descent() -> anyhow::Result<()> {
		let slow = Squat::for_loop(0.25, 1.0);
		let fast = Squat::for_loop(1.0, 1.0);
		assert!(slow.depth(0.5) < fast.depth(0.5));
		Ok(())
	}

	#[test]
	fn segment_starts_and_ends_at_stand() -> anyhow::Result<()> {
		let squat = Squat::with_speeds(0.5, 0.5);
		assert!(squat.depth(0.0).abs() < 1e-5);
		assert!(squat.depth(1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn segment_peaks_at_end_of_descent() -> anyhow::Result<()> {
		let squat = Squat::with_speeds(1.0, 1.0);
		let peak_progress = squat.descent_duration() / squat.cycle_duration();
		assert!((squat.depth(peak_progress) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn stand_phase_has_zero_drop() {
		let squat = Squat::for_loop(1.0, 1.0);
		assert_eq!(squat.vertical_drop(0.0, LegSegmentLengths::default()), 0.0);
	}

	#[test]
	fn deepest_squat_has_positive_drop() {
		let squat = Squat::for_loop(1.0, 1.0);
		assert!(squat.vertical_drop(0.5, LegSegmentLengths::default()) > 0.0);
	}

	#[test]
	fn held_depth_is_progress() -> anyhow::Result<()> {
		let squat = Squat::held();
		assert!((squat.depth(0.0)).abs() < 1e-6);
		assert!((squat.depth(1.0) - 1.0).abs() < 1e-6);
		Ok(())
	}

	#[test]
	fn held_squat_folds_harder_than_jump_windup() -> anyhow::Result<()> {
		let held = Squat::held();
		let jump = Squat::default();
		assert!(held.femur_peak.abs() > jump.femur_peak.abs());
		assert!(held.hip_peak > jump.hip_peak);
		Ok(())
	}

	#[test]
	fn doubling_segment_lengths_doubles_drop() {
		let squat = Squat::for_loop(1.0, 1.0);
		let unit = LegSegmentLengths { femur: 0.5, shin: 0.5 };
		let doubled = LegSegmentLengths { femur: 1.0, shin: 1.0 };
		assert!(
			(squat.vertical_drop(0.5, doubled) - squat.vertical_drop(0.5, unit) * 2.0).abs() < 1e-4
		);
	}
}
