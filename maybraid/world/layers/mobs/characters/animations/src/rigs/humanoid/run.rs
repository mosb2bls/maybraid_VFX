use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{Run, UprightRun};
use crate::rigs::humanoid::gait_knee::lerp_swing_knee;
use crate::rigs::humanoid::write_masks::{debug_assert_pose_within_mask, run_write_mask};
use crate::{Animation, Progress};

impl Run {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		UprightRun::from_run(self).sample_pose(progress)
	}
}

impl Animation<HumanoidV0Rig> for Run {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, run_write_mask(), "run");
		rig.apply_masked_pose(&pose, run_write_mask());
	}
}

impl UprightRun {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let phase = Progress(progress).cycle();
		let left_arm_swing = -arm_swing(phase);
		let right_arm_swing = arm_swing(phase + 0.5);

		apply_leg(&mut pose, Side::Left, phase, -1.0, self);
		apply_leg(&mut pose, Side::Right, phase, 1.0, self);
		// Both elbows share one flexion sign. Opposite arm_down values are a hang bias.
		apply_run_arm(&mut pose, Side::Left, left_arm_swing, phase, -self.arm_down, self);
		apply_run_arm(&mut pose, Side::Right, right_arm_swing, phase, self.arm_down, self);
		pose
	}
}

impl Animation<HumanoidV0Rig> for UprightRun {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, run_write_mask(), "upright_run");
		rig.apply_masked_pose(&pose, run_write_mask());
	}
}

fn apply_leg(pose: &mut HumanoidPose, side: Side, phase: f32, lift_sign: f32, run: &UprightRun) {
	let phase = if side == Side::Left { phase } else { phase + 0.5 };
	let swing = thigh_swing(phase);
	let leg = pose.leg_mut(side);
	// Pelvis yaw stays axial. Hip lift is a lateral hike; lift_sign is gait bias.
	leg.pelvis_turn = swing * run.hip_swing;
	leg.pelvis_lateral = hip_lift(swing, run.hip_lift) * lift_sign;
	leg.hip_flexion = swing * run.stride;
	leg.knee_flexion = knee_flex(phase, run) - run.knee_extended;
}

fn apply_run_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	humerus_flex: f32,
	run: &UprightRun,
) {
	pose.apply_arm(
		side,
		arm_swing_value * run.shoulder_swing,
		-shoulder_lift(arm_swing_value, run.shoulder_lift),
		arm_swing_value * run.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -1.0, run),
	);
}

fn thigh_swing(phase: f32) -> f32 {
	let p = phase.fract();
	if p < 0.5 {
		4.0 * p - 1.0
	} else {
		3.0 - 4.0 * p
	}
}

fn arm_swing(phase: f32) -> f32 {
	thigh_swing(phase) * 0.75
}

fn elbow_flex(arm_swing: f32, phase: f32, flex_sign: f32, run: &UprightRun) -> f32 {
	let pump = arm_swing.abs();
	let cycle = ((phase + arm_swing.signum() * 0.125) * std::f32::consts::PI * 4.0).sin().abs();
	flex_sign * (run.elbow_bend + pump * run.elbow_pump + cycle * run.elbow_cycle)
}

fn shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
	arm_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

fn knee_flex(leg_phase: f32, run: &UprightRun) -> f32 {
	let p = leg_phase.fract();
	if p < 0.5 {
		let t = p * 2.0;
		run.knee_neutral + (t * std::f32::consts::PI).sin() * (run.knee_extended - run.knee_neutral)
	} else {
		lerp_swing_knee(p, run.knee_extended, run.knee_contracted)
	}
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;

	use super::*;

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_run = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Run::default().apply(&mut from_run, phase);
			UprightRun::default().apply(&mut from_upright, phase);

			for name in from_run.animation_bone_names() {
				let run_rot = from_run.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					run_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn run_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn run_flexes_the_left_femur_sagittally() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("femur.L") > 0.2, "stride leaves rest");
	}

	#[test]
	fn run_right_leg_uses_half_cycle_phase_offset() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightRun::default().apply(&mut rig, 0.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
	}

	#[test]
	fn run_applies_knee_flex_to_shin() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.25);
		let extended = rig.posed_angle("shin.L");
		UprightRun::default().apply(&mut rig, 0.75);
		assert!(extended < 0.05, "expected a straight knee at this phase, got {extended}");
		assert!(rig.posed_angle("shin.L") > 0.5, "expected knee tuck later in the stride");
	}

	#[test]
	fn run_applies_elbow_bend_to_forearm() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("forearm.L") > 0.9, "expected elbow bend baseline");
	}

	#[test]
	fn run_applies_shoulder_and_hip_lift() {
		let mut rig = HumanoidV0Rig::imported();
		UprightRun::default().apply(&mut rig, 0.0);

		let shoulder = tip(&rig, "shoulder.L");
		let pelvis = tip(&rig, "pelvis.L");
		assert!(shoulder.x.abs() > 0.0, "expected shoulder bounce, got {shoulder:?}");
		assert!(pelvis.x.abs() > 0.0, "expected hip bounce, got {pelvis:?}");
	}
}
