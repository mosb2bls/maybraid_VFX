//! Humanoid mapping for [`Jab`](crate::animations::Jab).
//!
//! The punching humerus aims in character space (+Z fight-forward). Elbow flexion
//! stays bone-local. Trunk turn is axial yaw spread across lumbar, mid-back, and
//! upper back. Waist bend and root lean are sagittal flexion.

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Jab;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Jab {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let jab_side = self.side;
		let guard_side = self.opposite_side();

		pose.apply_leg(jab_side, self.lead_femur_swing(progress), self.stance_shin_flex(progress));
		pose.apply_leg(
			guard_side,
			self.rear_femur_swing(progress),
			self.stance_shin_flex(progress),
		);
		apply_trunk(
			&mut pose,
			jab_side,
			self.torso_turn(progress),
			self.waist_bend(progress),
			self.root_lean(progress),
		);
		apply_hip_turn(&mut pose, jab_side, self.hip_turn(progress));
		apply_arm(
			&mut pose,
			jab_side,
			self.humerus_along(jab_side, progress),
			humerus_roll(jab_side, self.punch_roll(progress)),
			self.shoulder_carry(progress),
			self.jab_elbow(progress),
		);
		apply_arm(
			&mut pose,
			guard_side,
			self.humerus_along(guard_side, progress),
			humerus_roll(guard_side, self.punch_roll(progress)),
			0.0,
			self.guard_elbow(progress),
		);
		rig.write_pose(&pose);
	}
}

/// Long-axis roll after aim. The sign follows the punching side so both elbows
/// share the fight plane; it is aim roll, not a mirrored flexion axis.
fn humerus_roll(side: Side, punch_roll: f32) -> f32 {
	punch_roll * side.sign()
}

fn apply_arm(
	pose: &mut HumanoidPose,
	side: Side,
	along: Vec3,
	roll: f32,
	shoulder_carry: f32,
	elbow: f32,
) {
	let arm = pose.arm_mut(side);
	arm.shoulder_forward += shoulder_carry;
	arm.elbow_flexion += elbow;
	arm.aim = Some(ArmAim { along, roll });
}

fn apply_trunk(
	pose: &mut HumanoidPose,
	jab_side: Side,
	turn: f32,
	waist_bend: f32,
	root_lean: f32,
) {
	let yaw = turn * -jab_side.sign();
	pose.spine.add_root_forward(root_lean);
	pose.spine.add_waist_forward(waist_bend);
	pose.spine.add_turn(yaw);
}

fn apply_hip_turn(pose: &mut HumanoidPose, jab_side: Side, hip: f32) {
	let yaw = hip * -jab_side.sign();
	pose.leg_mut(jab_side).pelvis_turn += yaw;
	pose.leg_mut(jab_side.opposite()).pelvis_turn += yaw * 0.65;
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use character_rigs::articulation::BONE_LENGTH_AXIS;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::Animation;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn aimed_length(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		let bone = rig.binding.definition.id(name).expect(name);
		let parent = rig.binding.definition.parent_rotation(&rig.pose, bone);
		(parent * rig.pose.rotation(bone) * BONE_LENGTH_AXIS).normalize()
	}

	#[test]
	fn jab_extends_punching_forearm() -> anyhow::Result<()> {
		let jab = Jab::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		assert!(rig.posed_angle("forearm.R") < 0.2, "extended elbow stays near rest");
		Ok(())
	}

	#[test]
	fn jab_applies_lead_stance() -> anyhow::Result<()> {
		let jab = Jab::default();
		let mut rig = HumanoidV0Rig::for_clip_test();
		jab.apply(&mut rig, 0.47);

		assert!(rig.posed_angle("femur.R") > 0.0, "lead hip leaves rest");
		assert!(rig.posed_angle("femur.L") > 0.0, "rear hip leaves rest");
		assert!(
			(rig.character_length("femur.L").z - rig.character_length("femur.R").z).abs() > 0.05,
			"lead and rear stance should differ"
		);
		Ok(())
	}

	#[test]
	fn cover_arm_keeps_tucked_elbow() -> anyhow::Result<()> {
		let jab = Jab::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		assert!(rig.posed_angle("forearm.L") > 0.5, "cover elbow should stay tucked");
		Ok(())
	}

	#[test]
	fn punch_aims_humerus_along_character_forward() -> anyhow::Result<()> {
		let jab = Jab::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.0);

		let along = jab.humerus_along(Side::Right, 0.0);
		let aimed = aimed_length(&rig, "humerus.R");
		assert!(aimed.dot(along.normalize()) > 0.99, "expected {along:?}, got {aimed:?}");
		assert!(jab.shoulder_carry(0.0) < 0.2);
		Ok(())
	}

	#[test]
	fn higher_target_aims_humerus_less_down() -> anyhow::Result<()> {
		let sternum = Jab::default().with_side(Side::Right);
		let chin = Jab::default().with_side(Side::Right).with_target(Vec3::new(0.0, 0.55, 0.7));
		assert!(chin.humerus_along(Side::Right, 0.0).y > sternum.humerus_along(Side::Right, 0.0).y);
		Ok(())
	}

	#[test]
	fn torso_turn_is_yaw_and_lean_is_sagittal() -> anyhow::Result<()> {
		let jab = Jab::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		for name in ["lumbar", "midback", "upper_back"] {
			let yawed = rig.rotation(name) * Vec3::Z;
			assert!(yawed.x.abs() > 0.02, "{name} should yaw, got {yawed:?}");
		}
		let root = tip(&rig, "root");
		assert!(root.z > 0.02, "root lean is sagittal, got {root:?}");
		assert!(root.x.abs() < 0.05, "root lean is not the turn, got {root:?}");
		Ok(())
	}

	#[test]
	fn waist_bend_loads_the_spine_sagittally() -> anyhow::Result<()> {
		let jab = Jab::default();
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let root = tip(&rig, "root");
		let lumbar = tip(&rig, "lumbar");
		let midback = tip(&rig, "midback");
		let upper = tip(&rig, "upper_back");
		assert!(root.z > lumbar.z, "root should carry more pitch than lumbar");
		assert!(lumbar.z > midback.z, "lumbar should carry more pitch than mid");
		assert!(midback.z > 0.0);
		assert!(upper.z > 0.0);
		assert!(upper.z < midback.z);
		for bone in [root, lumbar, midback, upper] {
			assert!(bone.x.abs() < 0.15, "pitch stays mostly sagittal, got {bone:?}");
		}
		Ok(())
	}

	#[test]
	fn hip_turn_yaws_the_pelvis() -> anyhow::Result<()> {
		let jab = Jab::default().with_side(Side::Right);
		let mut rig = HumanoidV0Rig::imported();
		jab.apply(&mut rig, 0.47);

		let pelvis = rig.rotation("pelvis.R") * Vec3::Z;
		assert!(pelvis.x.abs() > 0.02, "pelvis yaw, got {pelvis:?}");
		Ok(())
	}
}
