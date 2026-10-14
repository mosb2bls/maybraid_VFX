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
}

impl Default for UprightPowerDash {
	fn default() -> Self {
		Self {
			jab: Jab::default(),
			drive: 1.0,
			slide: 1.0,
			lead_stride: 1.05,
			trail_stride: 0.72,
			lunge_knee_lead: 1.35,
			lunge_knee_trail: 0.55,
			lunge_lean: 0.42,
			arm_reach: 0.65,
			slide_knee: 1.05,
			slide_lean: 0.38,
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
		}
	}
}
