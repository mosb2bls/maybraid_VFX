//! Biped upright leap tuning parameters.
//!
//! [`UprightLeap`] holds art-level humanoid knobs. Rig-agnostic callers use [`Leap`](super::Leap)
//! and convert via [`UprightLeap::from_leap`].

use super::Leap;

#[derive(Debug, Clone, PartialEq)]
pub struct UprightLeap {
	/// Lead (right) femur forward swing at takeoff (radians, applied negative).
	pub lead_stride: f32,
	/// Trail (left) femur back swing at takeoff (radians, applied positive).
	pub trail_stride: f32,
	/// Both femurs gathered forward at mid-air (radians, applied negative).
	pub air_femur: f32,
	/// Knee flex at mid-air gather.
	pub air_knee: f32,
	/// Lead knee flex at takeoff.
	pub takeoff_knee_lead: f32,
	/// Trail knee flex at takeoff (pushing leg).
	pub takeoff_knee_trail: f32,
	/// Femur forward at peak land absorb (radians, applied negative).
	pub land_femur: f32,
	/// Knee flex at peak land absorb.
	pub land_knee: f32,
	/// Root forward swing at peak lean (radians).
	pub lean: f32,
	/// Contralateral arm drive at takeoff.
	pub arm_drive: f32,
	/// Both arms forward in the air.
	pub air_arm: f32,
	/// Baseline elbow bend through the shot.
	pub elbow: f32,
}

impl Default for UprightLeap {
	fn default() -> Self {
		Self {
			lead_stride: 0.85,
			trail_stride: 0.95,
			air_femur: 0.40,
			air_knee: 1.15,
			takeoff_knee_lead: 0.25,
			takeoff_knee_trail: 0.45,
			land_femur: 0.28,
			land_knee: 0.75,
			lean: 0.22,
			arm_drive: 0.55,
			air_arm: 0.35,
			elbow: 1.1,
		}
	}
}

impl UprightLeap {
	/// Scale the tuned upright template from rig-agnostic [`Leap`] knobs.
	pub fn from_leap(leap: &Leap) -> Self {
		let template = Self::default();
		let reference = Leap::default();
		let stride_scale = leap.stride / reference.stride;

		Self {
			lead_stride: template.lead_stride * stride_scale,
			trail_stride: template.trail_stride * stride_scale,
			air_femur: template.air_femur * leap.gather,
			air_knee: template.air_knee * leap.gather,
			land_femur: template.land_femur * leap.gather,
			land_knee: template.land_knee * leap.gather,
			lean: template.lean * leap.lean,
			arm_drive: template.arm_drive * stride_scale,
			air_arm: template.air_arm * leap.gather,
			takeoff_knee_lead: template.takeoff_knee_lead,
			takeoff_knee_trail: template.takeoff_knee_trail,
			elbow: template.elbow,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_leap_default_matches_template() {
		assert_eq!(UprightLeap::from_leap(&Leap::default()), UprightLeap::default());
	}

	#[test]
	fn from_leap_scales_stride_gather_and_lean() {
		let leap = Leap { gather: 2.0, lean: 0.5, stride: 2.0 };
		let upright = UprightLeap::from_leap(&leap);
		let template = UprightLeap::default();
		assert!((upright.lead_stride - template.lead_stride * 2.0).abs() < 1e-5);
		assert!((upright.air_knee - template.air_knee * 2.0).abs() < 1e-5);
		assert!((upright.lean - template.lean * 0.5).abs() < 1e-5);
	}
}
