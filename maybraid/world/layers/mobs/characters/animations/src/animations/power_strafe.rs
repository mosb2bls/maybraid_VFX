//! Lateral ice-skating push and glide, generally forward-facing.
//!
//! Beat: load the opposite (pushing) leg, push sideways, transfer onto the
//! travel-side support leg, then hold a low glide. Recovery draws the trailing
//! foot underneath and returns to a compact ready stance.
//!
//! [`PowerStrafe::side`] is the character-space travel direction (left strafe →
//! [`Side::Left`] → −X). Humanoid mapping mirrors one authored left-travel key
//! through [`Side::sign`]; do not hand-author separate left/right pose tables.
//!
//! Tanz `MOVE_TIMING` (0.04 + 0.22 + 0.12 = 0.38 s) lines up with
//! [`LOAD_END`] ≈ 0.105 startup and [`GLIDE_END`] ≈ 0.684 active-phase end.

use bevy::prelude::Vec3;
use character_rigs::Side;

use crate::animations::{smoothstep, LinearBlend};
use crate::Progress;

/// Compress / load the pushing leg. Matches Tanz startup ~0.105.
pub const LOAD_END: f32 = 0.10;
/// Pushing leg extends sideways; pelvis transfers onto the support foot.
pub const PUSH_END: f32 = 0.28;
/// Push-off finishes; pushing foot unloads and lifts slightly.
pub const PUSHOFF_END: f32 = 0.45;
/// Held glide pose (authored first).
pub const GLIDE_HOLD: f32 = 0.50;
/// End of the held glide. Matches Tanz active-phase end ~0.684.
pub const GLIDE_END: f32 = 0.68;
/// Trailing foot tucked underneath; pelvis begins to rise.
pub const TUCK_END: f32 = 0.85;

/// Clip-test shin-tail ground plane. Visual soles sit ~0.5 m above this.
pub const SHIN_TAIL_GROUND_Y: f32 = -1.5;

const KEYS: [(f32, StrafeKey); 8] = [
	(0.0, READY),
	(LOAD_END, LOAD),
	(PUSH_END, PUSH),
	(PUSHOFF_END, PUSHOFF),
	(GLIDE_HOLD, GLIDE),
	(GLIDE_END, GLIDE),
	(TUCK_END, TUCK),
	(1.0, READY),
];

/// Authored for left travel (support = anatomical left / [`Side::Right`] bones).
/// Lateral signs are magnitudes here; the mapper applies [`Side::sign`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StrafeKey {
	pub support_femur: f32,
	pub support_knee: f32,
	/// Positive = support foot outward toward travel.
	pub support_abduct: f32,
	pub support_rot: f32,
	pub push_femur: f32,
	pub push_knee: f32,
	/// Positive = pushing foot outward opposite travel.
	pub push_abduct: f32,
	pub push_rot: f32,
	pub hip_fold_support: f32,
	pub hip_fold_push: f32,
	pub spine_pitch: f32,
	/// Total lateral lean into travel (radians). Mapper weights sum to 1.
	pub spine_side: f32,
	/// Left-authored chest yaw (mapper multiplies by [`Side::sign`]).
	pub chest_turn: f32,
	pub neck_nod: f32,
	pub neck_tilt: f32,
	pub fwd_along: Vec3,
	pub fwd_roll: f32,
	pub fwd_elbow: f32,
	pub fwd_lift: f32,
	pub back_along: Vec3,
	pub back_roll: f32,
	pub back_elbow: f32,
	pub back_lift: f32,
	/// Positive = whole-armature slide toward travel.
	pub offset_x: f32,
	/// Positive = downward armature drop.
	pub drop: f32,
}

