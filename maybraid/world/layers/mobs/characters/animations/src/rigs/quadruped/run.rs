use character_rigs::authoring::QuadrupedPose;
use character_rigs::rigs::quadruped_v0::QuadrupedV0Rig;
use character_rigs::Side;

use crate::animations::{QuadrupedRun, QuadrupedRunPose};
use crate::rigs::quadruped::apply::{apply_neck_axes, apply_spine};
use crate::rigs::quadruped::gait::{
	apply_front_leg_at_strike, apply_hind_leg_at_strike, thigh_swing, KneeTuning, LegStrideTuning,
};
use crate::{Animation, Progress};

impl Animation<QuadrupedV0Rig> for QuadrupedRun {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		QuadrupedRunPose::from_run(self).apply_for(rig, progress)
	}
}

impl Animation<QuadrupedV0Rig> for QuadrupedRunPose {
	fn apply_for(&self, rig: &mut QuadrupedV0Rig, progress: f32) {
		let mut pose = QuadrupedPose::default();
		sample_run(self, progress, &mut pose);
		rig.write_pose(&pose);
	}
}

fn sample_run(run: &QuadrupedRunPose, progress: f32, pose: &mut QuadrupedPose) {
	let cycle = Progress(progress).cycle();
	let tuning = leg_tuning(run);

	// Diagonal trot: FL, HR, FR, HL.
	apply_front_leg_at_strike(pose, Side::Left, cycle, 0.0, tuning);
	apply_hind_leg_at_strike(pose, Side::Right, cycle, 0.25, tuning);
	apply_front_leg_at_strike(pose, Side::Right, cycle, 0.5, tuning);
	apply_hind_leg_at_strike(pose, Side::Left, cycle, 0.75, tuning);

	let spine_swing = thigh_swing(cycle) * run.spine_swing;
	let spine = run.spine_swing.max(1e-4);
	apply_spine(pose, spine_swing, -spine_swing * 0.5);
	apply_neck_axes(
		pose,
		-spine_swing * run.neck_swing / spine,
		-spine_swing * run.neck_bow / spine,
		-spine_swing * run.neck_pitch / spine,
	);
}

fn leg_tuning(run: &QuadrupedRunPose) -> LegStrideTuning {
	LegStrideTuning {
		shoulder_swing: run.shoulder_swing,
		shoulder_lift: run.shoulder_lift,
		hip_swing: run.hip_swing,
		hip_lift: run.hip_lift,
		stride: run.stride,
		knee: KneeTuning {
			knee_neutral: run.knee_neutral,
			knee_contracted: run.knee_contracted,
			knee_extended: run.knee_extended,
		},
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::*;

	fn tip(rig: &QuadrupedV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_run = QuadrupedV0Rig::imported();
			let mut from_pose = QuadrupedV0Rig::imported();
			QuadrupedRun::default().apply(&mut from_run, phase);
			QuadrupedRunPose::default().apply(&mut from_pose, phase);

			for name in from_run.animation_bone_names() {
				let run_rot = from_run.rotation(name);
				let pose_rot = from_pose.rotation(name);
				assert!(
					run_rot.dot(pose_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn quadruped_run_delegates_to_pose_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn quadruped_run_strides_the_left_front_thigh() {
		let rest = QuadrupedV0Rig::imported();
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedRunPose::default().apply(&mut rig, 0.0);
		assert!(
			rig.rotation("anterior_thigh.L").dot(rest.rotation("anterior_thigh.L")).abs() < 0.999,
			"stride leaves rest"
		);
	}

	#[test]
	fn quadruped_run_bows_and_rotates_the_neck() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedRunPose::default().apply(&mut rig, 0.0);

		let nod = tip(&rig, "neck");
		let yaw = rig.rotation("neck") * Vec3::Z;
		assert!(yaw.x.abs() > 0.02, "turn is yaw, got {yaw:?}");
		assert!(nod.x.abs() > 0.03, "tilt is lateral, got {nod:?}");
		assert!(nod.z.abs() > 0.03, "nod is sagittal, got {nod:?}");
	}

	#[test]
	fn quadruped_run_offsets_legs_across_stride() {
		let mut rig = QuadrupedV0Rig::imported();
		QuadrupedRunPose::default().apply(&mut rig, 0.0);
		assert!(
			rig.rotation("anterior_thigh.L").dot(rig.rotation("posterior_thigh.R")).abs() < 0.999,
			"diagonal pair should not share one rotation"
		);
	}
}
