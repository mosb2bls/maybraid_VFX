use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::Spring;
use crate::Animation;

impl Animation<HumanoidV0Rig> for Spring {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		pose.apply_leg(Side::Left, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_leg(Side::Right, self.femur_swing(progress), self.shin_flex(progress));
		pose.apply_root(self.root_swing(progress));

		for side in [Side::Left, Side::Right] {
			pose.apply_arm(
				side,
				self.shoulder_swing(progress),
				0.0,
				0.0,
				self.humerus_flex(progress),
				self.forearm_flex(progress),
			);
		}
		rig.write_pose(&pose);
	}
}
