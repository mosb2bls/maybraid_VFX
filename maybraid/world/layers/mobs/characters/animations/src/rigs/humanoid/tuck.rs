use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Tuck, TuckProfile};
use crate::{Animation, Effects};

/// Apply tuck articulation scaled by `amount` in `[0.0, 1.0]`.
pub fn apply_tuck_profile(rig: &mut HumanoidV0Rig, profile: &TuckProfile, amount: f32) -> Effects {
	let mut pose = HumanoidPose::default();
	pose.apply_leg(Side::Left, profile.femur_swing(amount), profile.shin_flex(amount));
	pose.apply_leg(Side::Right, profile.femur_swing(amount), profile.shin_flex(amount));

	for side in [Side::Left, Side::Right] {
		let arm = pose.arm_mut(side);
		arm.shoulder_forward = profile.shoulder_roll(side, amount);
		arm.forward_elevation = profile.humerus_swing(side, amount);
		arm.lateral_elevation = profile.humerus_flex(side, amount);
		arm.axial_rotation = profile.humerus_twist(side, amount);
		arm.elbow_flexion = profile.forearm_flex(amount);
	}

	rig.write_pose(&pose);
	ArmatureOffset::IDENTITY
}

impl Animation<HumanoidV0Rig> for Tuck {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let _ = apply_tuck_profile(rig, &self.profile(), self.tuck_amount(progress));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tuck_bends_knees_on_rig() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 0.5);

		assert!(rig.posed_angle("shin.L") > 0.5, "knee folds");
		Ok(())
	}

	#[test]
	fn tuck_drives_humerus_lateral_axial_and_forearm() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 0.5);

		assert!(rig.posed_angle("humerus.L") > 0.05, "humerus leaves rest");
		assert!(rig.posed_angle("forearm.L") > 0.05, "elbow folds");
		Ok(())
	}

	#[test]
	fn tuck_shoulders_roll_inward_symmetrically() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 1.0);

		assert!(rig.posed_angle("shoulder.L") > 0.05, "shoulders leave rest");
		assert!(
			(rig.posed_angle("shoulder.L") - rig.posed_angle("shoulder.R")).abs() < 1e-3,
			"matched amplitude"
		);
		Ok(())
	}

	#[test]
	fn tuck_drives_humerus_twist_and_forearm_flex() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		Tuck::default().apply(&mut rig, 1.0);

		assert!(rig.posed_angle("humerus.L") > 0.5, "full tuck spins the humerus");
		assert!(rig.posed_angle("forearm.L") > 1.0, "full tuck closes the elbow");
		Ok(())
	}
}
