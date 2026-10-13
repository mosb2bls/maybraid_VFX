//! Quadruped leap tuning parameters.
//!
//! [`QuadrupedLeap`] holds art-level quadruped knobs. Rig-agnostic callers use [`Leap`](super::Leap)
//! and convert via [`QuadrupedLeap::from_leap`].

use super::Leap;

#[derive(Debug, Clone, PartialEq)]
pub struct QuadrupedLeap {
	/// Hind thigh back-swing at takeoff push (radians).
	pub hind_push: f32,
	/// Front thigh gather at takeoff (radians, applied negative).
	pub front_gather: f32,
	/// All four thighs tucked at mid-air (radians).
	pub air_tuck: f32,
	/// Spine gather (lumbar lateral) at mid-bound.
	pub spine_gather: f32,
	/// Knee flex at peak land absorb.
	pub land_compress: f32,
	/// Thigh stride scale for takeoff split.
	pub stride: f32,
	pub knee_extended: f32,
	pub knee_contracted: f32,
	pub neck_follow: f32,
}

impl Default for QuadrupedLeap {
	fn default() -> Self {
		Self {
			hind_push: 0.70,
			front_gather: 0.55,
			air_tuck: 0.50,
			spine_gather: 0.16,
			land_compress: 0.55,
			stride: 1.1,
			knee_extended: 0.25,
			knee_contracted: 2.0,
			neck_follow: 0.45,
		}
	}
}

impl QuadrupedLeap {
	/// Scale the tuned quadruped template from rig-agnostic [`Leap`] knobs.
	pub fn from_leap(leap: &Leap) -> Self {
		let template = Self::default();
		let reference = Leap::default();
		let stride_scale = leap.stride / reference.stride;

		Self {
			hind_push: template.hind_push * stride_scale,
			front_gather: template.front_gather * stride_scale,
			air_tuck: template.air_tuck * leap.gather,
			spine_gather: template.spine_gather * leap.lean,
			land_compress: template.land_compress * leap.gather,
			stride: template.stride * stride_scale,
			knee_extended: template.knee_extended,
			knee_contracted: template.knee_extended
				+ (template.knee_contracted - template.knee_extended) * leap.gather,
			neck_follow: template.neck_follow,
		}
	}

	pub fn knee_air(&self) -> f32 {
		self.knee_extended + (self.knee_contracted - self.knee_extended) * 0.65
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_leap_default_matches_template() {
		assert_eq!(QuadrupedLeap::from_leap(&Leap::default()), QuadrupedLeap::default());
	}

	#[test]
	fn from_leap_scales_push_gather_and_spine() {
		let leap = Leap { gather: 2.0, lean: 0.5, stride: 2.0 };
		let authored = QuadrupedLeap::from_leap(&leap);
		let template = QuadrupedLeap::default();
		assert!((authored.hind_push - template.hind_push * 2.0).abs() < 1e-5);
		assert!((authored.air_tuck - template.air_tuck * 2.0).abs() < 1e-5);
		assert!((authored.spine_gather - template.spine_gather * 0.5).abs() < 1e-5);
	}
}
