//! Humanoid mapping for [`ThinkAgain`](crate::animations::ThinkAgain).

use character_rigs::authoring::{ArmAim, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::ThinkAgain;
use crate::Animation;

impl Animation<HumanoidV0Rig> for ThinkAgain {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		if self.gesture_amount(progress) < 1e-5 {
			return;
		}

		let mut pose = HumanoidPose::default();
		let arm = pose.arm_mut(self.side);
		arm.elbow_flexion += self.elbow_flexion(progress);
		arm.aim =
			Some(ArmAim { along: self.humerus_along(progress), roll: self.humerus_roll(progress) });
		let tilt = self.head_cock_tilt(progress);
		let nod = self.head_cock_nod(progress);
		pose.apply_neck_twisted(0.0, tilt * 0.65, nod * 0.35, 0.0, tilt * 0.35, nod * 0.65);

		rig.write_pose(&pose);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Vec3;
	use character_rigs::Side;

	#[derive(Debug, Clone, Copy, PartialEq)]
	struct ArmKeyPose {
		shoulder: Vec3,
		elbow: Vec3,
		tip: Vec3,
		humerus_dir: Vec3,
		forearm_dir: Vec3,
	}

	fn bone_name(prefix: &str, side: Side) -> String {
		format!("{prefix}.{}", side.suffix())
	}

	fn segment_tip(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		let Some(id) = rig.binding.definition.id(bone) else {
			panic!("missing bone {bone}");
		};
		let len = rig.binding.effective_rest.local[id.index()].translation.length();
		rig.character_point(bone) + rig.character_length(bone) * len
	}

	fn arm_key_pose(rig: &HumanoidV0Rig, side: Side) -> ArmKeyPose {
		let humerus = bone_name("humerus", side);
		let forearm = bone_name("forearm", side);
		let shoulder = rig.character_point(&humerus);
		let elbow = rig.character_point(&forearm);
		let tip = segment_tip(rig, &forearm);
		let humerus_dir = rig.character_length(&humerus);
		let forearm_dir = (tip - elbow).normalize_or_zero();
		ArmKeyPose { shoulder, elbow, tip, humerus_dir, forearm_dir }
	}

	fn forearm_tilt_from_positions(elbow: Vec3, tip: Vec3) -> f32 {
		let delta = tip - elbow;
		delta.x.atan2(delta.y).to_degrees()
	}

	fn tip_is_above_elbow(elbow: Vec3, tip: Vec3) -> bool {
		tip.y > elbow.y + 0.05
	}

	fn tip_is_inboard_of_elbow(elbow: Vec3, tip: Vec3) -> bool {
		tip.x.abs() < elbow.x.abs()
	}

	fn humerus_points_laterally_outward(rest: &ArmKeyPose, posed: &ArmKeyPose) -> bool {
		let rest_outward = rest.tip.x - rest.shoulder.x;
		let posed_outward = posed.elbow.x - posed.shoulder.x;
		rest_outward.signum() == posed_outward.signum()
	}

	/// Same side of the body midline (`x = 0`) as the shoulder.
	fn tip_stays_on_gesture_side(shoulder: Vec3, tip: Vec3) -> bool {
		if shoulder.x.abs() < 1e-4 {
			return true;
		}
		shoulder.x.signum() == tip.x.signum()
	}

	fn palm_inward_dot(rig: &HumanoidV0Rig, side: Side) -> f32 {
		let bone = bone_name("forearm", side);
		let local_palm = match side {
			Side::Right => Vec3::X,
			Side::Left => Vec3::NEG_X,
		};
		let palm = rig.rotation(&bone) * local_palm;
		let inward = Vec3::new(side.sign(), 0.0, 0.0);
		palm.dot(inward)
	}

	fn pose_with_roll(side: Side, roll: f32, progress: f32) -> HumanoidV0Rig {
		let clip = ThinkAgain::default().with_side(side);
		let mut rig = HumanoidV0Rig::for_clip_test();
		let mut pose = HumanoidPose::default();
		let arm = pose.arm_mut(side);
		arm.elbow_flexion = clip.elbow_flexion(progress);
		arm.aim = Some(ArmAim { along: clip.humerus_along(progress), roll });
		rig.write_pose(&pose);
		rig
	}

	fn think_hold() -> f32 {
		1.50
	}

	fn again_hold() -> f32 {
		3.40
	}

	fn posed_arm(side: Side, progress: f32) -> ArmKeyPose {
		let clip = ThinkAgain::default().with_side(side);
		let mut rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut rig, progress);
		arm_key_pose(&rig, side)
	}

	#[test]
	fn think_again_repeatability() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let mut a = HumanoidV0Rig::for_clip_test();
		let mut b = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut a, think_hold());
		clip.apply(&mut b, think_hold());
		for bone in ["humerus.R", "forearm.R"] {
			let ra = a.rotation(bone);
			let rb = b.rotation(bone);
			assert!((ra.x - rb.x).abs() < 1e-5, "{bone} x");
			assert!((ra.y - rb.y).abs() < 1e-5, "{bone} y");
			assert!((ra.z - rb.z).abs() < 1e-5, "{bone} z");
			assert!((ra.w - rb.w).abs() < 1e-5, "{bone} w");
		}
		Ok(())
	}

	#[test]
	fn think_again_entry_and_exit_are_continuous_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let mut entry = HumanoidV0Rig::for_clip_test();
		let mut exit = HumanoidV0Rig::for_clip_test();
		let rest_pose = entry.pose.clone();
		clip.apply_for(&mut entry, 0.0);
		clip.apply_for(&mut exit, crate::animations::think_again::DURATION);
		assert_eq!(entry.pose, rest_pose, "entry should not write bones");
		assert_eq!(exit.pose, rest_pose, "exit should not write bones");
		Ok(())
	}

	#[test]
	fn think_again_interruption_mid_clip_is_stable() -> anyhow::Result<()> {
		let clip = ThinkAgain::default();
		let mut once = HumanoidV0Rig::for_clip_test();
		let mut twice = HumanoidV0Rig::for_clip_test();
		clip.apply_for(&mut once, 1.40);
		clip.apply_for(&mut twice, 1.40);
		clip.apply_for(&mut twice, 1.40);
		for bone in ["humerus.R", "forearm.R"] {
			let delta = once.rotation(bone).angle_between(twice.rotation(bone));
			assert!(delta < 1e-3, "{bone} drifted {delta}");
		}
		Ok(())
	}

	#[test]
	fn think_again_humerus_points_laterally_outward() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let rest = arm_key_pose(&HumanoidV0Rig::for_clip_test(), side);
			let posed = posed_arm(side, think_hold());
			assert!(
				humerus_points_laterally_outward(&rest, &posed),
				"{side:?} humerus should abduct outboard, rest {rest:?} posed {posed:?}"
			);
			let elevation = posed.humerus_dir.y.atan2(posed.humerus_dir.x.abs()).to_degrees();
			assert!(
				(elevation + 5.0).abs() < 3.0,
				"{side:?} humerus should drop ~5°, elevation {elevation:.1}° dir {:?}",
				posed.humerus_dir
			);
		}
		Ok(())
	}

	#[test]
	fn think_again_humerus_barely_moves_between_key_poses() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let think = posed_arm(side, think_hold());
			let again = posed_arm(side, again_hold());
			let delta = think.humerus_dir.angle_between(again.humerus_dir).to_degrees();
			let elbow_shift = (think.elbow - again.elbow).length();
			assert!(delta < 3.0, "{side:?} humerus drift {delta:.1}°");
			assert!(elbow_shift < 0.02, "{side:?} elbow moved {elbow_shift:.3}");
		}
		Ok(())
	}

	#[test]
	fn think_again_think_pose_forearm_inboard() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let pose = posed_arm(side, think_hold());
			let tilt = forearm_tilt_from_positions(pose.elbow, pose.tip);
			let head_y = HumanoidV0Rig::for_clip_test().character_point("upper_neck").y + 0.12;
			assert!(tip_is_above_elbow(pose.elbow, pose.tip), "{side:?} tip above elbow {pose:?}");
			assert!(
				tip_is_inboard_of_elbow(pose.elbow, pose.tip),
				"{side:?} Think inboard |tip.x| < |elbow.x|, {pose:?}"
			);
			assert!(
				tilt.abs() > 35.0 && tilt.abs() < 55.0,
				"{side:?} Think tilt ~45°, got {tilt:.1}°"
			);
			assert!(
				pose.tip.y > pose.shoulder.y && pose.tip.y > head_y - 0.35,
				"{side:?} hand stays up beside the head after the humerus drop, tip {:?} head {head_y}",
				pose.tip
			);
			assert!(
				tip_stays_on_gesture_side(pose.shoulder, pose.tip),
				"{side:?} Think must stay on gesture side of midline, {pose:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn think_again_again_pose_forearm_inboard_fifteen() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let think = posed_arm(side, think_hold());
			let again = posed_arm(side, again_hold());
			let tilt = forearm_tilt_from_positions(again.elbow, again.tip);
			let think_tilt = forearm_tilt_from_positions(think.elbow, think.tip);
			assert!(
				tip_is_above_elbow(again.elbow, again.tip),
				"{side:?} tip above elbow {again:?}"
			);
			assert!(
				tip_is_inboard_of_elbow(again.elbow, again.tip),
				"{side:?} Again stays inboard |tip.x| < |elbow.x|, {again:?}"
			);
			assert!(
				tilt.abs() > 10.0 && tilt.abs() < 20.0,
				"{side:?} Again tilt ~+15° inboard, got {tilt:.1}°"
			);
			assert!(
				tilt.abs() + 8.0 < think_tilt.abs(),
				"{side:?} Again should be shallower than Think, think {think_tilt:.1}° again {tilt:.1}°"
			);
			assert!(
				tip_stays_on_gesture_side(again.shoulder, again.tip),
				"{side:?} Again must stay on gesture side of midline, {again:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn think_again_hold_pose_is_stable() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut early = HumanoidV0Rig::for_clip_test();
		let mut late = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut early, 1.20);
		clip.apply(&mut late, 1.80);
		let early_tip = segment_tip(&early, "forearm.R");
		let late_tip = segment_tip(&late, "forearm.R");
		assert!(
			(early_tip - late_tip).length() < 0.04,
			"Think hold drift {early_tip:?} vs {late_tip:?}"
		);

		let mut again_early = HumanoidV0Rig::for_clip_test();
		let mut again_late = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut again_early, 3.15);
		clip.apply(&mut again_late, 3.55);
		let again_a = segment_tip(&again_early, "forearm.R");
		let again_b = segment_tip(&again_late, "forearm.R");
		assert!((again_a - again_b).length() < 0.04, "Again hold drift {again_a:?} vs {again_b:?}");
		Ok(())
	}

	#[test]
	fn think_again_palm_faces_inward_both_sides() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let clip = ThinkAgain::default().with_side(side);
			let mut rig = HumanoidV0Rig::for_clip_test();
			clip.apply(&mut rig, think_hold());
			let dot = palm_inward_dot(&rig, side);
			assert!(dot > 0.2, "{side:?} palm should face inward, dot {dot}");
		}
		Ok(())
	}

	#[test]
	fn think_again_roll_is_not_mirrored_by_side_sign() -> anyhow::Result<()> {
		use std::f32::consts::PI;

		let progress = think_hold();
		let clip_right = ThinkAgain::default().with_side(Side::Right);
		let clip_left = ThinkAgain::default().with_side(Side::Left);
		assert_eq!(
			clip_right.humerus_roll(progress),
			clip_left.humerus_roll(progress),
			"roll is a shared constant, not multiplied by Side::sign"
		);

		for side in [Side::Right, Side::Left] {
			let tuned = pose_with_roll(side, clip_right.humerus_roll(progress), progress);
			let tuned_pose = arm_key_pose(&tuned, side);
			assert!(
				tuned_pose.tip.y > tuned_pose.elbow.y + 0.25,
				"{side:?} tuned roll keeps hand at head height"
			);
			assert!(palm_inward_dot(&tuned, side) > 0.2, "{side:?} tuned roll keeps palm inward");

			// The first draft used `π` together with inverted `humerus_along` (+X on right).
			// With corrected lateral aim, `π` still passes palm but folds to shoulder height.
			let legacy_pi = pose_with_roll(side, PI, progress);
			let legacy_pose = arm_key_pose(&legacy_pi, side);
			assert!(
				tuned_pose.tip.y > legacy_pose.tip.y + 0.15,
				"{side:?}: π is not the correct hinge with corrected aim, tuned {:?} vs π {:?}",
				tuned_pose.tip,
				legacy_pose.tip
			);
		}
		Ok(())
	}

	#[test]
	fn think_again_mirrors_for_left_side() -> anyhow::Result<()> {
		let right = posed_arm(Side::Right, think_hold());
		let left = posed_arm(Side::Left, think_hold());
		assert!((right.tip.x + left.tip.x).abs() < 0.06, "mirrored tips {right:?} {left:?}");
		assert!((right.tip.y - left.tip.y).abs() < 0.08, "matched height {right:?} {left:?}");
		Ok(())
	}

	#[test]
	fn think_again_head_cocks_toward_gesture_side() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			let rest = HumanoidV0Rig::for_clip_test().character_length("upper_neck");
			let think = posed_neck(side, think_hold());
			let again = posed_neck(side, again_hold());
			let toward = think.x - rest.x;
			assert!(
				toward.signum() == side.sign(),
				"{side:?} should cock toward the raised hand, rest {rest:?} think {think:?}"
			);
			assert!(toward.abs() > 0.04, "{side:?} cock too small, toward {toward}");
			assert!(
				again.x.abs() < think.x.abs() * 0.35,
				"{side:?} Again should un-cock, think {think:?} again {again:?}"
			);
		}
		Ok(())
	}

	fn posed_neck(side: Side, progress: f32) -> Vec3 {
		let clip = ThinkAgain::default().with_side(side);
		let mut rig = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut rig, progress);
		rig.character_length("upper_neck")
	}

	#[test]
	fn think_again_leaves_opposite_arm_at_rest() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut posed = HumanoidV0Rig::for_clip_test();
		clip.apply(&mut posed, think_hold());

		for bone in ["humerus.L", "forearm.L"] {
			assert!(posed.posed_angle(bone) < 1e-4, "opposite {bone} stays at rest");
		}
		Ok(())
	}

	#[test]
	fn think_again_tip_stays_on_gesture_side_of_midline() -> anyhow::Result<()> {
		for side in [Side::Right, Side::Left] {
			for progress in [think_hold(), again_hold()] {
				let pose = posed_arm(side, progress);
				assert!(
					tip_stays_on_gesture_side(pose.shoulder, pose.tip),
					"{side:?} at {progress} crossed midline: shoulder {:?} tip {:?}",
					pose.shoulder,
					pose.tip
				);
			}
		}
		Ok(())
	}

	#[test]
	fn think_again_samples_are_continuous() -> anyhow::Result<()> {
		let clip = ThinkAgain::default().with_side(Side::Right);
		let mut prev = segment_tip(&HumanoidV0Rig::for_clip_test(), "forearm.R");
		for i in 1..=256 {
			let t = i as f32 / 256.0 * crate::animations::think_again::DURATION;
			let mut rig = HumanoidV0Rig::for_clip_test();
			clip.apply(&mut rig, t);
			let tip = segment_tip(&rig, "forearm.R");
			if clip.gesture_amount(t) > 0.05
				&& clip.gesture_amount(
					(i - 1) as f32 / 256.0 * crate::animations::think_again::DURATION,
				) > 0.05
			{
				let jump = (tip - prev).length();
				assert!(jump < 0.28, "discontinuity at {t:.3}: {jump:.3} {prev:?} -> {tip:?}");
			}
			prev = tip;
		}
		Ok(())
	}
}
