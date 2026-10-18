//! Humanoid mapping for [`PowerStrafe`](crate::animations::PowerStrafe).

use bevy::prelude::Vec3;
use character_rigs::Side;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::PowerStrafe;
use crate::{Animation, Effects};

/// Lateral spine spread. Weights sum to 1 so [`StrafeKey::spine_side`] is the total bend.
const SPINE_SIDE_WEIGHTS: [f32; 4] = [0.40, 0.35, 0.15, 0.10];

impl Animation<HumanoidV0Rig> for PowerStrafe {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let key = self.key(progress);
		let sign = self.side.sign();
		let travel_x = self.travel_x();
		let push = self.push_side();
		let support = self.support_side();
		let mut pose = HumanoidPose::default();

		apply_leg(
			&mut pose,
			support,
			key.support_femur,
			key.support_knee,
			key.support_abduct * travel_x,
			key.support_rot * sign,
		);
		apply_leg(
			&mut pose,
			push,
			key.push_femur,
			key.push_knee,
			key.push_abduct * sign,
			key.push_rot * sign,
		);
		pose.apply_hip_fold(support, key.hip_fold_support);
		pose.apply_hip_fold(push, key.hip_fold_push);
		pose.apply_spine_pitch(key.spine_pitch);

		let side_bend = key.spine_side * sign;
		for (slot, weight) in pose.spine.side_bend.iter_mut().zip(SPINE_SIDE_WEIGHTS) {
			*slot += side_bend * weight;
		}
		pose.spine.add_turn(key.chest_turn * sign);
		pose.apply_neck_twisted(
			0.0,
			key.neck_tilt * sign,
			key.neck_nod,
			0.0,
			key.neck_tilt * sign * 0.55,
			key.neck_nod * 0.6,
		);

		apply_aimed_arm(
			&mut pose,
			push,
			mirror_along(key.fwd_along, sign),
			key.fwd_roll * sign,
			key.fwd_elbow,
			key.fwd_lift,
		);
		apply_aimed_arm(
			&mut pose,
			support,
			mirror_along(key.back_along, sign),
			key.back_roll * sign,
			key.back_elbow,
			key.back_lift,
		);

		rig.write_pose(&pose);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let offset = self.armature_translation(progress);
		if offset.length_squared() < 1e-8 {
			ArmatureOffset::IDENTITY
		} else {
			ArmatureOffset::from_translation(offset)
		}
	}
}

fn mirror_along(along: Vec3, sign: f32) -> Vec3 {
	let mirrored = Vec3::new(along.x * sign, along.y, along.z);
	let len = mirrored.length();
	if len < 1e-5 { Vec3::NEG_Y } else { mirrored / len }
}

fn apply_leg(pose: &mut HumanoidPose, side: Side, femur: f32, shin: f32, abduct: f32, rot: f32) {
	pose.apply_leg(side, femur, shin);
	pose.leg_mut(side).hip_abduction += abduct;
	pose.leg_mut(side).hip_rotation += rot;
}

fn apply_aimed_arm(
	pose: &mut HumanoidPose,
	side: Side,
	along: Vec3,
	roll: f32,
	elbow: f32,
	shoulder_lift: f32,
) {
	let arm = pose.arm_mut(side);
	arm.elbow_flexion += elbow;
	arm.shoulder_lift += shoulder_lift;
	arm.aim = Some(ArmAim { along, roll });
}

#[cfg(test)]
mod tests {
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::Idle;
	use crate::animations::PowerStrafe;
	use crate::animations::power_strafe::{
		GLIDE_END, GLIDE_HOLD, LOAD_END, PUSH_END, PUSHOFF_END, SHIN_TAIL_GROUND_Y, TUCK_END,
	};
	use crate::animations::transition::Transition;

	const REVIEW_SAMPLES: [f32; 6] = [LOAD_END, PUSH_END, GLIDE_HOLD, GLIDE_END, TUCK_END, 1.0];
	const TANZ_SECS: f32 = 0.38;
	const FPS: f32 = 60.0;
	const GROUND_SLACK: f32 = 0.22;
	const HAND_FLOOR_CLEARANCE: f32 = 0.45;

	fn bone_tail(rig: &HumanoidV0Rig, name: &str) -> Vec3 {
		rig.character_point(name) + rig.character_length(name)
	}

