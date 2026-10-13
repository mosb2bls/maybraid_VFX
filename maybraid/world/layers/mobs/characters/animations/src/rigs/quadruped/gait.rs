use character_rigs::authoring::QuadrupedPose;
use character_rigs::Side;

use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct KneeTuning {
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LegStrideTuning {
	pub shoulder_swing: f32,
	pub shoulder_lift: f32,
	pub hip_swing: f32,
	pub hip_lift: f32,
	pub stride: f32,
	pub knee: KneeTuning,
}

pub(crate) fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

pub(crate) fn knee_flex(leg_phase: f32, knee: KneeTuning) -> f32 {
	let p = leg_phase.fract();
	let peak = if p < 0.5 { knee.knee_extended } else { knee.knee_contracted };
	let t = if p < 0.5 { p * 2.0 } else { (p - 0.5) * 2.0 };
	knee.knee_neutral + (t * std::f32::consts::PI).sin() * (peak - knee.knee_neutral)
}

pub(crate) fn leg_phase_from_strike(cycle: f32, strike: f32) -> f32 {
	(cycle - strike).rem_euclid(1.0)
}

/// Continuous stride articulation at an arbitrary leg phase (`0` at foot strike).
///
/// Shoulder lift flips with the side as a gait bias. Shin hinge stays the same
/// positive flexion on both legs.
pub(crate) fn apply_front_leg_stride(
	pose: &mut QuadrupedPose,
	side: Side,
	leg_phase: f32,
	tuning: LegStrideTuning,
) {
	let swing = thigh_swing(leg_phase);
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };

	apply_front_leg(
		pose,
		side,
		swing * tuning.shoulder_swing,
		swing * tuning.shoulder_lift * lift_sign,
		swing * tuning.stride,
		knee_flex(leg_phase, tuning.knee) - tuning.knee.knee_extended,
	);
}

/// Continuous stride articulation at an arbitrary leg phase (`0` at foot strike).
pub(crate) fn apply_hind_leg_stride(
	pose: &mut QuadrupedPose,
	side: Side,
	leg_phase: f32,
	tuning: LegStrideTuning,
) {
	let swing = thigh_swing(leg_phase);
	let lift_sign = if side == Side::Left { -1.0 } else { 1.0 };

	apply_hind_leg(
		pose,
		side,
		swing * tuning.hip_swing,
		swing * tuning.hip_lift * lift_sign,
		swing * tuning.stride,
		knee_flex(leg_phase, tuning.knee) - tuning.knee.knee_extended,
	);
}

pub(crate) fn apply_front_leg_at_strike(
	pose: &mut QuadrupedPose,
	side: Side,
	cycle: f32,
	strike: f32,
	tuning: LegStrideTuning,
) {
	apply_front_leg_stride(pose, side, leg_phase_from_strike(cycle, strike), tuning);
}

pub(crate) fn apply_hind_leg_at_strike(
	pose: &mut QuadrupedPose,
	side: Side,
	cycle: f32,
	strike: f32,
	tuning: LegStrideTuning,
) {
	apply_hind_leg_stride(pose, side, leg_phase_from_strike(cycle, strike), tuning);
}
