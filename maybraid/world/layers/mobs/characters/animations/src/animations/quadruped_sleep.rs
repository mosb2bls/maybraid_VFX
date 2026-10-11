//! Epiphant sleep entry, held sleep, and wake.
//!
//! Ground-impact gameplay should use [`EpiphantSleepEnter::GROUND_IMPACT_NORM`] and
//! [`EpiphantWake::GROUND_IMPACT_NORM`], not pose sampling.

use std::f32::consts::TAU;

use crate::animations::smoothstep;
use crate::Progress;

/// Belly-down settle at the end of sleep entry.
pub const SLEEP_ENTER_IMPACT_NORM: f32 = 0.88;
/// Front pair takes weight early in wake.
pub const WAKE_IMPACT_NORM: f32 = 0.18;

/// End of the sleep-entry settle segment (after [`SLEEP_ENTER_IMPACT_NORM`]).
pub const SETTLE_END: f32 = 0.92;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpiphantSleepEnter {
	pub fold: f32,
	pub lower: f32,
}

impl Default for EpiphantSleepEnter {
	fn default() -> Self {
		Self { fold: 0.55, lower: 0.42 }
	}
}

impl EpiphantSleepEnter {
	pub fn fold_weight(progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn leg_hinge(self, progress: f32) -> f32 {
		self.fold * Self::fold_weight(progress) * 1.35
	}

	pub fn leg_stride(self, progress: f32) -> f32 {
		-self.fold * 0.35 * Self::fold_weight(progress)
	}

	pub fn spine_lower(self, progress: f32) -> f32 {
		-self.lower * Self::fold_weight(progress)
	}

	pub fn neck_down(self, progress: f32) -> f32 {
		-self.lower * 0.65 * Self::fold_weight(progress)
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpiphantSleepHold {
	pub breathe: f32,
}

impl EpiphantSleepHold {
	pub fn breathe_phase(progress: f32) -> f32 {
		(TAU * Progress(progress).cycle()).sin()
	}

	pub fn spine_breathe(self, progress: f32) -> f32 {
		self.breathe * Self::breathe_phase(progress) * 0.04
	}

	pub fn neck_breathe(self, progress: f32) -> f32 {
		self.breathe * Self::breathe_phase(progress) * 0.02
	}
}

impl Default for EpiphantSleepHold {
	fn default() -> Self {
		Self { breathe: 1.0 }
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EpiphantWake {
	pub rise: f32,
}

impl Default for EpiphantWake {
	fn default() -> Self {
		Self { rise: 1.0 }
	}
}

impl EpiphantWake {
	pub fn rise_weight(progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	pub fn leg_hinge(self, progress: f32) -> f32 {
		let u = Self::rise_weight(progress);
		EpiphantSleepEnter::default().leg_hinge(1.0) * (1.0 - u)
	}

	pub fn leg_stride(self, progress: f32) -> f32 {
		let u = Self::rise_weight(progress);
		EpiphantSleepEnter::default().leg_stride(1.0) * (1.0 - u)
			+ self.rise * 0.22 * smoothstep((progress - WAKE_IMPACT_NORM).max(0.0) / 0.35)
	}

	pub fn spine_lift(self, progress: f32) -> f32 {
		let u = Self::rise_weight(progress);
		-EpiphantSleepEnter::default().spine_lower(1.0) * u
	}

	pub fn neck_lift(self, progress: f32) -> f32 {
		let u = Self::rise_weight(progress);
		-EpiphantSleepEnter::default().neck_down(1.0) * u
	}

	pub fn front_plant_weight(progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		if t < WAKE_IMPACT_NORM - 0.05 {
			return 0.0;
		}
		let u = ((t - (WAKE_IMPACT_NORM - 0.05)) / 0.12).clamp(0.0, 1.0);
		1.0 - (1.0 - u).powi(2)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sleep_enter_impact_follows_settle() {
		assert!(EpiphantSleepEnter::fold_weight(SLEEP_ENTER_IMPACT_NORM) > 0.9);
		assert!(SLEEP_ENTER_IMPACT_NORM <= SETTLE_END);
		assert!(EpiphantSleepEnter::fold_weight(SETTLE_END) > EpiphantSleepEnter::fold_weight(0.5));
	}

	#[test]
	fn wake_impact_aligns_with_front_stride() {
		let wake = EpiphantWake::default();
		assert!(EpiphantWake::front_plant_weight(WAKE_IMPACT_NORM) > 0.5);
		assert!(wake.leg_stride(WAKE_IMPACT_NORM) > wake.leg_stride(0.0));
	}
}
