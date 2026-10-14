//! Biped upright PowerDash tuning parameters.
//!
//! [`UprightPowerDash`] holds art-level humanoid knobs. Rig-agnostic callers use
//! [`PowerDash`](super::PowerDash) and convert via [`UprightPowerDash::from_power_dash`].

use super::PowerDash;
use crate::animations::Jab;

#[derive(Debug, Clone)]
pub struct UprightPowerDash {
	pub jab: Jab,
	pub drive: f32,
	pub slide: f32,
	pub(crate) lead_stride: f32,
	pub(crate) trail_stride: f32,
	pub(crate) lunge_knee_lead: f32,
	pub(crate) lunge_knee_trail: f32,
	pub(crate) lunge_lean: f32,
	pub(crate) arm_reach: f32,
	pub(crate) slide_knee: f32,
	pub(crate) slide_lean: f32,
	pub(crate) hip_abduct: f32,
	pub(crate) windup_knee: f32,
}

impl Default for UprightPowerDash {
	fn default() -> Self {
		Self {
			jab: Jab::default(),
			drive: 1.0,
			slide: 1.0,
			lead_stride: 0.78,
			trail_stride: 0.50,
			lunge_knee_lead: 1.42,
			lunge_knee_trail: 1.32,
			lunge_lean: 0.24,
			arm_reach: 1.0,
			slide_knee: 1.34,
			slide_lean: 0.40,
			hip_abduct: 0.50,
			windup_knee: 1.28,
		}
	}
}

impl UprightPowerDash {
	/// Scale the tuned upright template from rig-agnostic [`PowerDash`] knobs.
	pub fn from_power_dash(power: &PowerDash) -> Self {
		let template = Self::default();
		let reference = PowerDash::default();
		let drive_scale = power.drive / reference.drive;
		let slide_scale = power.slide / reference.slide;

		Self {
			jab: Jab::default().with_side(power.side),
			drive: template.drive * drive_scale,
			slide: template.slide * slide_scale,
			lead_stride: template.lead_stride * drive_scale,
			trail_stride: template.trail_stride * drive_scale,
			lunge_knee_lead: template.lunge_knee_lead * drive_scale,
			lunge_knee_trail: template.lunge_knee_trail,
			lunge_lean: template.lunge_lean * drive_scale,
			arm_reach: template.arm_reach * drive_scale,
			slide_knee: template.slide_knee,
			slide_lean: template.slide_lean,
			hip_abduct: template.hip_abduct,
			windup_knee: template.windup_knee,
		}
	}
}
