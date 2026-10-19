use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::{EpiphantSleepEnter, EpiphantSleepHold, EpiphantWake};
use crate::rigs::quadruped::apply::{apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine};
use crate::{Animation, Progress};

impl Animation<QuadrupedV0Rig> for EpiphantSleepEnter {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		let mut pose = QuadrupedPose::default();
		let hinge = self.leg_hinge(t);
		let stride = self.leg_stride(t);
		for side in [Side::Left, Side::Right] {
			apply_front_leg(&mut pose, side, 0.0, 0.0, stride, hinge);
			apply_hind_leg(&mut pose, side, 0.0, 0.0, stride * 0.85, hinge * 1.05);
		}
		apply_spine(&mut pose, 0.0, self.spine_lower(t));
		apply_neck_axes(&mut pose, 0.0, 0.0, self.neck_down(t));
		rig.write_pose(&pose);
	}
}

impl Animation<QuadrupedV0Rig> for EpiphantSleepHold {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let enter = EpiphantSleepEnter::default();
		let mut pose = QuadrupedPose::default();
		let end = Progress(1.0).clamp();
		let hinge = enter.leg_hinge(end);
		let stride = enter.leg_stride(end);
		for side in [Side::Left, Side::Right] {
			apply_front_leg(&mut pose, side, 0.0, 0.0, stride, hinge);
			apply_hind_leg(&mut pose, side, 0.0, 0.0, stride * 0.85, hinge * 1.05);
		}
		apply_spine(&mut pose, 0.0, enter.spine_lower(end) + self.spine_breathe(progress));
		apply_neck_axes(&mut pose, 0.0, 0.0, enter.neck_down(end) + self.neck_breathe(progress));
		rig.write_pose(&pose);
	}
}

impl Animation<QuadrupedV0Rig> for EpiphantWake {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		let mut pose = QuadrupedPose::default();
		let hinge = self.leg_hinge(t);
		let stride = self.leg_stride(t);
		for side in [Side::Left, Side::Right] {
			let plant = EpiphantWake::front_plant_weight(t);
			apply_front_leg(&mut pose, side, 0.0, 0.0, stride, hinge * (1.0 - plant * 0.35));
			apply_hind_leg(&mut pose, side, 0.0, 0.0, stride * 0.7, hinge);
		}
		apply_spine(&mut pose, 0.0, self.spine_lift(t));
		apply_neck_axes(&mut pose, 0.0, 0.0, self.neck_lift(t));
		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn sleep_entry_folds_legs_and_returns_at_end_of_wake() {
		let enter = EpiphantSleepEnter::default();
		let mut folded = QuadrupedV0Rig::for_clip_test();
		enter.apply_for(&mut folded, 1.0);
		assert!(folded.posed_angle("anterior_shin.L") > 0.2);

		let wake = EpiphantWake::default();
		let mut standing = QuadrupedV0Rig::for_clip_test();
		wake.apply_for(&mut standing, 1.0);
		assert!(standing.posed_angle("anterior_shin.L") < folded.posed_angle("anterior_shin.L"));
	}
}
