use character_rigs::authoring::HumanoidPose;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{UprightWalk, Walk};
use crate::rigs::humanoid::gait_knee::lerp_swing_knee;
use crate::rigs::humanoid::write_masks::{debug_assert_pose_within_mask, walk_write_mask};
use crate::{Animation, Progress};

impl Walk {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		UprightWalk::from_walk(self).sample_pose(progress)
	}
}

impl Animation<HumanoidV0Rig> for Walk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, walk_write_mask(), "walk");
		rig.apply_masked_pose(&pose, walk_write_mask());
	}
}

impl UprightWalk {
	/// Authored semantic pose at `progress`. Rest is applied later by the rig.
	pub fn sample_pose(&self, progress: f32) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		sample_walk(self, progress, &mut pose);
		pose
	}
}

impl Animation<HumanoidV0Rig> for UprightWalk {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let pose = self.sample_pose(progress);
		debug_assert_pose_within_mask(&pose, walk_write_mask(), "upright_walk");
		rig.apply_masked_pose(&pose, walk_write_mask());
	}
}

fn sample_walk(walk: &UprightWalk, progress: f32, pose: &mut HumanoidPose) {
	let phase = Progress(progress).cycle();
	let left_arm_swing = -arm_swing(phase);
	let right_arm_swing = arm_swing(phase + 0.5);

	pose.apply_root(walk.torso_lean);
	apply_leg(pose, Side::Left, phase, -1.0, walk);
	apply_leg(pose, Side::Right, phase, 1.0, walk);
	// Both elbows share one flexion sign. The opposite arm_down values are a
	// hang bias, not a mirror of the joint frame.
	apply_walk_arm(pose, Side::Left, left_arm_swing, phase, -walk.arm_down, walk);
	apply_walk_arm(pose, Side::Right, right_arm_swing, phase, walk.arm_down, walk);
}

fn apply_leg(pose: &mut HumanoidPose, side: Side, phase: f32, lift_sign: f32, walk: &UprightWalk) {
	let phase = if side == Side::Left { phase } else { phase + side.phase_offset() };
	let swing = thigh_swing(phase);
	let leg = pose.leg_mut(side);
	// Pelvis yaw stays axial. Hip lift is a lateral hike, with opposite signs so
	// the stance side rises. Those signs are gait bias, not axis mirrors.
	leg.pelvis_turn = swing * walk.hip_swing * lift_sign;
	leg.pelvis_lateral = hip_lift(swing, walk.hip_lift) * lift_sign;
	leg.hip_flexion = swing * walk.stride;
	leg.hip_abduction = -swing * walk.femur_medial_counter * lift_sign;
	leg.knee_flexion = knee_flex(phase, walk);
}

