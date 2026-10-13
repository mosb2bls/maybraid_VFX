use std::f32::consts::TAU;

use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::{Idle, QuadrupedIdle};
use crate::rigs::quadruped::apply::{
	apply_front_leg, apply_hind_leg, apply_neck_axes, apply_spine,
};
use crate::Animation;

impl Animation<QuadrupedV0Rig> for QuadrupedIdle {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		rig.write_pose(&sample(self, progress));
	}
}

fn sample(idle: &QuadrupedIdle, progress: f32) -> QuadrupedPose {
	let mut pose = QuadrupedPose::default();
	let graze = QuadrupedIdle::graze_weight(progress);
	let look = QuadrupedIdle::look_weight(progress);
	let shake = QuadrupedIdle::shake_weight(progress);
	let sway = (TAU * (progress * QuadrupedIdle::SWAY_FREQ)).sin();
	let bob = (TAU * (progress * QuadrupedIdle::BOB_FREQ)).sin();
	let glance = Idle::look_wave(progress, QuadrupedIdle::GLANCE_FREQ, 0.21);
	let rattle = (TAU * (progress * QuadrupedIdle::SHAKE_FREQ)).sin();

	// Swing is turn, flex is tilt, twist is nod.
	let neck_tilt = -idle.graze_neck * graze
		+ idle.look_neck * look
		+ idle.look_neck * 0.55 * glance * (look + graze * 0.35)
		+ idle.sway * 0.35 * sway;
	let neck_nod = -idle.graze_pitch * graze
		+ idle.look_pitch * look
		+ idle.graze_pitch * 0.06 * bob * graze
		+ idle.sway * 0.35 * sway;
	let neck_turn = idle.shake * rattle * shake;
	apply_neck_axes(&mut pose, neck_turn, neck_tilt, neck_nod);

	let lumbar = idle.lumbar * graze - idle.lumbar * 0.35 * look + idle.sway * sway;
	apply_spine(&mut pose, lumbar * 0.35, lumbar);

	for side in [Side::Left, Side::Right] {
		let plant = graze * 0.08;
		apply_front_leg(&mut pose, side, 0.0, 0.0, -plant, plant * 0.7);
		apply_hind_leg(
			&mut pose,
			side,
			0.0,
			idle.sway * sway * side.sign(),
			plant * 0.5,
			plant * 0.4,
		);
	}
	pose
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	#[test]
	fn graze_bows_the_neck_and_gathers_the_spine() {
		let idle = QuadrupedIdle::default();
		let pose = sample(&idle, QuadrupedIdle::graze_peak());
		assert!(pose.neck_tilt < -0.5, "graze should keep the side-to-side");
		assert!(pose.neck_nod < -0.3, "graze should also nod down");
		assert!(pose.spine_lateral > 0.1);

		let mut rig = QuadrupedV0Rig::imported();
		rig.write_pose(&pose);
		let neck = rig.rotation("neck") * Vec3::Y;
		assert!(neck.x > 0.2, "tilt is lateral, got {neck:?}");
		assert!(neck.z < -0.15, "nod is sagittal, got {neck:?}");
		let lumbar = rig.rotation("lumbar") * Vec3::Y;
		assert!(lumbar.x < -0.05, "gather is lateral, got {lumbar:?}");
		assert!(lumbar.z.abs() < 1e-3, "gather stays out of the sagittal plane, got {lumbar:?}");
	}

	#[test]
	fn look_raises_the_neck_above_rest() {
		let idle = QuadrupedIdle::default();
		let graze = sample(&idle, QuadrupedIdle::graze_peak());
		let look = sample(&idle, QuadrupedIdle::look_peak());

		assert!(look.neck_nod > 0.12);
		assert!(look.neck_nod > graze.neck_nod + 0.4);
		assert!(look.spine_lateral < graze.spine_lateral);
	}

	#[test]
	fn shake_rotates_the_neck_after_the_look() {
		let idle = QuadrupedIdle::default();
		let pose = sample(&idle, QuadrupedIdle::shake_peak());
		assert!(pose.neck_turn.abs() > 0.03);
		assert!(QuadrupedIdle::shake_weight(QuadrupedIdle::shake_peak()) > 0.9);

		let mut rig = QuadrupedV0Rig::imported();
		rig.write_pose(&pose);
		let yaw = rig.rotation("neck") * Vec3::Z;
		assert!(yaw.x.abs() > 0.02, "shake turns the neck, got {yaw:?}");
	}

	#[test]
	fn rest_and_period_wrap_stay_quiet() {
		let idle = QuadrupedIdle::default();
		let rest = sample(&idle, 0.0);
		let wrap = sample(&idle, QuadrupedIdle::PERIOD);

		assert!(rest.neck_turn.abs() < 0.02);
		assert!(rest.neck_tilt.abs() < 0.02);
		assert!(rest.neck_nod.abs() < 0.02);
		assert!(wrap.neck_turn.abs() < 0.03);
		assert!(wrap.neck_tilt.abs() < 0.03);
		assert!(wrap.neck_nod.abs() < 0.03);
	}

	#[test]
	fn envelopes_do_not_snap_across_unit_progress() {
		let idle = QuadrupedIdle::default();
		let before = sample(&idle, 0.999);
		let after = sample(&idle, 1.001);

		assert!((before.neck_turn - after.neck_turn).abs() < 0.05);
		assert!((before.neck_tilt - after.neck_tilt).abs() < 0.05);
		assert!((before.neck_nod - after.neck_nod).abs() < 0.05);
	}

	#[test]
	fn phase_offset_changes_pose() {
		let idle = QuadrupedIdle::default();
		let a = sample(&idle, QuadrupedIdle::graze_peak());
		let b = sample(&idle, QuadrupedIdle::graze_peak() + Idle::phase_from_entity_bits(7));

		assert_ne!(a.neck_nod, b.neck_nod);
	}
}
