use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::Soaring;
use crate::rigs::humanoid::wing::{apply_flight_body, apply_flight_wings};
use crate::Animation;

impl Animation<HumanoidV0Rig> for Soaring {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		apply_flight_body(&mut pose);
		apply_flight_wings(&mut pose, self.flap_amount(progress));
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn soaring_holds_spread_while_gliding() -> anyhow::Result<()> {
		let soar = Soaring::default();
		let mut rig = HumanoidV0Rig::imported();
		let glide_t = soar.burst_duration() + soar.pause * 0.5;
		soar.apply(&mut rig, glide_t);
		assert!(rig.posed_angle("shoulder.L") > 0.15, "held swing leaves rest");
		assert!(
			(rig.posed_angle("shoulder.L") - rig.posed_angle("shoulder.R")).abs() < 1e-3,
			"opposite sides share the held amplitude"
		);
		Ok(())
	}
}