const READY: StrafeKey = StrafeKey {
	support_femur: -0.10,
	support_knee: 0.22,
	support_abduct: 0.02,
	support_rot: 0.0,
	push_femur: -0.10,
	push_knee: 0.22,
	push_abduct: 0.02,
	push_rot: 0.0,
	hip_fold_support: 0.02,
	hip_fold_push: 0.02,
	spine_pitch: 0.10,
	spine_side: 0.0,
	chest_turn: 0.0,
	neck_nod: 0.04,
	neck_tilt: 0.0,
	fwd_along: Vec3::new(-0.02, -0.58, 0.60),
	fwd_roll: 0.55,
	fwd_elbow: 1.32,
	fwd_lift: 0.0,
	back_along: Vec3::new(-0.20, -0.52, -0.40),
	back_roll: 0.28,
	back_elbow: 1.05,
	back_lift: 0.0,
	offset_x: 0.0,
	drop: 0.0,
};

const LOAD: StrafeKey = StrafeKey {
	support_femur: -0.32,
	support_knee: 0.55,
	support_abduct: 0.03,
	support_rot: 0.0,
	push_femur: -0.48,
	push_knee: 0.95,
	push_abduct: 0.18,
	push_rot: 0.0,
	hip_fold_support: 0.04,
	hip_fold_push: 0.08,
	spine_pitch: 0.22,
	spine_side: 0.06,
	chest_turn: -0.05,
	neck_nod: 0.06,
	neck_tilt: 0.02,
	fwd_along: Vec3::new(-0.02, -0.48, 0.78),
	fwd_roll: 0.60,
	fwd_elbow: 1.40,
	fwd_lift: 0.0,
	back_along: Vec3::new(-0.18, -0.42, -0.68),
	back_roll: 0.42,
	back_elbow: 0.70,
	back_lift: 0.0,
	offset_x: 0.02,
	drop: 0.14,
};

const PUSH: StrafeKey = StrafeKey {
	support_femur: -0.58,
	support_knee: 0.92,
	support_abduct: 0.05,
	support_rot: 0.0,
	push_femur: -0.18,
	push_knee: 0.38,
	push_abduct: 0.52,
	push_rot: 0.0,
	hip_fold_support: 0.06,
	hip_fold_push: 0.05,
	spine_pitch: 0.34,
	spine_side: 0.14,
	chest_turn: -0.09,
	neck_nod: 0.07,
	neck_tilt: 0.03,
	fwd_along: Vec3::new(-0.10, -0.32, 0.90),
	fwd_roll: 0.85,
	fwd_elbow: 1.55,
	fwd_lift: 0.0,
	back_along: Vec3::new(-0.18, -0.38, -0.82),
	back_roll: 0.55,
	back_elbow: 0.62,
	back_lift: 0.0,
	offset_x: 0.06,
	drop: 0.18,
};

const PUSHOFF: StrafeKey = StrafeKey {
	support_femur: -0.72,
	support_knee: 1.08,
	support_abduct: 0.06,
	support_rot: 0.0,
	push_femur: -0.12,
	push_knee: 0.40,
	push_abduct: 0.66,
	push_rot: 0.0,
	hip_fold_support: 0.08,
	hip_fold_push: 0.04,
	spine_pitch: 0.40,
	spine_side: 0.18,
	chest_turn: -0.11,
	neck_nod: 0.08,
	neck_tilt: 0.04,
	fwd_along: Vec3::new(-0.12, -0.28, 0.92),
	fwd_roll: 0.95,
	fwd_elbow: 1.60,
	fwd_lift: 0.0,
	back_along: Vec3::new(-0.16, -0.42, -0.84),
	back_roll: 0.65,
	back_elbow: 0.58,
	back_lift: 0.0,
	offset_x: 0.08,
	drop: 0.20,
};

const GLIDE: StrafeKey = StrafeKey {
	support_femur: -0.75,
	support_knee: 1.12,
	support_abduct: 0.06,
	support_rot: 0.0,
	push_femur: -0.10,
	push_knee: 0.50,
	push_abduct: 0.70,
	push_rot: 0.0,
	hip_fold_support: 0.08,
	hip_fold_push: 0.04,
	spine_pitch: 0.42,
	spine_side: 0.20,
	chest_turn: -0.12,
	neck_nod: 0.08,
	neck_tilt: 0.04,
	fwd_along: Vec3::new(-0.14, -0.26, 0.92),
	fwd_roll: 1.05,
	fwd_elbow: 1.62,
	fwd_lift: 0.0,
	back_along: Vec3::new(-0.16, -0.45, -0.84),
	back_roll: 0.70,
	back_elbow: 0.52,
	back_lift: 0.0,
	offset_x: 0.08,
	drop: 0.21,
};