fn apply_walk_arm(
	pose: &mut HumanoidPose,
	side: Side,
	arm_swing_value: f32,
	phase: f32,
	humerus_flex: f32,
	walk: &UprightWalk,
) {
	pose.apply_arm(
		side,
		arm_swing_value * walk.shoulder_swing,
		-shoulder_lift(arm_swing_value, walk.shoulder_lift),
		arm_swing_value * walk.humerus_swing_scale,
		humerus_flex,
		elbow_flex(arm_swing_value, phase, -1.0, walk),
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

fn elbow_flex(arm_swing: f32, phase: f32, flex_sign: f32, walk: &UprightWalk) -> f32 {
	let pump = arm_swing.abs();
	let cycle = ((phase + arm_swing.signum() * 0.125) * std::f32::consts::PI * 4.0).sin().abs();
	flex_sign * (walk.elbow_bend + pump * walk.elbow_pump + cycle * walk.elbow_cycle)
}

fn shoulder_lift(arm_swing: f32, amplitude: f32) -> f32 {
	arm_swing * amplitude
}

fn hip_lift(leg_swing: f32, amplitude: f32) -> f32 {
	leg_swing * amplitude
}

/// Soft knee on stance; shared swing envelope peaks mid-stride for toe clearance.
fn knee_flex(leg_phase: f32, walk: &UprightWalk) -> f32 {
	lerp_swing_knee(leg_phase, walk.knee_stance_bend, walk.knee_swing_bend)
}

#[cfg(test)]
mod tests {
	use bevy::prelude::*;
	use character_rigs::authoring::resolve_humanoid;
	use std::hint::black_box;
	use std::time::Instant;

	use super::*;
	use crate::animations::{Idle, Run};
	use crate::rigs::humanoid::write_masks::{idle_write_mask, run_write_mask, walk_write_mask};

	fn tip(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.rotation(name) * Vec3::Y
	}

	fn assert_pose_matches_at_phases(phases: &[f32]) {
		for &phase in phases {
			let mut from_walk = HumanoidV0Rig::imported();
			let mut from_upright = HumanoidV0Rig::imported();
			Walk::default().apply(&mut from_walk, phase);
			UprightWalk::default().apply(&mut from_upright, phase);

			for name in from_walk.animation_bone_names() {
				let walk_rot = from_walk.rotation(name);
				let upright_rot = from_upright.rotation(name);
				assert!(
					walk_rot.dot(upright_rot).abs() > 1.0 - 1e-5,
					"rotation mismatch on {name} at {phase}"
				);
			}
		}
	}

	#[test]
	fn walk_delegates_to_upright_default() {
		assert_pose_matches_at_phases(&[0.0, 0.25, 0.5, 0.75]);
	}

	#[test]
	fn walk_animates_femur_in_the_sagittal_plane() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("femur.L") > 0.05, "stride leaves rest");
	}

	#[test]
	fn walk_legs_are_half_cycle_out_of_phase() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		UprightWalk::default().apply(&mut rig, 0.0);

		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.05, "legs are out of phase, L={left:?} R={right:?}");
	}

	#[test]
	fn walk_stride_is_smaller_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.0);
		Run::default().apply(&mut run_rig, 0.0);

		assert!(
			walk_rig.posed_angle("femur.L") < run_rig.posed_angle("femur.L"),
			"walk stride should be smaller than run"
		);
	}

	#[test]
	fn walk_keeps_knee_closer_to_extended_than_run() {
		let mut walk_rig = HumanoidV0Rig::imported();
		let mut run_rig = HumanoidV0Rig::imported();
		Walk::default().apply(&mut walk_rig, 0.75);
		Run::default().apply(&mut run_rig, 0.75);

		assert!(
			walk_rig.posed_angle("shin.L") < run_rig.posed_angle("shin.L"),
			"walk knee should stay closer to rest than run"
		);
	}

	#[test]
	fn walk_applies_forward_torso_lean() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		let root = tip(&rig, "root");
		assert!(root.z > 0.05, "forward lean goes to +Z, got {root:?}");
		assert!(root.x.abs() < 1e-3, "lean must not yaw, got {root:?}");
	}

	#[test]
	fn walk_femur_counters_hip_swing_out() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("pelvis.L") > 0.0, "pelvis yaws");
		assert!(rig.posed_angle("femur.L") > 0.0, "femur strides");
	}

	#[test]
	fn walk_stance_leg_has_soft_knee_bend() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(rig.posed_angle("shin.L") > 0.0, "stance knee flexes");
	}

	#[test]
	fn walk_knee_flex_is_continuous_across_stride() {
		use crate::rigs::humanoid::gait_knee::SWING_KNEE_LIFT_SPAN;

		let walk = UprightWalk::default();
		let samples = 120;
		let max_step = (walk.knee_swing_bend - walk.knee_stance_bend) * std::f32::consts::PI
			/ (SWING_KNEE_LIFT_SPAN * samples as f32)
			+ 1e-4;
		let mut prev = knee_flex(0.0, &walk);
		for i in 1..=samples {
			let phase = i as f32 / samples as f32;
			let flex = knee_flex(phase, &walk);
			assert!((flex - prev).abs() < max_step, "knee snap at phase {phase}: {prev} -> {flex}");
			prev = flex;
		}
	}

	#[test]
	fn walk_knee_peak_precedes_late_swing_contact() {
		use crate::rigs::humanoid::gait_knee::{SWING_KNEE_LIFT_SPAN, SWING_KNEE_LIFT_START};

		let walk = UprightWalk::default();
		let expected_peak = SWING_KNEE_LIFT_START + SWING_KNEE_LIFT_SPAN * 0.5;
		let samples = 120;
		let mut peak_phase = 0.0;
		let mut peak_flex = knee_flex(0.0, &walk);
		for i in 1..=samples {
			let phase = i as f32 / samples as f32;
			let flex = knee_flex(phase, &walk);
			if flex > peak_flex {
				peak_flex = flex;
				peak_phase = phase;
			}
		}

		assert!(
			(peak_phase - expected_peak).abs() < 0.02,
			"expected peak near {expected_peak}, got {peak_phase}"
		);
		assert!(peak_phase < 0.75, "knee peak should precede late-swing contact");

		let mut rig = HumanoidV0Rig::for_clip_test();
		Walk::default().apply(&mut rig, peak_phase);
		assert!(
			rig.posed_angle("shin.L") > walk.knee_stance_bend + 0.4,
			"swing knee should flex at peak"
		);

		let mut late_neutral = None;
		for i in 0..=samples {
			let phase = i as f32 / samples as f32;
			if phase < 0.5 {
				continue;
			}
			let mut sample = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut sample, phase);
			if sample.character_length("femur.L").z.abs() < 0.02 {
				late_neutral = Some(phase);
				break;
			}
		}
		let contact = late_neutral.expect("femur should recross neutral late in stride");
		assert!(
			peak_phase < contact - 0.05,
			"peak knee {peak_phase} should clear before contact at {contact}"
		);
	}

	#[test]
	fn walk_knee_lift_envelope_matches_both_legs() {
		let walk = UprightWalk::default();
		for (global, shin) in [(0.60, "shin.L"), (0.10, "shin.R")] {
			let mut rig = HumanoidV0Rig::for_clip_test();
			Walk::default().apply(&mut rig, global);
			assert!(
				(rig.posed_angle(shin) - walk.knee_swing_bend).abs() < 0.05,
				"{shin} at global {global}"
			);
		}
	}

	#[test]
	fn walk_vertical_bob_comes_mostly_from_hips() {
		let mut rig = HumanoidV0Rig::imported();
		UprightWalk::default().apply(&mut rig, 0.0);

		assert!(
			rig.posed_angle("pelvis.L") > rig.posed_angle("shoulder.L"),
			"vertical bob comes mostly from hips"
		);
	}

	fn assert_masked_matches_full_resolve(
		pose: &character_rigs::authoring::HumanoidPose,
		mask: u32,
	) {
		let mut full = HumanoidV0Rig::imported();
		resolve_humanoid(pose, &full.binding, &mut full.pose);
		let mut masked = HumanoidV0Rig::imported();
		masked.apply_masked_pose(pose, mask);
		assert_eq!(full.pose.local, masked.pose.local, "masked resolve must match full");
	}

	#[test]
	fn cyclic_locomotion_masks_match_full_resolve() {
		let phases = [0.0, 0.11, 0.37, 0.62, 0.93];
		for &phase in &phases {
			assert_masked_matches_full_resolve(
				&Idle::default().sample_pose(phase),
				idle_write_mask(),
			);
			assert_masked_matches_full_resolve(
				&Walk::default().sample_pose(phase),
				walk_write_mask(),
			);
			assert_masked_matches_full_resolve(
				&Run::default().sample_pose(phase),
				run_write_mask(),
			);
		}
	}

	fn bench_locomotion_apply(use_masked: bool) -> Vec<u128> {
		const FRAMES: u32 = 2_000;
		const CHARACTERS: u32 = 32;
		const RUNS: u32 = 5;
		let clips: [(&str, f32); 3] = [("idle", 0.0), ("walk", 0.25), ("run", 0.5)];

		let mut rigs: Vec<HumanoidV0Rig> =
			(0..CHARACTERS).map(|_| HumanoidV0Rig::imported()).collect();

		let mut run_ns: Vec<u128> = Vec::with_capacity(RUNS as usize);
		for _ in 0..RUNS {
			let start = Instant::now();
			for frame in 0..FRAMES {
				let frame = black_box(frame);
				for (index, rig) in rigs.iter_mut().enumerate() {
					let progress =
						black_box((frame as f32 * 0.013 + (index as f32 * 0.07)).rem_euclid(1.0));
					let clip = clips[index as usize % clips.len()].0;
					match clip {
						"idle" => {
							let pose = Idle::default().sample_pose(progress);
							if use_masked {
								rig.apply_masked_pose(&pose, idle_write_mask());
							} else {
								rig.write_pose(&pose);
							}
						}
						"walk" => {
							let pose = Walk::default().sample_pose(progress);
							if use_masked {
								rig.apply_masked_pose(&pose, walk_write_mask());
							} else {
								rig.write_pose(&pose);
							}
						}
						"run" => {
							let pose = Run::default().sample_pose(progress);
							if use_masked {
								rig.apply_masked_pose(&pose, run_write_mask());
							} else {
								rig.write_pose(&pose);
							}
						}
						_ => {}
					}
					black_box(&rig.pose);
				}
			}
			let samples = FRAMES as u64 * CHARACTERS as u64;
			run_ns.push(start.elapsed().as_nanos() / samples as u128);
		}

		run_ns.sort_unstable();
		run_ns
	}

	fn report_locomotion_bench(label: &str, run_ns: &[u128]) {
		let min = *run_ns.first().expect("run");
		let median = run_ns[run_ns.len() / 2];
		let mean = run_ns.iter().sum::<u128>() / run_ns.len() as u128;
		let spread = run_ns.last().expect("run") - min;
		eprintln!(
			"locomotion_resolve_microbench {label}: runs={runs:?} min={min} median={median} mean={mean} spread={spread} ns/sample",
			runs = run_ns,
		);
	}

	/// `cargo test -p character-animations locomotion_resolve_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn locomotion_resolve_microbench() {
		eprintln!(
			"32 humanoid characters, idle/walk/run sample_pose + resolve, real Animation apply path"
		);
		report_locomotion_bench("legacy write_pose", &bench_locomotion_apply(false));
		report_locomotion_bench("masked partial resolve", &bench_locomotion_apply(true));
	}
}
