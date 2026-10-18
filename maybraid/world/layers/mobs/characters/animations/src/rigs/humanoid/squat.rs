use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Squat;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for Squat {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let femur_swing = self.femur_swing(progress);
		let shin_flex = self.shin_flex(progress);

		pose.apply_leg(Side::Left, femur_swing, shin_flex);
		pose.apply_leg(Side::Right, femur_swing, shin_flex);
		let hip = self.hip_fold(progress);
		if hip.abs() > f32::EPSILON {
			pose.apply_hip_fold(Side::Left, hip);
			pose.apply_hip_fold(Side::Right, hip);
		}
		// Held and looping squat share the same sagittal stack. The authored
		// angle is the total fold, split across root, lumbar, mid-back, and upper back.
		pose.apply_spine_pitch(self.root_swing(progress));
		rig.write_pose(&pose);
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, progress: f32) -> Effects {
		if self.bones_only {
			return ArmatureOffset::IDENTITY;
		}
		let drop = self.vertical_drop(progress, rig.segment_lengths);
		if drop > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn stand_phase_keeps_pose_neutral() {
		let mut rig = HumanoidV0Rig::imported();
		let squat = Squat::for_loop(1.0, 1.0);
		let effects = squat.apply(&mut rig, 0.0);

		assert!((tip(&rig, "femur.L") - Vec3::Y).length() < 1e-4);
		assert!((tip(&rig, "shin.L") - Vec3::Y).length() < 1e-4);
		assert!((tip(&rig, "root") - Vec3::Y).length() < 1e-4);
		assert!(effects.is_identity());
	}

	#[test]
	fn deepest_squat_folds_both_legs_and_the_spine_sagittally() {
		let mut rig = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut rig, 0.5);

		assert!(
			(rig.posed_angle("femur.L") - rig.posed_angle("femur.R")).abs() < 1e-4,
			"same swing on both femurs"
		);
		assert!(rig.posed_angle("femur.L") > 0.3, "hips fold");
		assert!(rig.posed_angle("shin.L") > 0.5, "knees fold");

		for name in ["root", "lumbar", "midback", "upper_back"] {
			let bone = tip(&rig, name);
			assert!(bone.z > 0.02, "{name} should pitch forward, got {bone:?}");
			assert!(bone.x.abs() < 1e-3, "{name} must not yaw, got {bone:?}");
		}
		let root_only = Quat::from_rotation_x(15.0_f32.to_radians()) * Vec3::Y;
		let root = tip(&rig, "root");
		assert!(
			(root - Vec3::Y).length() < (root_only - Vec3::Y).length(),
			"stack share is less than the full angle on the root"
		);
	}

	#[test]
	fn looping_and_held_squat_bend_the_torso_the_same_way() {
		let mut looping = HumanoidV0Rig::imported();
		let mut held = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut looping, 0.5);
		Squat::held().apply(&mut held, 1.0);

		for name in ["root", "lumbar", "midback", "upper_back"] {
			let loop_tip = tip(&looping, name);
			let held_tip = tip(&held, name);
			assert!(loop_tip.z > 0.0 && held_tip.z > 0.0, "{name}");
			assert!(loop_tip.x.abs() < 1e-3 && held_tip.x.abs() < 1e-3, "{name}");
		}
	}

	#[test]
	fn held_squat_has_no_armature_move() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let squat = Squat::held();
		let effects = squat.apply(&mut rig, 1.0);
		if !effects.is_identity() {
			return Err(anyhow::anyhow!("held squat must not move the armature"));
		}
		if rig.posed_angle("femur.L") < 0.4 {
			return Err(anyhow::anyhow!("held squat should fold the hips"));
		}
		if rig.posed_angle("pelvis.L") < 0.1 {
			return Err(anyhow::anyhow!("held squat should crease the pelvis"));
		}
		let root = tip(&rig, "root");
		if root.z < 0.05 || root.x.abs() > 1e-3 {
			return Err(anyhow::anyhow!("held squat should pitch the spine, got {root:?}"));
		}
		Ok(())
	}

	#[test]
	fn deepest_squat_returns_armature_drop_without_bone_translation() {
		let mut rig = HumanoidV0Rig::imported();
		rig.seed_rest("shin.L", Transform::from_translation(Vec3::new(0.0, 0.6, 0.0)));

		let squat = Squat::for_loop(1.0, 1.0);
		let effects = squat.apply(&mut rig, 0.5);

		let drop = squat.vertical_drop(0.5, rig.segment_lengths);
		assert!(drop > 0.0);
		assert_eq!(effects, ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0)));

		let femur = rig.binding.definition.id("femur.L").expect("femur");
		assert_eq!(
			rig.pose.get(femur).expect("pose").translation,
			rig.binding.effective_rest.get(femur).expect("rest").translation
		);
	}
}
