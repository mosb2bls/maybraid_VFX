//! Humanoid idle: hung arms, a visible look-around, and an occasional scratch.
//!
//! Progress is owned by the controller and may grow past `1.0`. Oscillators
//! use that unwrapped time so a non-integer look rate does not snap at the
//! cycle seam. Scratch is a short burst on a longer period; the envelope
//! starts and ends at rest so a period wrap is quiet.

use character_rigs::Side;

use crate::animations::{burst_envelope, DEFAULT_BURST_EDGE};

/// Golden-ratio odd constant; same mix family as POI discovery.
const PHASE_GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;

/// Walk-sized hang so rest is not a T-pose. Sway on top stays a few degrees.
const DEFAULT_ARM_HANG: f32 = 1.2;
const DEFAULT_ELBOW_HANG: f32 = 0.32;
const DEFAULT_ARM_SWAY: f32 = 0.06;
const DEFAULT_NECK_ROLL: f32 = 0.2;
const DEFAULT_HIP_SHIFT: f32 = 0.025;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Idle {
	/// Humerus flex that drops the arms from T-pose (radians).
	pub arm_hang: f32,
	/// Rest elbow bend (radians).
	pub elbow_hang: f32,
	/// Shoulder / humerus swing on top of the hang (radians).
	pub arm_sway: f32,
	/// Neck yaw / nod amplitude (radians).
	pub neck_roll: f32,
	/// Pelvis weight-shift amplitude (radians). Keep smaller than the arms.
	pub hip_shift: f32,
}

impl Default for Idle {
	fn default() -> Self {
		Self {
			arm_hang: DEFAULT_ARM_HANG,
			elbow_hang: DEFAULT_ELBOW_HANG,
			arm_sway: DEFAULT_ARM_SWAY,
			neck_roll: DEFAULT_NECK_ROLL,
			hip_shift: DEFAULT_HIP_SHIFT,
		}
	}
}

impl Idle {
	/// Cycles of each oscillator per unit of unwrapped progress.
	pub const ARM_FREQ: f32 = 1.0;
	pub const NECK_YAW_FREQ: f32 = 0.35;
	pub const NECK_PITCH_FREQ: f32 = 0.22;
	pub const HIP_FREQ: f32 = 0.8;
	/// Quiet time between scratches, in progress units (~16 s at idle speed).
	pub const SCRATCH_PERIOD: f32 = 3.2;
	/// Scratch burst length, in progress units (~2 s at idle speed).
	pub const SCRATCH_DURATION: f32 = 0.45;

	/// Deterministic `[0, 1)` phase from entity bits so a crowd does not lock step.
	pub fn phase_from_entity_bits(bits: u64) -> f32 {
		let mixed = bits.wrapping_mul(PHASE_GOLDEN);
		((mixed >> 32) as u32 as f32) * (1.0 / (u32::MAX as f32))
	}

	/// Look wave: eases at the glance, more travel than a cube. Continuous
	/// in `progress` (do not wrap first).
	pub fn look_wave(progress: f32, freq: f32, offset: f32) -> f32 {
		let s = (std::f32::consts::TAU * (progress * freq + offset)).sin();
		s * s.abs()
	}

	/// `0` outside the burst, `1` at the hold. Continuous across the period wrap.
	pub fn scratch_weight(progress: f32) -> f32 {
		let t = progress.rem_euclid(Self::SCRATCH_PERIOD);
		if t >= Self::SCRATCH_DURATION {
			return 0.0;
		}
		burst_envelope(t / Self::SCRATCH_DURATION, DEFAULT_BURST_EDGE)
	}

	/// Alternate hands each period so a long idle is not always the same arm.
	pub fn scratch_side(progress: f32) -> Side {
		let cycle = (progress / Self::SCRATCH_PERIOD).floor() as i32;
		if cycle.rem_euclid(2) == 0 {
			Side::Right
		} else {
			Side::Left
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_sway_stays_small_look_is_readable() {
		let idle = Idle::default();
		assert!(idle.arm_sway < 0.1);
		assert!(idle.neck_roll > 0.12);
		assert!(idle.neck_roll < 0.3);
		assert!(idle.hip_shift < idle.arm_sway);
		assert!(idle.arm_hang > 1.0);
	}

	#[test]
	fn entity_bits_desync_phase() {
		let a = Idle::phase_from_entity_bits(1);
		let b = Idle::phase_from_entity_bits(2);
		assert!(a >= 0.0 && a < 1.0);
		assert!(b >= 0.0 && b < 1.0);
		assert_ne!(a, b);
	}

	#[test]
	fn look_wave_is_continuous_across_unit_progress() {
		let before = Idle::look_wave(0.999, Idle::NECK_YAW_FREQ, 0.15);
		let after = Idle::look_wave(1.001, Idle::NECK_YAW_FREQ, 0.15);
		assert!((before - after).abs() < 0.02);
	}

	#[test]
	fn scratch_is_quiet_outside_the_burst() {
		assert!(Idle::scratch_weight(Idle::SCRATCH_DURATION + 0.05) < 1e-5);
		assert!(Idle::scratch_weight(Idle::SCRATCH_PERIOD - 0.01) < 1e-5);
		assert!(Idle::scratch_weight(Idle::SCRATCH_DURATION * 0.5) > 0.9);
	}

	#[test]
	fn scratch_envelope_is_closed_at_the_period_wrap() {
		assert!(Idle::scratch_weight(0.0) < 1e-5);
		assert!(Idle::scratch_weight(Idle::SCRATCH_PERIOD) < 1e-5);
	}

	#[test]
	fn scratch_alternates_hands() {
		assert_eq!(Idle::scratch_side(0.2), Side::Right);
		assert_eq!(Idle::scratch_side(Idle::SCRATCH_PERIOD + 0.2), Side::Left);
	}
}