	fn forearm_bone(side: Side) -> &'static str {
		match side {
			Side::Left => "forearm.L",
			Side::Right => "forearm.R",
		}
	}

	fn shin_bone(side: Side) -> &'static str {
		match side {
			Side::Left => "shin.L",
			Side::Right => "shin.R",
		}
	}

	fn pelvis_center(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("lumbar")
	}

	fn head_point(rig: &HumanoidV0Rig) -> Vec3 {
		bone_tail(rig, "upper_neck")
	}

	fn sample(side: Side, progress: f32) -> (HumanoidV0Rig, ArmatureOffset) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = PowerStrafe::new(side).apply(&mut rig, progress);
		(rig, effects)
	}

	fn max_offset_delta_60fps(strafe: &PowerStrafe) -> f32 {
		let frames = (TANZ_SECS * FPS).round() as u32;
		let mut max_delta = 0.0f32;
		let mut prev = strafe.armature_translation(0.0);
		for step in 1..=frames {
			let progress = step as f32 / frames as f32;
			let next = strafe.armature_translation(progress);
			max_delta = max_delta.max((next - prev).length());
			prev = next;
		}
		max_delta
	}

	fn fmt_vec(v: Vec3) -> String {
		format!("({:.3}, {:.3}, {:.3})", v.x, v.y, v.z)
	}

	fn world_of(point: Vec3, offset: Vec3) -> Vec3 {
		point + offset
	}

	#[test]
	fn spine_side_weights_sum_to_one() {
		let sum: f32 = SPINE_SIDE_WEIGHTS.iter().sum();
		assert!((sum - 1.0).abs() < 1e-6, "spine side weights {SPINE_SIDE_WEIGHTS:?} sum {sum}");
	}

	#[test]
	fn repeatability_same_progress_same_pose() -> anyhow::Result<()> {
		let strafe = PowerStrafe::new(Side::Right);
		let progress = 0.42;
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		strafe.apply(&mut once, progress);
		strafe.apply(&mut twice, progress);
		strafe.apply(&mut twice, progress);
		for name in once.animation_bone_names() {
			assert!(
				once.rotation(name).dot(twice.rotation(name)).abs() > 1.0 - 1e-5,
				"drift on {name}"
			);
		}
		Ok(())
	}

	#[test]
	fn start_and_end_are_compact_ready() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let (start, start_fx) = sample(side, 0.0);
			let (end, end_fx) = sample(side, 1.0);
			assert!(start_fx.0.translation.length_squared() < 1e-10, "start offset");
			assert!(end_fx.0.translation.length_squared() < 1e-10, "end offset");
			for name in ["femur.L", "femur.R", "shin.L", "shin.R"] {
				assert!(
					start.posed_angle(name) < 0.45,
					"{name} should stay compact at start, {}",
					start.posed_angle(name)
				);
				assert!(
					end.posed_angle(name) < 0.45,
					"{name} should recover compact, {}",
					end.posed_angle(name)
				);
			}
			for hand in [forearm_bone(Side::Left), forearm_bone(Side::Right)] {
				let y = bone_tail(&end, hand).y;
				assert!(y > -0.05, "{hand} should stay off the floor at end, y={y}");
				let x = bone_tail(&end, hand).x.abs();
				assert!(x < 1.55, "{hand} should not recover to a T-pose reach, |x|={x}");
			}
		}
		Ok(())
	}

	fn assert_glide_silhouette(side: Side, progress: f32) {
		let strafe = PowerStrafe::new(side);
		let (rig, effects) = sample(side, progress);
		let offset = effects.0.translation;
		let wish = strafe.travel_x();
		let support = world_of(bone_tail(&rig, shin_bone(strafe.support_side())), offset);
		let push = world_of(bone_tail(&rig, shin_bone(strafe.push_side())), offset);
		let support_hip = world_of(
			rig.character_point(match strafe.support_side() {
				Side::Left => "femur.L",
				Side::Right => "femur.R",
			}),
			offset,
		);
		let support_knee = world_of(
			rig.character_point(match strafe.support_side() {
				Side::Left => "shin.L",
				Side::Right => "shin.R",
			}),
			offset,
		);
		let fwd_hand = world_of(bone_tail(&rig, forearm_bone(strafe.push_side())), offset);
		let back_hand = world_of(bone_tail(&rig, forearm_bone(strafe.support_side())), offset);
		let pelvis = world_of(pelvis_center(&rig), offset);
		let head = world_of(head_point(&rig), offset);

		assert!(
			(support.y - SHIN_TAIL_GROUND_Y).abs() < GROUND_SLACK,
			"support foot should contact shin-tail ground at {progress} on {side:?}: {support:?}"
		);
		assert!(
			push.y > support.y + 0.06,
			"pushing foot should unload/lift at {progress} on {side:?}: push {push:?} support {support:?}"
		);
		assert!(
			(push.x - support.x) * -wish > 0.45,
			"pushing leg should extend opposite travel at {progress}: push {push:?} support {support:?} wish {wish}"
		);
		assert!(
			(support.x - support_hip.x).abs() < 0.28,
			"support foot under hip at {progress}: foot {support:?} hip {support_hip:?}"
		);
		assert!(
			(support_knee.x - support.x).abs() < 0.22,
			"knee tracks over support foot at {progress}: knee {support_knee:?} foot {support:?}"
		);
		assert!(
			(support_hip.x - support.x).abs() < 0.28,
			"hips should sit over the support foot at {progress}: hip {support_hip:?} support {support:?} pelvis {pelvis:?}"
		);
		assert!(
			support.x * wish > 0.05,
			"support foot should be on the travel side at {progress}: {support:?} wish {wish}"
		);
		assert!(
			(head.x - pelvis.x).abs() < 0.80,
			"readable lateral lean at {progress}: head {head:?} pelvis {pelvis:?}"
		);
		assert!(
			head.z > pelvis.z + 0.15,
			"torso/gaze stay generally forward at {progress}: head {head:?} pelvis {pelvis:?}"
		);
		for (name, hand) in [("fwd", fwd_hand), ("back", back_hand)] {
			assert!(
				hand.y > SHIN_TAIL_GROUND_Y + HAND_FLOOR_CLEARANCE,
				"{name} hand must stay off the floor at {progress}: {hand:?}"
			);
		}
		assert!(
			fwd_hand.y < 1.45,
			"forward hand should not go overhead at {progress}: {fwd_hand:?}"
		);
		assert!(offset.y < -0.12, "glide drop at {progress}, {offset:?}");
		assert!(offset.y > -0.40, "drop stays in the skating band at {progress}, {offset:?}");
		assert!(
			offset.x.abs() > 0.18 && offset.x.abs() < 0.28,
			"glide visual slide stays a follow-through, not capsule travel, {offset:?}"
		);
		assert!(
			offset.x * wish > 0.0,
			"visual slide should match travel at {progress}: {offset:?} wish {wish}"
		);
	}

	#[test]
	fn glide_hold_is_a_skating_silhouette() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			for progress in [PUSHOFF_END, GLIDE_HOLD, GLIDE_END] {
				assert_glide_silhouette(side, progress);
			}
		}
		Ok(())
	}

	#[test]
	fn push_phase_keeps_the_pushing_foot_near_the_ground() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let strafe = PowerStrafe::new(side);
			let (rig, effects) = sample(side, PUSH_END);
			let offset = effects.0.translation;
			let push = world_of(bone_tail(&rig, shin_bone(strafe.push_side())), offset);
			assert!(
				(push.y - SHIN_TAIL_GROUND_Y).abs() < 0.35,
				"pushing foot still near the ground at push-end on {side:?}: {push:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn left_and_right_pose_samples_mirror_in_character_space() -> anyhow::Result<()> {
		for progress in REVIEW_SAMPLES {
			let (right_rig, right_effects) = sample(Side::Right, progress);
			let (left_rig, left_effects) = sample(Side::Left, progress);
			for (right_name, left_name) in [
				("forearm.R", "forearm.L"),
				("forearm.L", "forearm.R"),
				("shin.R", "shin.L"),
				("shin.L", "shin.R"),
				("humerus.R", "humerus.L"),
				("humerus.L", "humerus.R"),
			] {
				let right = bone_tail(&right_rig, right_name);
				let left = bone_tail(&left_rig, left_name);
				assert!(
					(right.x + left.x).abs() < 0.12,
					"{right_name}/{left_name} mirror X at {progress}: {right:?} {left:?}"
				);
				assert!(
					(right.y - left.y).abs() < 0.1,
					"{right_name}/{left_name} mirror Y at {progress}: {right:?} {left:?}"
				);
				assert!(
					(right.z - left.z).abs() < 0.1,
					"{right_name}/{left_name} mirror Z at {progress}: {right:?} {left:?}"
				);
			}
			let right_head = head_point(&right_rig);
			let left_head = head_point(&left_rig);
			assert!((right_head.x + left_head.x).abs() < 0.12, "heads at {progress}");
			assert!((right_head.y - left_head.y).abs() < 0.1, "heads at {progress}");
			assert!(
				(right_effects.0.translation.x + left_effects.0.translation.x).abs() < 1e-5,
				"offsets at {progress}"
			);
			assert!(
				(right_effects.0.translation.y - left_effects.0.translation.y).abs() < 1e-5,
				"offsets at {progress}"
			);
		}
		Ok(())
	}

	#[test]
	fn aims_blend_continuously_without_threshold_pops() -> anyhow::Result<()> {
		let step = 1.0 / (FPS * TANZ_SECS);
		let mut prev_fwd: Option<Vec3> = None;
		let mut prev_back: Option<Vec3> = None;
		let mut t: f32 = 0.0;
		while t <= 1.0 + 1e-6 {
			let (rig, _) = sample(Side::Left, t.min(1.0));
			let fwd = bone_tail(&rig, "forearm.L");
			let back = bone_tail(&rig, "forearm.R");
			if let (Some(pf), Some(pb)) = (prev_fwd, prev_back) {
				assert!(
					(fwd - pf).length() < 0.62,
					"forward wrist jump at {t}: {} -> {}",
					fmt_vec(pf),
					fmt_vec(fwd)
				);
				assert!(
					(back - pb).length() < 0.62,
					"back wrist jump at {t}: {} -> {}",
					fmt_vec(pb),
					fmt_vec(back)
				);
			}
			prev_fwd = Some(fwd);
			prev_back = Some(back);
			t += step;
		}
		Ok(())
	}

	#[test]
	fn joint_discontinuities_stay_bounded_at_60hz() -> anyhow::Result<()> {
		let step = 1.0 / (FPS * TANZ_SECS);
		let bones = ["shin.L", "shin.R", "forearm.L", "forearm.R", "femur.L", "femur.R"];
		let mut prev: Option<HumanoidV0Rig> = None;
		let mut t: f32 = 0.0;
		let mut max_delta = 0.0f32;
		while t <= 1.0 + 1e-6 {
			let (rig, _) = sample(Side::Left, t.min(1.0));
			if let Some(prev_rig) = prev.as_ref() {
				for name in bones {
					let delta = (bone_tail(&rig, name) - bone_tail(prev_rig, name)).length();
					max_delta = max_delta.max(delta);
					assert!(
						delta < 0.62,
						"{name} jumped {delta} at progress {t} (60 Hz Tanz frame)"
					);
				}
			}
			prev = Some(rig);
			t += step;
		}
		eprintln!("max 60 Hz joint |Δ| over Tanz {TANZ_SECS:.2}s: {max_delta:.6}");
		Ok(())
	}

	#[test]
	fn review_tables_show_skate_glide_and_mirror() -> anyhow::Result<()> {
		let max_delta = max_offset_delta_60fps(&PowerStrafe::new(Side::Left));
		eprintln!(
			"max armature |Δ| at 60 Hz over Tanz {TANZ_SECS:.2}s ({:.0} frames): {max_delta:.6}",
			TANZ_SECS * FPS
		);
		assert!(max_delta < 0.12, "max 60 Hz Tanz-frame offset delta {max_delta}");

		for side in [Side::Left, Side::Right] {
			eprintln!("\n### Side {:?} (world = pose + ArmatureOffset)\n", side);
			eprintln!(
				"| progress | hand.L | hand.R | foot.L | foot.R | pelvis | head | armature offset |"
			);
			eprintln!("| --- | --- | --- | --- | --- | --- | --- | --- |");
			for progress in REVIEW_SAMPLES {
				let (rig, effects) = sample(side, progress);
				let offset = effects.0.translation;
				eprintln!(
					"| {progress:.2} | {} | {} | {} | {} | {} | {} | {} |",
					fmt_vec(world_of(bone_tail(&rig, forearm_bone(Side::Left)), offset)),
					fmt_vec(world_of(bone_tail(&rig, forearm_bone(Side::Right)), offset)),
					fmt_vec(world_of(bone_tail(&rig, shin_bone(Side::Left)), offset)),
					fmt_vec(world_of(bone_tail(&rig, shin_bone(Side::Right)), offset)),
					fmt_vec(world_of(pelvis_center(&rig), offset)),
					fmt_vec(world_of(head_point(&rig), offset)),
					fmt_vec(offset),
				);
			}
		}
		Ok(())
	}

	#[test]
	fn interruption_blends_from_visible_pose() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Idle::default().apply(&mut rig, 0.0);
		let from_pose = rig.pose.clone();
		let transition = Transition::from_visible(
			PowerStrafe::new(Side::Right),
			from_pose,
			ArmatureOffset::IDENTITY,
		);
		transition.apply(&mut rig, GLIDE_HOLD, 0.35);
		let full = PowerStrafe::new(Side::Right).key(GLIDE_HOLD).support_knee;
		assert!(
			rig.posed_angle("shin.L") > 0.05 && rig.posed_angle("shin.L") < full,
			"mid-blend between idle and skating support knee"
		);
		Ok(())
	}
}
