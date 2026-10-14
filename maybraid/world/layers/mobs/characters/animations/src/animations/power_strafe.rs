//! Cartoon football juke: fake opposite, plant, snap into a sideways cut.
//!
//! Beat: a short head / shoulder / hip lean away from travel, then a hard
//! outside-foot plant, an explosive cut, and the travel-side hand swept down
//! to skim the ground. The smear extreme holds, then overshoots and settles.
//!
//! [`PowerStrafe::side`] is the travel direction (left strafe → [`Side::Left`]).
//! Humanoid rigs mirror lateral offsets with [`Side::sign`]; do not hand-author
//! separate left/right pose tables.

use std::f32::consts::{FRAC_PI_2, PI};

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::smoothstep;
use crate::Progress;

/// Fake / anticipation finishes here; the plant and snap begin.
pub const DROP_END: f32 = 0.16;
/// Fake lean peaks near this progress (review sample `0.1`).
pub const FAKE_PEAK: f32 = 0.10;
/// Explosive cut reaches the smear pose.
pub const SNAP_END: f32 = 0.34;
/// Held smear-like extreme through this point.
pub const SMEAR_END: f32 = 0.58;
/// Overshoot crest; recovery is the remainder.
pub const SLIDE_END: f32 = 0.78;

const TRAVEL_FEMUR: f32 = 0.70;
const TRAIL_FEMUR_CUT: f32 = 0.44;
const TRAIL_FEMUR_FAKE: f32 = 0.28;
const STANCE_SHIN: f32 = 0.78;
const TRAIL_SHIN_CUT: f32 = 0.58;
const TRAIL_SHIN_FAKE: f32 = 0.32;
const HIP_ABDUCT_TRAVEL: f32 = 0.36;
const HIP_ABDUCT_TRAIL: f32 = -0.14;
const HIP_ABDUCT_FAKE: f32 = 0.12;
const SPINE_PITCH: f32 = 0.32;
const SPINE_SIDE_CUT: f32 = 0.26;
const SPINE_SIDE_FAKE: f32 = 0.22;
const NECK_TILT_FAKE: f32 = 0.36;
const NECK_TILT_CUT: f32 = 0.18;
const NECK_NOD_FAKE: f32 = 0.14;
const NECK_NOD_CUT: f32 = 0.08;
const HIP_LATERAL_FAKE: f32 = 0.16;
const HIP_LATERAL_CUT: f32 = 0.18;
const HAND_ELBOW: f32 = 0.22;
const BALANCE_ELBOW: f32 = 0.88;
const HAND_ROLL: f32 = FRAC_PI_2;
const SHOULDER_LIFT_CUT: f32 = 0.58;
const SHOULDER_LIFT_FAKE: f32 = 0.26;
const OVERSHOOT: f32 = 0.14;

