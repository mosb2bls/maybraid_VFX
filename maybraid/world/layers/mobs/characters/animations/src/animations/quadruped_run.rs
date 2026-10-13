//! Quadruped run (diagonal trot) parameters.
//!
//! Progress is owned by the controller; [`QuadrupedRun`] is a pure sampler at a normalized
//! cycle phase in `[0, 1)`. [`QuadrupedRunPose`] holds the tuned joint knobs.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct QuadrupedRun {
	/// Thigh forward/back stride amplitude (radians).
	pub stride: f32,
	/// Vertical bob; 1.0 = tuned quadruped default.
	pub bounce: f32,
	/// Spine/neck motion in the gait; 1.0 = tuned quadruped default.
	pub rotation: f32,
}

impl Default for QuadrupedRun {
	fn default() -> Self {
		Self { stride: 1.0, bounce: 1.0, rotation: 1.0 }
	}
}

/// Tuned quadruped run pose knobs derived from [`QuadrupedRun`].
#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedRunPose {
	pub shoulder_swing: f32,
	pub shoulder_lift: f32,
	pub hip_swing: f32,
	pub hip_lift: f32,
	pub stride: f32,
	pub knee_neutral: f32,
	pub knee_contracted: f32,
	pub knee_extended: f32,
	pub spine_swing: f32,
	/// Neck turn (axial +Y).
	pub neck_swing: f32,
	/// Neck tilt (lateral +Z).
	pub neck_bow: f32,
	/// Neck nod (flexion +X).
	pub neck_pitch: f32,
}

impl Default for QuadrupedRunPose {
	fn default() -> Self {
		Self {
			shoulder_swing: 0.12,
			shoulder_lift: 0.05,
			hip_swing: 0.14,
			hip_lift: 0.06,
			stride: 1.0,
			knee_neutral: PI * 0.5,
			knee_contracted: 2.0,
			knee_extended: 0.3,
			spine_swing: 0.08,
			neck_swing: 0.05,
			neck_bow: 0.08,
			neck_pitch: 0.07,
		}
	}
}

impl QuadrupedRunPose {
	/// Scale the tuned quadruped template from rig-agnostic [`QuadrupedRun`] knobs.
	pub fn from_run(run: &QuadrupedRun) -> Self {
		let template = Self::default();
		let reference = QuadrupedRun::default();
		let stride_scale = run.stride / reference.stride;

		let knee_neutral_delta = template.knee_neutral - template.knee_extended;
		let knee_contracted_delta = template.knee_contracted - template.knee_neutral;

		Self {
			stride: template.stride * stride_scale,
			shoulder_lift: template.shoulder_lift * run.bounce,
			hip_lift: template.hip_lift * run.bounce,
			shoulder_swing: template.shoulder_swing * run.rotation,
			hip_swing: template.hip_swing * run.rotation,
			spine_swing: template.spine_swing * run.rotation,
			neck_swing: template.neck_swing * run.rotation,
			neck_bow: template.neck_bow * run.rotation,
			neck_pitch: template.neck_pitch * run.rotation,
			knee_extended: template.knee_extended,
			knee_neutral: template.knee_extended + knee_neutral_delta * stride_scale,
			knee_contracted: template.knee_extended
				+ knee_neutral_delta * stride_scale
				+ knee_contracted_delta * stride_scale,
		}
	}
}
