//! Low sideways power slide: wide crouch, lead hand skims the ground, recover.
//!
//! [`PowerStrafe::side`] is the travel direction (left strafe → [`Side::Left`]).
//! Humanoid rigs mirror lateral offsets with [`Side::sign`]; do not hand-author
//! separate left/right pose tables.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// Drop into stance through the first slice of normalized progress.
pub const DROP_END: f32 = 0.16;
/// Slide holds through this point; recovery is the remainder.
pub const SLIDE_END: f32 = 0.80;

const TRAVEL_FEMUR: f32 = 0.58;
const TRAIL_FEMUR: f32 = 0.42;
const STANCE_SHIN: f32 = 0.72;
const TRAIL_SHIN: f32 = 0.62;
const HIP_ABDUCT_TRAVEL: f32 = 0.28;
const HIP_ABDUCT_TRAIL: f32 = -0.12;
const SPINE_PITCH: f32 = 0.22;
const SPINE_SIDE: f32 = 0.10;
const HAND_ELBOW: f32 = 1.35;
const BALANCE_ELBOW: f32 = 0.55;
const HAND_ROLL: f32 = FRAC_PI_2;

/// Lateral armature slide at full extension (character +X = right).
pub const SLIDE_OFFSET_X: f32 = 0.11;
/// Visual crouch via armature Y at peak stance.
pub const STANCE_DROP_Y: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerStrafe {
	pub side: Side,
}

impl Default for PowerStrafe {
	fn default() -> Self {
		Self { side: Side::Right }
	}
}

impl PowerStrafe {
	pub fn new(side: Side) -> Self {
		Self { side }
	}

	pub fn with_side(mut self, side: Side) -> Self {
		self.side = side;
		self
	}

	pub fn travel_side(self) -> Side {
		self.side
	}

	pub fn opposite_side(self) -> Side {
		self.side.opposite()
	}

	pub fn clamped(progress: f32) -> f32 {
		Progress(progress).clamp()
	}

	/// 0 at rest, 1 through the slide window.
	pub fn stance_amount(&self, progress: f32) -> f32 {
		let t = Self::clamped(progress);
		if t < DROP_END {
			smoothstep(t / DROP_END)
		} else if t < SLIDE_END {
			1.0
		} else {
			1.0 - smoothstep((t - SLIDE_END) / (1.0 - SLIDE_END).max(f32::EPSILON))
		}
	}

	/// Lateral slide envelope (0 outside the slide core).
	pub fn slide_amount(&self, progress: f32) -> f32 {
		let t = Self::clamped(progress);
		if t < DROP_END {
			return 0.0;
		}
		if t < SLIDE_END {
			let u = (t - DROP_END) / (SLIDE_END - DROP_END);
			return smoothstep(u);
		}
		1.0 - smoothstep((t - SLIDE_END) / (1.0 - SLIDE_END).max(f32::EPSILON))
	}

	pub fn travel_femur(&self, progress: f32) -> f32 {
		TRAVEL_FEMUR * self.stance_amount(progress)
	}

	pub fn trail_femur(&self, progress: f32) -> f32 {
		TRAIL_FEMUR * self.stance_amount(progress)
	}

	pub fn travel_shin(&self, progress: f32) -> f32 {
		STANCE_SHIN * self.stance_amount(progress)
	}

	pub fn trail_shin(&self, progress: f32) -> f32 {
		TRAIL_SHIN * self.stance_amount(progress)
	}

	pub fn travel_abduction(&self, progress: f32) -> f32 {
		HIP_ABDUCT_TRAVEL * self.stance_amount(progress)
	}

	pub fn trail_abduction(&self, progress: f32) -> f32 {
		HIP_ABDUCT_TRAIL * self.stance_amount(progress)
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		SPINE_PITCH * self.stance_amount(progress)
	}

	pub fn spine_side(&self, progress: f32) -> f32 {
		SPINE_SIDE * self.stance_amount(progress) * self.side.sign()
	}

	pub fn hand_elbow(&self, progress: f32) -> f32 {
		HAND_ELBOW * self.stance_amount(progress)
	}

	pub fn balance_elbow(&self, progress: f32) -> f32 {
		BALANCE_ELBOW * self.stance_amount(progress)
	}

	/// Character-space reach for the ground-skimming hand.
	pub fn hand_along(&self, progress: f32) -> Vec3 {
		let reach = self.stance_amount(progress);
		if reach < 1e-4 {
			return Vec3::Y;
		}
		Vec3::new(-self.side.sign() * 0.42, -0.78, 0.18).normalize()
	}

	pub fn hand_roll(&self, progress: f32) -> f32 {
		HAND_ROLL * self.side.sign() * self.stance_amount(progress)
	}

	pub fn armature_translation(&self, progress: f32) -> Vec3 {
		let slide = self.slide_amount(progress);
		let drop = self.stance_amount(progress);
		Vec3::new(-SLIDE_OFFSET_X * slide * self.side.sign(), -STANCE_DROP_Y * drop, 0.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn phase_windows_are_ordered() {
		assert!(DROP_END > 0.0);
		assert!(SLIDE_END > DROP_END);
		assert!(SLIDE_END < 1.0);
	}

	#[test]
	fn stance_envelope_peaks_mid_clip() {
		let strafe = PowerStrafe::default();
		assert!(strafe.stance_amount(0.0) < 0.05);
		assert!(strafe.stance_amount(0.5) > 0.95);
		assert!(strafe.stance_amount(1.0) < 0.05);
	}
}
