//! Humanoid mapping for [`PowerStrafe`](crate::animations::PowerStrafe).

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::PowerStrafe;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for PowerStrafe {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let mut pose = HumanoidPose::default();
		let travel = self.travel_side();
		let trail = self.opposite_side();
		let fake = self.fake_amount(progress);
		let cut = self.cut_amount(progress).abs();

		apply_leg(
			&mut pose,
			travel,
			self.travel_femur(progress),
			self.travel_shin(progress),
			self.travel_abduction(progress),
		);
		apply_leg(
			&mut pose,
			trail,
			self.trail_femur(progress),
			self.trail_shin(progress),
			self.trail_abduction(progress),
		);

		pose.apply_spine_pitch(self.spine_pitch(progress));
		if fake + cut > f32::EPSILON {
			let side = self.spine_side(progress);
			pose.spine.side_bend[1] += side;
			pose.spine.side_bend[2] += side * 0.70;
			pose.spine.side_bend[3] += side * 0.45;
		}

		let hip = self.hip_lateral(progress);
		if hip.abs() > f32::EPSILON {
			pose.leg_mut(travel).pelvis_lateral += hip;
			pose.leg_mut(trail).pelvis_lateral += hip * 0.75;
		}

		let tilt = self.neck_tilt(progress);
		let nod = self.neck_nod(progress);
		if tilt.abs() + nod > f32::EPSILON {
			pose.apply_neck_twisted(0.0, tilt, nod, 0.0, tilt * 0.85, nod * 0.55);
		}

		if let Some(along) = self.hand_along(progress) {
			apply_hand_down(
				&mut pose,
				travel,
				along,
				self.hand_roll(progress),
				self.hand_elbow(progress),
				self.shoulder_lift_travel(progress),
			);
		} else if fake > f32::EPSILON {
			apply_fake_travel_arm(&mut pose, travel, fake, self.shoulder_lift_travel(progress));
		}
		if fake + cut > f32::EPSILON {
			apply_balance_arm(
				&mut pose,
				trail,
				self.balance_elbow(progress),
				self.shoulder_lift_trail(progress),
				fake,
				cut,
			);
		}

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

fn apply_leg(pose: &mut HumanoidPose, side: Side, femur: f32, shin: f32, abduct: f32) {
	pose.apply_leg(side, femur, shin);
	if abduct.abs() > f32::EPSILON {
		pose.leg_mut(side).hip_abduction += abduct;
	}
}

fn apply_hand_down(
	pose: &mut HumanoidPose,
	side: Side,
	along: Vec3,
	roll: f32,
	elbow: f32,
	shoulder_lift: f32,
) {
	let arm = pose.arm_mut(side);
	arm.elbow_flexion += elbow;
	arm.shoulder_forward += -0.16 * side.sign();
	arm.shoulder_lift += shoulder_lift;
	arm.aim = Some(ArmAim { along, roll });
}

fn apply_fake_travel_arm(pose: &mut HumanoidPose, side: Side, fake: f32, shoulder_lift: f32) {
	pose.apply_arm(side, 0.12 * side.sign() * fake, shoulder_lift, 0.0, 0.0, 0.28 * fake);
}

fn apply_balance_arm(
	pose: &mut HumanoidPose,
	side: Side,
	elbow: f32,
	shoulder_lift: f32,
	fake: f32,
	cut: f32,
) {
	let wind = 0.20 * fake + 0.10 * cut;
	pose.apply_arm(
		side,
		-0.20 * side.sign(),
		0.28 * side.sign() + shoulder_lift,
		0.16 * side.sign() + wind * side.sign(),
		0.0,
		elbow,
	);
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::power_strafe::{DROP_END, FAKE_PEAK, SLIDE_END};
	use crate::animations::transition::Transition;
	use crate::animations::Idle;
	use crate::animations::PowerStrafe;

	const REVIEW_SAMPLES: [f32; 8] = [0.0, 0.1, 0.2, 0.35, 0.5, 0.65, 0.8, 1.0];
	const TANZ_SECS: f32 = 0.38;
	const FPS: f32 = 60.0;

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

	/// Tanz `left_strafe` / `right_strafe` body-space travel at look yaw 0.
	fn body_travel_x(side: Side) -> f32 {
		match side {
			Side::Left => -1.0,
			Side::Right => 1.0,
		}
	}

	fn pelvis_center(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("lumbar")
	}

	fn head_point(rig: &HumanoidV0Rig) -> Vec3 {
		bone_tail(rig, "upper_neck")
	}

	fn peak_progress() -> f32 {
		(DROP_END + SLIDE_END) * 0.5
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
	fn start_near_rest_end_recovers() -> anyhow::Result<()> {
		let strafe = PowerStrafe::new(Side::Left);
		let rest = HumanoidV0Rig::for_clip_test();
		let mut start = HumanoidV0Rig::for_clip_test();
		let mut end = HumanoidV0Rig::for_clip_test();
		strafe.apply(&mut start, 0.0);
		strafe.apply(&mut end, 1.0);
		for name in ["femur.L", "femur.R", "forearm.L"] {
			assert!(
				start.rotation(name).dot(rest.rotation(name)).abs() > 1.0 - 1e-4,
				"{name} should stay near rest at start"
			);
		}
		assert!(
			end.posed_angle("femur.L") < strafe.travel_femur(peak_progress()) * 0.35,
			"end should recover toward rest"
		);
		Ok(())
	}

	#[test]
	fn peak_low_stance_and_hand_down_on_travel_side() -> anyhow::Result<()> {
		let strafe = PowerStrafe::new(Side::Right);
		let mut rig = HumanoidV0Rig::for_clip_test();
		strafe.apply(&mut rig, peak_progress());
		assert!(rig.posed_angle("femur.R") > 0.35, "travel hip folds");
		assert!(rig.posed_angle("shin.R") > 0.45, "travel knee bends");
		let down = bone_tail(&rig, forearm_bone(Side::Right));
		let up = bone_tail(&rig, forearm_bone(Side::Left));
		let foot_r = bone_tail(&rig, shin_bone(Side::Right));
		assert!(down.y < up.y - 0.12, "travel hand lower than balance, down={down:?} up={up:?}");
		assert!(down.x > 0.05, "travel hand lateral on +X for right strafe");
		assert!(
			(down.y - foot_r.y).abs() < 0.15,
			"travel hand should skim the foot height, down={down:?} foot={foot_r:?}"
		);
		Ok(())
	}

	#[test]
	fn left_and_right_mirror_hand_positions() -> anyhow::Result<()> {
		let progress = peak_progress();
		let mut right_rig = HumanoidV0Rig::for_clip_test();
		let mut left_rig = HumanoidV0Rig::for_clip_test();
		PowerStrafe::new(Side::Right).apply(&mut right_rig, progress);
		PowerStrafe::new(Side::Left).apply(&mut left_rig, progress);
		let right = bone_tail(&right_rig, forearm_bone(Side::Right));
		let left = bone_tail(&left_rig, forearm_bone(Side::Left));
		assert!((right.x + left.x).abs() < 0.1, "mirror X: {right:?} {left:?}");
		assert!((right.y - left.y).abs() < 0.08, "mirror Y: {right:?} {left:?}");
		assert!((right.z - left.z).abs() < 0.08, "mirror Z: {right:?} {left:?}");
		Ok(())
	}

	#[test]
	fn fake_leans_opposite_then_cut_leans_into_travel() -> anyhow::Result<()> {
		for side in [Side::Left, Side::Right] {
			let wish = body_travel_x(side);
			let (fake_rig, _) = sample(side, FAKE_PEAK);
			let head = head_point(&fake_rig);
			let pelvis = pelvis_center(&fake_rig);
			assert!(
				(head.x - pelvis.x).signum() == -wish,
				"fake head {head:?} should lean opposite wish {wish}, pelvis {pelvis:?}"
			);
			let (cut_rig, _) = sample(side, 0.5);
			let cut_head = head_point(&cut_rig);
			let cut_pelvis = pelvis_center(&cut_rig);
			assert!(
				(cut_head.x - cut_pelvis.x).signum() == wish,
				"cut head {cut_head:?} should lean into wish {wish}, pelvis {cut_pelvis:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn slide_uses_modest_armature_offset() -> anyhow::Result<()> {
		let strafe = PowerStrafe::new(Side::Left);
		let mut rig = HumanoidV0Rig::for_clip_test();
		let effects = strafe.apply(&mut rig, peak_progress());
		assert!(effects.0.translation.x < -0.04, "left strafe offsets −X");
		assert!(effects.0.translation.y < -0.01, "crouch drop");
		assert!(effects.0.translation.x.abs() < 0.2, "offset stays modest");
		Ok(())
	}

	#[test]
	fn armature_slide_and_travel_hand_match_strafe_travel() -> anyhow::Result<()> {
		let progress = peak_progress();
		for side in [Side::Left, Side::Right] {
			let wish_x = body_travel_x(side);
			let mut rig = HumanoidV0Rig::for_clip_test();
			let strafe = PowerStrafe::new(side);
			let effects = strafe.apply(&mut rig, progress);
			assert!(
				effects.0.translation.x.signum() == wish_x.signum(),
				"offset {:?} must match travel {wish_x}",
				effects.0.translation
			);
			let travel_hand = bone_tail(&rig, forearm_bone(side));
			let pelvis = pelvis_center(&rig);
			assert!(
				(travel_hand.x - pelvis.x).signum() == wish_x.signum(),
				"travel hand {travel_hand:?} vs pelvis {pelvis:?} for wish {wish_x}"
			);
			let balance_hand = bone_tail(&rig, forearm_bone(side.opposite()));
			assert!(
				travel_hand.y < balance_hand.y + 0.15,
				"travel hand {travel_hand:?} should not flare above balance {balance_hand:?}"
			);
		}
		Ok(())
	}

	#[test]
	fn left_and_right_pose_samples_mirror_in_character_space() -> anyhow::Result<()> {
		for progress in REVIEW_SAMPLES {
			let (right_rig, right_effects) = sample(Side::Right, progress);
			let (left_rig, left_effects) = sample(Side::Left, progress);
			let right_hand = bone_tail(&right_rig, forearm_bone(Side::Right));
			let left_hand = bone_tail(&left_rig, forearm_bone(Side::Left));
			assert!((right_hand.x + left_hand.x).abs() < 0.12, "hands at {progress}");
			assert!((right_hand.y - left_hand.y).abs() < 0.1, "hands at {progress}");
			assert!((right_hand.z - left_hand.z).abs() < 0.1, "hands at {progress}");
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
	fn review_tables_show_fake_hand_down_and_mirror() -> anyhow::Result<()> {
		let max_delta = max_offset_delta_60fps(&PowerStrafe::new(Side::Left));
		eprintln!(
			"max armature |Δ| at 60 Hz over Tanz {TANZ_SECS:.2}s ({:.0} frames): {max_delta:.6}",
			TANZ_SECS * FPS
		);
		assert!(max_delta < 0.05, "max 60 Hz Tanz-frame offset delta {max_delta}");

		for side in [Side::Left, Side::Right] {
			eprintln!("\n### Side {:?}\n", side);
			eprintln!(
				"| progress | hand.L | hand.R | foot.L | foot.R | pelvis | head | armature offset |"
			);
			eprintln!("| --- | --- | --- | --- | --- | --- | --- | --- |");
			for progress in REVIEW_SAMPLES {
				let (rig, effects) = sample(side, progress);
				let hand_l = bone_tail(&rig, forearm_bone(Side::Left));
				let hand_r = bone_tail(&rig, forearm_bone(Side::Right));
				let foot_l = bone_tail(&rig, shin_bone(Side::Left));
				let foot_r = bone_tail(&rig, shin_bone(Side::Right));
				let pelvis = pelvis_center(&rig);
				let head = head_point(&rig);
				eprintln!(
					"| {progress:.2} | {} | {} | {} | {} | {} | {} | {} |",
					fmt_vec(hand_l),
					fmt_vec(hand_r),
					fmt_vec(foot_l),
					fmt_vec(foot_r),
					fmt_vec(pelvis),
					fmt_vec(head),
					fmt_vec(effects.0.translation),
				);
			}
		}

		for side in [Side::Left, Side::Right] {
			let wish = body_travel_x(side);
			let (fake_rig, _) = sample(side, 0.1);
			let fake_head = head_point(&fake_rig);
			let fake_pelvis = pelvis_center(&fake_rig);
			assert!(
				(fake_head.x - fake_pelvis.x).signum() == -wish,
				"progress 0.1 fake lean opposite for {side:?}"
			);

			for progress in [0.35, 0.5] {
				let (rig, _) = sample(side, progress);
				let hand = bone_tail(&rig, forearm_bone(side));
				let foot = bone_tail(&rig, shin_bone(side));
				assert!(
					(hand.y - foot.y).abs() < 0.15,
					"progress {progress} travel hand {hand:?} vs foot {foot:?} on {side:?}"
				);
			}
		}
		Ok(())
	}

	/// Prints markdown-friendly pose tables for PR review (`cargo test … -- --ignored --nocapture`).
	#[test]
	#[ignore]
	fn power_strafe_pose_table_for_review() {
		let max_delta = max_offset_delta_60fps(&PowerStrafe::new(Side::Left));
		eprintln!("max armature |Δ| at 60 Hz over Tanz {TANZ_SECS:.2}s: {max_delta:.6}");
		for side in [Side::Left, Side::Right] {
			eprintln!("\n### Side {:?}\n", side);
			eprintln!(
				"| progress | hand.L | hand.R | foot.L | foot.R | pelvis | head | armature offset |"
			);
			eprintln!("| --- | --- | --- | --- | --- | --- | --- | --- |");
			for progress in REVIEW_SAMPLES {
				let (rig, effects) = sample(side, progress);
				eprintln!(
					"| {progress:.2} | {} | {} | {} | {} | {} | {} | {} |",
					fmt_vec(bone_tail(&rig, forearm_bone(Side::Left))),
					fmt_vec(bone_tail(&rig, forearm_bone(Side::Right))),
					fmt_vec(bone_tail(&rig, shin_bone(Side::Left))),
					fmt_vec(bone_tail(&rig, shin_bone(Side::Right))),
					fmt_vec(pelvis_center(&rig)),
					fmt_vec(head_point(&rig)),
					fmt_vec(effects.0.translation),
				);
			}
		}
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
		transition.apply(&mut rig, peak_progress(), 0.35);
		assert!(
			rig.posed_angle("femur.R") > 0.05
				&& rig.posed_angle("femur.R")
					< PowerStrafe::new(Side::Right).travel_femur(peak_progress()),
			"mid-blend between idle and full strafe"
		);
		Ok(())
	}
}