const TUCK: StrafeKey = StrafeKey {
	support_femur: -0.38,
	support_knee: 0.58,
	support_abduct: 0.03,
	support_rot: 0.0,
	push_femur: -0.28,
	push_knee: 0.62,
	push_abduct: 0.16,
	push_rot: 0.0,
	hip_fold_support: 0.04,
	hip_fold_push: 0.04,
	spine_pitch: 0.18,
	spine_side: 0.06,
	chest_turn: -0.04,
	neck_nod: 0.05,
	neck_tilt: 0.02,
	fwd_along: Vec3::new(-0.04, -0.42, 0.78),
	fwd_roll: 0.70,
	fwd_elbow: 1.42,
	fwd_lift: 0.0,
	back_along: Vec3::new(-0.18, -0.48, -0.62),
	back_roll: 0.48,
	back_elbow: 0.72,
	back_lift: 0.0,
	offset_x: 0.03,
	drop: 0.08,
};

impl StrafeKey {
	fn lerp(self, other: Self, t: f32) -> Self {
		let f = |a, b| LinearBlend::eval(a, b, t);
		Self {
			support_femur: f(self.support_femur, other.support_femur),
			support_knee: f(self.support_knee, other.support_knee),
			support_abduct: f(self.support_abduct, other.support_abduct),
			support_rot: f(self.support_rot, other.support_rot),
			push_femur: f(self.push_femur, other.push_femur),
			push_knee: f(self.push_knee, other.push_knee),
			push_abduct: f(self.push_abduct, other.push_abduct),
			push_rot: f(self.push_rot, other.push_rot),
			hip_fold_support: f(self.hip_fold_support, other.hip_fold_support),
			hip_fold_push: f(self.hip_fold_push, other.hip_fold_push),
			spine_pitch: f(self.spine_pitch, other.spine_pitch),
			spine_side: f(self.spine_side, other.spine_side),
			chest_turn: f(self.chest_turn, other.chest_turn),
			neck_nod: f(self.neck_nod, other.neck_nod),
			neck_tilt: f(self.neck_tilt, other.neck_tilt),
			fwd_along: lerp_along(self.fwd_along, other.fwd_along, t),
			fwd_roll: f(self.fwd_roll, other.fwd_roll),
			fwd_elbow: f(self.fwd_elbow, other.fwd_elbow),
			fwd_lift: f(self.fwd_lift, other.fwd_lift),
			back_along: lerp_along(self.back_along, other.back_along, t),
			back_roll: f(self.back_roll, other.back_roll),
			back_elbow: f(self.back_elbow, other.back_elbow),
			back_lift: f(self.back_lift, other.back_lift),
			offset_x: f(self.offset_x, other.offset_x),
			drop: f(self.drop, other.drop),
		}
	}
}

