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
	pub(crate) support_knee: f32,
	pub(crate) trail_knee: f32,
	pub(crate) support_femur: f32,
	pub(crate) trail_femur: f32,
	pub(crate) lean: f32,
	pub(crate) yaw: f32,
	pub(crate) hip_abduct: f32,
}

impl Default for UprightPowerDash {
	fn default() -> Self {
		Self {
			jab: Jab::default(),
			drive: 1.0,
			slide: 1.0,
			support_knee: 1.38,
			trail_knee: 0.38,
			support_femur: -1.08,
			trail_femur: 0.82,
			lean: 0.68,
			yaw: 0.22,
			hip_abduct: 0.22,
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
			support_knee: template.support_knee * drive_scale,
			trail_knee: template.trail_knee,
			support_femur: template.support_femur * drive_scale,
			trail_femur: template.trail_femur * drive_scale,
			lean: template.lean * drive_scale,
			yaw: template.yaw,
			hip_abduct: template.hip_abduct,
		}
	}
}
