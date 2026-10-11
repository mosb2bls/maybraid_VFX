use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::EpiphantBray;
use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine};
use crate::{Animation, Progress};

impl Animation<QuadrupedV0Rig> for EpiphantBray {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		let mut pose = QuadrupedPose::default();
		let gather = self.spine_gather(t);
		apply_spine(&mut pose, gather * 0.2, gather);
		apply_neck_axes(&mut pose, self.neck_turn(t), 0.0, self.neck_nod(t));

		let brace = EpiphantBray::flick_weight(progress) * 0.12;
		for side in [Side::Left, Side::Right] {
			apply_front_leg(&mut pose, side, 0.0, 0.0, -brace, brace * 0.6);
			apply_hind_leg(&mut pose, side, 0.0, 0.0, brace * 0.4, brace * 0.35);
		}
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	#[test]
	fn flick_moves_the_neck_on_the_imported_rig() {
		let bray = EpiphantBray::default();
		let mut rig = QuadrupedV0Rig::for_clip_test();
		bray.apply_for(&mut rig, 0.34);
		let neck = rig.rotation("neck") * Vec3::Y;
		assert!(neck.x.abs() > 0.08, "flick should turn the neck, got {neck:?}");
		assert!(neck.z > 0.05, "flick should nod up, got {neck:?}");

		bray.apply_for(&mut rig, 1.0);
		let rest = rig.rotation("neck") * Vec3::Y;
		assert!(
			(rest - Vec3::Y).length() < 0.12,
			"recovery should return near rest, got {rest:?}"
		);
	}
}
