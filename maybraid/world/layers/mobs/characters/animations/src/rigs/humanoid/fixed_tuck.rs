use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{FixedPosition, FixedTuck};
use crate::rigs::humanoid::tuck::apply_tuck_profile;
use crate::{Animation, Effects};

impl FixedPosition<HumanoidV0Rig> for FixedTuck {
	fn apply_fixed(&self, rig: &mut HumanoidV0Rig) -> Effects {
		apply_tuck_profile(rig, &self.profile(), 1.0)
	}
}

impl Animation<HumanoidV0Rig> for FixedTuck {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, _progress: f32) {
		let _ = self.apply_fixed(rig);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;
	use crate::animations::Tuck;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	#[test]
	fn fixed_tuck_ignores_progress() -> anyhow::Result<()> {
		let mut at_zero = HumanoidV0Rig::imported();
		let mut at_half = HumanoidV0Rig::imported();
		let fixed = FixedTuck::default();

		fixed.apply(&mut at_zero, 0.0);
		fixed.apply(&mut at_half, 0.5);

		for name in at_zero.animation_bone_names() {
			let zero = at_zero.rotation(name);
			let half = at_half.rotation(name);
			assert!(zero.dot(half).abs() > 1.0 - 1e-5, "drift on {name}");
		}
		Ok(())
	}

	#[test]
	fn fixed_tuck_matches_full_tuck_sample() -> anyhow::Result<()> {
		let mut fixed = HumanoidV0Rig::imported();
		let mut ramped = HumanoidV0Rig::imported();
		FixedTuck::default().apply_fixed(&mut fixed);
		Tuck::default().apply(&mut ramped, 1.0);

		let fixed_shin = tip(&fixed, "shin.L");
		let ramped_shin = tip(&ramped, "shin.L");
		assert!((fixed_shin - ramped_shin).length() < 1e-4);
		Ok(())
	}
}
