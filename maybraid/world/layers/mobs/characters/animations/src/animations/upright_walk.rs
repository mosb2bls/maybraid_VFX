//! Biped upright walk tuning parameters.
//!
//! [`UprightWalk`] holds art-level humanoid knobs. Rig-agnostic callers use [`Walk`](super::Walk)
//! and convert via [`UprightWalk::from_walk`].

use super::Walk;

#[derive(Debug, Clone, PartialEq)]
pub struct UprightWalk {
	/// Humerus flex bias that keeps forearms slightly forward of the torso.
	pub arm_down: f32,
	/// Baseline elbow bend while the arm hangs at the side.
	pub elbow_bend: f32,
	/// Extra elbow bend on the back-swing of the arm.
	pub elbow_pump: f32,
	/// Oscillating elbow contribution over the stride.
	pub elbow_cycle: f32,
	/// Shoulder swing amplitude opposite the matching leg.
	pub shoulder_swing: f32,
	/// Vertical shoulder bounce over the stride (kept small; lift comes from hips).
	pub shoulder_lift: f32,
	/// Pelvis sagittal swing in phase with the femur (radians).
	pub hip_swing: f32,
	/// Vertical pelvis bounce over the stride (primary source of body lift).
	pub hip_lift: f32,
	/// Femur forward/back swing amplitude.
	pub stride: f32,
	/// Femur medial flex opposing hip swing-out (radians at full hip excursion).
	pub femur_medial_counter: f32,
	/// Humerus swing relative to shoulder swing.
	pub humerus_swing_scale: f32,
	/// Constant forward root lean while walking (radians).
	pub torso_lean: f32,
	/// Shin flex on the stance leg (radians).
	pub knee_stance_bend: f32,
	/// Peak shin flex during swing for toe clearance (radians).
	pub knee_swing_bend: f32,
}

impl Default for UprightWalk {
	fn default() -> Self {
		Self {
			arm_down: 1.2,
			elbow_bend: 0.35,
			elbow_pump: 0.12,
			elbow_cycle: 0.05,
			shoulder_swing: 0.08,
			shoulder_lift: 0.0,
			hip_swing: 0.03,
			hip_lift: 0.15,
			stride: 0.35,
			femur_medial_counter: 0.1,
			humerus_swing_scale: 0.5,
			torso_lean: 0.08,
			knee_stance_bend: 0.1,
			knee_swing_bend: 0.8,
		}
	}
}

impl UprightWalk {
	/// Scale the tuned upright template from rig-agnostic [`Walk`] knobs.
	pub fn from_walk(walk: &Walk) -> Self {
		let template = Self::default();
		let reference = Walk::default();
		let stride_scale = walk.stride / reference.stride;

		Self {
			stride: template.stride * stride_scale,
			hip_lift: template.hip_lift * walk.bounce,
			hip_swing: template.hip_swing * walk.rotation,
			femur_medial_counter: template.femur_medial_counter * walk.rotation,
			torso_lean: template.torso_lean * walk.rotation,
			shoulder_swing: template.shoulder_swing * walk.rotation,
			knee_stance_bend: template.knee_stance_bend * stride_scale,
			knee_swing_bend: template.knee_swing_bend * stride_scale,
			shoulder_lift: template.shoulder_lift * walk.bounce,
			arm_down: template.arm_down,
			elbow_bend: template.elbow_bend,
			elbow_pump: template.elbow_pump,
			elbow_cycle: template.elbow_cycle,
			humerus_swing_scale: template.humerus_swing_scale,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_walk_default_matches_template() {
		let walk = Walk::default();
		assert_eq!(UprightWalk::from_walk(&walk), UprightWalk::default());
	}

	#[test]
	fn from_walk_scales_stride() {
		let walk = Walk { stride: 0.7, ..Walk::default() };
		let upright = UprightWalk::from_walk(&walk);
		let template = UprightWalk::default();
		assert!((upright.stride - template.stride * 2.0).abs() < 1e-5);
		assert!((upright.knee_swing_bend - template.knee_swing_bend * 2.0).abs() < 1e-5);
	}

	#[test]
	fn from_walk_scales_bounce_and_rotation() {
		let walk = Walk { bounce: 2.0, rotation: 0.5, ..Walk::default() };
		let upright = UprightWalk::from_walk(&walk);
		let template = UprightWalk::default();
		assert!((upright.hip_lift - template.hip_lift * 2.0).abs() < 1e-5);
		assert!((upright.hip_swing - template.hip_swing * 0.5).abs() < 1e-5);
		assert!((upright.torso_lean - template.torso_lean * 0.5).abs() < 1e-5);
	}
}