fn lerp_along(a: Vec3, b: Vec3, t: f32) -> Vec3 {
	let mixed = a + (b - a) * t;
	let len = mixed.length();
	if len < 1e-5 {
		Vec3::NEG_Y
	} else {
		mixed / len
	}
}

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

	/// Travel direction in documented character space (left → −X).
	pub fn travel_x(self) -> f32 {
		-self.side.sign()
	}

	/// Pushing-leg bones: anatomical right for left travel ([`Side::Left`] bones).
	pub fn push_side(self) -> Side {
		self.side
	}

	/// Support-leg bones: anatomical left for left travel ([`Side::Right`] bones).
	pub fn support_side(self) -> Side {
		self.side.opposite()
	}

	pub fn clamped(progress: f32) -> f32 {
		Progress(progress).clamp()
	}

	/// Continuously blended key at `progress`. Aims are always present.
	pub fn key(&self, progress: f32) -> StrafeKey {
		let t = Self::clamped(progress);
		for pair in KEYS.windows(2) {
			let (t0, a) = pair[0];
			let (t1, b) = pair[1];
			if t <= t1 {
				let span = (t1 - t0).max(f32::EPSILON);
				return a.lerp(b, smoothstep((t - t0) / span));
			}
		}
		READY
	}

	pub fn armature_translation(&self, progress: f32) -> Vec3 {
		let key = self.key(progress);
		Vec3::new(key.offset_x * self.travel_x(), -key.drop, 0.0)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn phase_windows_match_tanz_boundaries() {
		assert!((LOAD_END - 0.105).abs() < 0.01);
		assert!(PUSH_END > LOAD_END);
		assert!(PUSHOFF_END > PUSH_END);
		assert!(GLIDE_HOLD > PUSHOFF_END);
		assert!(GLIDE_END > GLIDE_HOLD);
		assert!((GLIDE_END - 0.684).abs() < 0.01);
		assert!(TUCK_END > GLIDE_END);
		assert!(TUCK_END < 1.0);
	}

	#[test]
	fn glide_is_held_through_the_active_window() {
		let strafe = PowerStrafe::new(Side::Left);
		let a = strafe.key(GLIDE_HOLD);
		let b = strafe.key(GLIDE_END);
		assert!((a.support_knee - b.support_knee).abs() < 1e-5);
		assert!((a.push_abduct - b.push_abduct).abs() < 1e-5);
		assert!((a.drop - b.drop).abs() < 1e-5);
	}

	#[test]
	fn ready_has_zero_offset_and_compact_knees() {
		let strafe = PowerStrafe::new(Side::Left);
		for progress in [0.0, 1.0] {
			let key = strafe.key(progress);
			assert!(key.drop.abs() < 1e-5);
			assert!(key.offset_x.abs() < 1e-5);
			assert!(key.support_knee < 0.35);
			assert!(key.push_knee < 0.35);
			assert!(strafe.armature_translation(progress).length_squared() < 1e-10);
		}
	}

	#[test]
	fn load_compresses_the_pushing_leg() {
		let strafe = PowerStrafe::new(Side::Left);
		let load = strafe.key(LOAD_END);
		let glide = strafe.key(GLIDE_HOLD);
		assert!(load.push_knee > load.support_knee);
		assert!(load.push_knee > 0.7);
		assert!(glide.support_knee > 1.0);
		assert!(glide.support_knee < 1.40);
		assert!(glide.push_knee > 0.26);
		assert!(glide.push_knee < 0.55);
		assert!(glide.spine_pitch > 0.35);
		assert!(glide.spine_pitch < 0.53);
		assert!(glide.spine_side > 0.17);
		assert!(glide.spine_side < 0.27);
	}

	#[test]
	fn armature_offset_is_zero_at_end_and_smooth_over_recovery_window() {
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
		assert!(max_delta < 0.12, "max 60 Hz step delta {max_delta} in recovery tail");
	}

	#[test]
	fn left_and_right_travel_signs_are_opposites() {
		let left = PowerStrafe::new(Side::Left);
		let right = PowerStrafe::new(Side::Right);
		assert_eq!(left.travel_x(), -1.0);
		assert_eq!(right.travel_x(), 1.0);
		assert_eq!(left.push_side(), Side::Left);
		assert_eq!(left.support_side(), Side::Right);
		assert_eq!(right.push_side(), Side::Right);
		assert_eq!(right.support_side(), Side::Left);
		let l = left.armature_translation(GLIDE_HOLD);
		let r = right.armature_translation(GLIDE_HOLD);
		assert!((l.x + r.x).abs() < 1e-5);
		assert!((l.y - r.y).abs() < 1e-5);
		assert!(l.x < 0.0);
	}
}