/// Lateral armature slide at full extension (character +X = right).
pub const SLIDE_OFFSET_X: f32 = 0.12;
/// Visual crouch via armature Y at peak stance.
pub const STANCE_DROP_Y: f32 = 0.07;

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

	/// Fast attack into the smear (cubic ease-out).
	fn snap_ease(u: f32) -> f32 {
		let u = u.clamp(0.0, 1.0);
		1.0 - (1.0 - u).powi(3)
	}

	/// Opposite-side fake: 0 at rest, ~1 at [`FAKE_PEAK`], 0 once the snap starts.
	pub fn fake_amount(&self, progress: f32) -> f32 {
		let t = Self::clamped(progress);
		if t < FAKE_PEAK {
			smoothstep(t / FAKE_PEAK)
		} else if t < DROP_END {
			1.0 - smoothstep((t - FAKE_PEAK) / (DROP_END - FAKE_PEAK).max(f32::EPSILON))
		} else {
			0.0
		}
	}

	/// Cut / plant envelope. Overshoots past 1 in the smear tail, then settles.
	pub fn cut_amount(&self, progress: f32) -> f32 {
		let t = Self::clamped(progress);
		if t < DROP_END {
			0.0
		} else if t < SNAP_END {
			Self::snap_ease((t - DROP_END) / (SNAP_END - DROP_END).max(f32::EPSILON))
		} else if t < SMEAR_END {
			1.0
		} else if t < SLIDE_END {
			let u = (t - SMEAR_END) / (SLIDE_END - SMEAR_END).max(f32::EPSILON);
			1.0 + OVERSHOOT * (u * PI).sin()
		} else {
			let settle = 1.0 - smoothstep((t - SLIDE_END) / (1.0 - SLIDE_END).max(f32::EPSILON));
			settle
		}
	}

	/// 0 at rest, 1 through the smear window. Used for shared stance depth.
	pub fn stance_amount(&self, progress: f32) -> f32 {
		self.cut_amount(progress).abs().min(1.0)
	}

	/// Lateral slide envelope (0 during the fake, 0 again at the end).
	pub fn slide_amount(&self, progress: f32) -> f32 {
		let t = Self::clamped(progress);
		if t < DROP_END {
			return 0.0;
		}
		if t < SNAP_END {
			return smoothstep((t - DROP_END) / (SNAP_END - DROP_END).max(f32::EPSILON));
		}
		if t < SLIDE_END {
			return 1.0;
		}
		1.0 - smoothstep((t - SLIDE_END) / (1.0 - SLIDE_END).max(f32::EPSILON))
	}

	/// Travel-hand sweep, slightly behind the plant so the slam reads after the cut.
	pub fn hand_amount(&self, progress: f32) -> f32 {
		let t = Self::clamped(progress);
		if t >= 1.0 - 1e-4 {
			return 0.0;
		}
		let attack = if t < SNAP_END { (t - 0.03).max(0.0) } else { t };
		self.cut_amount(attack).clamp(0.0, 1.12)
	}

	pub fn travel_femur(&self, progress: f32) -> f32 {
		TRAVEL_FEMUR * self.stance_amount(progress)
	}

	pub fn trail_femur(&self, progress: f32) -> f32 {
		TRAIL_FEMUR_FAKE * self.fake_amount(progress)
			+ TRAIL_FEMUR_CUT * self.stance_amount(progress)
	}

	pub fn travel_shin(&self, progress: f32) -> f32 {
		STANCE_SHIN * self.stance_amount(progress)
	}

	pub fn trail_shin(&self, progress: f32) -> f32 {
		TRAIL_SHIN_FAKE * self.fake_amount(progress) + TRAIL_SHIN_CUT * self.stance_amount(progress)
	}

	pub fn travel_abduction(&self, progress: f32) -> f32 {
		HIP_ABDUCT_TRAVEL * self.stance_amount(progress)
	}

	pub fn trail_abduction(&self, progress: f32) -> f32 {
		HIP_ABDUCT_FAKE * self.fake_amount(progress)
			+ HIP_ABDUCT_TRAIL * self.stance_amount(progress)
	}

	pub fn spine_pitch(&self, progress: f32) -> f32 {
		0.10 * self.fake_amount(progress) + SPINE_PITCH * self.stance_amount(progress)
	}

	/// Lateral spine. Fake is opposite travel; cut leans into travel.
	pub fn spine_side(&self, progress: f32) -> f32 {
		let fake = -SPINE_SIDE_FAKE * self.fake_amount(progress);
		let cut = SPINE_SIDE_CUT * self.cut_amount(progress);
		(fake + cut) * self.side.sign()
	}

	pub fn neck_tilt(&self, progress: f32) -> f32 {
		let fake = -NECK_TILT_FAKE * self.fake_amount(progress);
		let cut = NECK_TILT_CUT * self.cut_amount(progress);
		(fake + cut) * self.side.sign()
	}

	pub fn neck_nod(&self, progress: f32) -> f32 {
		NECK_NOD_FAKE * self.fake_amount(progress) + NECK_NOD_CUT * self.stance_amount(progress)
	}

	pub fn hip_lateral(&self, progress: f32) -> f32 {
		let fake = -HIP_LATERAL_FAKE * self.fake_amount(progress);
		let cut = HIP_LATERAL_CUT * self.cut_amount(progress);
		(fake + cut) * self.side.sign()
	}

	pub fn hand_elbow(&self, progress: f32) -> f32 {
		HAND_ELBOW * self.hand_amount(progress).min(1.0)
	}

	pub fn balance_elbow(&self, progress: f32) -> f32 {
		BALANCE_ELBOW
			* (0.45 * self.fake_amount(progress) + 0.70 * self.stance_amount(progress)).min(1.0)
	}

	pub fn shoulder_lift_travel(&self, progress: f32) -> f32 {
		(SHOULDER_LIFT_FAKE * self.fake_amount(progress)
			+ SHOULDER_LIFT_CUT * self.hand_amount(progress))
			* self.side.sign()
	}

	pub fn shoulder_lift_trail(&self, progress: f32) -> f32 {
		(-SHOULDER_LIFT_FAKE * self.fake_amount(progress) + 0.22 * self.stance_amount(progress))
			* self.opposite_side().sign()
	}

	/// Character-space reach for the ground-skimming hand.
	pub fn hand_along(&self, progress: f32) -> Option<Vec3> {
		let reach = self.hand_amount(progress);
		if reach < 1e-4 {
			return None;
		}
		Some(Vec3::new(-self.side.sign() * 0.32, -0.93, 0.08).normalize())
	}

	pub fn hand_roll(&self, progress: f32) -> f32 {
		HAND_ROLL * self.side.sign() * self.hand_amount(progress).min(1.0)
	}

	/// Lateral slide in documented character space (+X right), matching Tanz
	/// `MoveWish` at look yaw 0 (`left_strafe` → −X, `right_strafe` → +X).
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
		assert!(FAKE_PEAK > 0.0);
		assert!(DROP_END > FAKE_PEAK);
		assert!(SNAP_END > DROP_END);
		assert!(SMEAR_END > SNAP_END);
		assert!(SLIDE_END > SMEAR_END);
		assert!(SLIDE_END < 1.0);
	}

	#[test]
	fn fake_peaks_before_the_cut() {
		let strafe = PowerStrafe::default();
		assert!(strafe.fake_amount(0.0) < 0.05);
		assert!(strafe.fake_amount(FAKE_PEAK) > 0.95);
		assert!(strafe.fake_amount(0.5) < 0.05);
		assert!(strafe.cut_amount(FAKE_PEAK) < 0.05);
		assert!(strafe.cut_amount(0.5) > 0.95);
	}

	#[test]
	fn stance_envelope_peaks_mid_clip() {
		let strafe = PowerStrafe::default();
		assert!(strafe.stance_amount(0.0) < 0.05);
		assert!(strafe.stance_amount(0.5) > 0.95);
		assert!(strafe.stance_amount(1.0) < 0.05);
	}

	#[test]
	fn fake_spine_is_opposite_travel() {
		let left = PowerStrafe::new(Side::Left);
		let right = PowerStrafe::new(Side::Right);
		assert!(left.spine_side(FAKE_PEAK) * left.side.sign() < -0.1);
		assert!(right.spine_side(FAKE_PEAK) * right.side.sign() < -0.1);
		assert!(left.spine_side(0.5) * left.side.sign() > 0.1);
		assert!(right.spine_side(0.5) * right.side.sign() > 0.1);
		assert!((left.spine_side(FAKE_PEAK) + right.spine_side(FAKE_PEAK)).abs() < 1e-5);
		assert!((left.spine_side(0.5) + right.spine_side(0.5)).abs() < 1e-5);
	}

	#[test]
	fn armature_offset_is_zero_at_end_and_smooth_over_recovery_window() {
		use character_rigs::Side;

		const FPS: f32 = 60.0;
		const RECOVERY_WINDOW: f32 = 0.38;
		let strafe = PowerStrafe::new(Side::Left);
		let start = 1.0 - RECOVERY_WINDOW;
		let steps = (RECOVERY_WINDOW * FPS).round() as u32;
		let mut max_delta = 0.0f32;
		let mut prev = strafe.armature_translation(start);
		for step in 1..=steps {
			let progress = start + RECOVERY_WINDOW * step as f32 / steps as f32;
			let next = strafe.armature_translation(progress);
			max_delta = max_delta.max((next - prev).length());
			prev = next;
		}
		assert!(prev.length_squared() < 1e-10, "offset at progress 1: {prev:?}");
		assert!(max_delta < 0.02, "max 60 Hz step delta {max_delta} in recovery tail");
	}
}
