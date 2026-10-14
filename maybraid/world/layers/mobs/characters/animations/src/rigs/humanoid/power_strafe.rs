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
		let amount = self.stance_amount(progress);

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
		if amount > f32::EPSILON {
			pose.spine.side_bend[1] += self.spine_side(progress);
			pose.spine.side_bend[2] += self.spine_side(progress) * 0.65;
		}

		apply_hand_down(
			&mut pose,
			travel,
			self.hand_along(progress),
			self.hand_roll(progress),
			self.hand_elbow(progress),
		);
		apply_balance_arm(&mut pose, trail, self.balance_elbow(progress));

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

fn apply_hand_down(pose: &mut HumanoidPose, side: Side, along: Vec3, roll: f32, elbow: f32) {
	let arm = pose.arm_mut(side);
	arm.elbow_flexion += elbow;
	arm.shoulder_forward += -0.12 * side.sign();
	arm.aim = Some(ArmAim { along, roll });
}

fn apply_balance_arm(pose: &mut HumanoidPose, side: Side, elbow: f32) {
	pose.apply_arm(side, -0.18 * side.sign(), 0.22, 0.12 * side.sign(), 0.0, elbow);
}

#[cfg(test)]
mod tests {
	use bevy::prelude::Vec3;
	use character_rigs::authoring::ArmatureOffset;
	use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

	use super::*;
	use crate::animations::power_strafe::{DROP_END, SLIDE_END};
	use crate::animations::transition::Transition;
	use crate::animations::Idle;
	use crate::animations::PowerStrafe;

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

	fn peak_progress() -> f32 {
		(DROP_END + SLIDE_END) * 0.5
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
		assert!(down.y < up.y - 0.12, "travel hand lower than balance, down={down:?} up={up:?}");
		assert!(down.x > 0.05, "travel hand lateral on +X for right strafe");
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
	fn unwritten_neck_stays_at_rest() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let rest = HumanoidV0Rig::for_clip_test();
		PowerStrafe::default().apply(&mut rig, peak_progress());
		for name in ["lower_neck", "upper_neck"] {
			assert!(
				rig.rotation(name).dot(rest.rotation(name)).abs() > 1.0 - 1e-5,
				"{name} should stay at rest"
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
		const SAMPLES: [f32; 7] = [0.0, 0.2, 0.4, 0.5, 0.6, 0.8, 1.0];
		for progress in SAMPLES {
			let mut right_rig = HumanoidV0Rig::for_clip_test();
			let mut left_rig = HumanoidV0Rig::for_clip_test();
			let right_effects = PowerStrafe::new(Side::Right).apply(&mut right_rig, progress);
			let left_effects = PowerStrafe::new(Side::Left).apply(&mut left_rig, progress);
			let right_hand = bone_tail(&right_rig, forearm_bone(Side::Right));
			let left_hand = bone_tail(&left_rig, forearm_bone(Side::Left));
			assert!((right_hand.x + left_hand.x).abs() < 0.12, "hands at {progress}");
			assert!((right_hand.y - left_hand.y).abs() < 0.1, "hands at {progress}");
			assert!((right_hand.z - left_hand.z).abs() < 0.1, "hands at {progress}");
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

	/// Prints markdown-friendly pose tables for PR review (`cargo test … -- --ignored --nocapture`).
	#[test]
	#[ignore]
	fn power_strafe_pose_table_for_review() {
		const SAMPLES: [f32; 7] = [0.0, 0.2, 0.4, 0.5, 0.6, 0.8, 1.0];
		const FPS: f32 = 60.0;
		const RECOVERY_WINDOW: f32 = 0.38;
		let strafe = PowerStrafe::new(Side::Left);
		let start = 1.0 - RECOVERY_WINDOW;
		let steps = (RECOVERY_WINDOW * FPS).round() as u32;
		let mut max_delta = 0.0f32;
		let mut prev = strafe.armature_translation(start);
		for step in 1..=steps {
			let progress = start + RECOVERY_WINDOW * step as f32 / steps as f32;
			let next = strafe.armature_translation(progress);
			max_delta = max_delta.max((next - prev).length());
			prev = next;
		}
		eprintln!("max armature |Δ| at 60 Hz over progress [{start}, 1.0]: {max_delta:.6}");
		for side in [Side::Left, Side::Right] {
			eprintln!("\n### Side {:?}\n", side);
			eprintln!(
				"| progress | hand.L | hand.R | foot.L | foot.R | pelvis | armature offset |"
			);
			eprintln!("| --- | --- | --- | --- | --- | --- | --- |");
			for progress in SAMPLES {
				let mut rig = HumanoidV0Rig::for_clip_test();
				let effects = PowerStrafe::new(side).apply(&mut rig, progress);
				let hand_l = bone_tail(&rig, forearm_bone(Side::Left));
				let hand_r = bone_tail(&rig, forearm_bone(Side::Right));
				let foot_l = bone_tail(&rig, shin_bone(Side::Left));
				let foot_r = bone_tail(&rig, shin_bone(Side::Right));
				let pelvis = pelvis_center(&rig);
				eprintln!(
					"| {progress:.1} | ({hlx:.3}, {hly:.3}, {hlz:.3}) | ({hrx:.3}, {hry:.3}, {hrz:.3}) | ({flx:.3}, {fly:.3}, {flz:.3}) | ({frx:.3}, {fry:.3}, {frz:.3}) | ({px:.3}, {py:.3}, {pz:.3}) | ({ox:.3}, {oy:.3}, {oz:.3}) |",
					progress = progress,
					hlx = hand_l.x,
					hly = hand_l.y,
					hlz = hand_l.z,
					hrx = hand_r.x,
					hry = hand_r.y,
					hrz = hand_r.z,
					flx = foot_l.x,
					fly = foot_l.y,
					flz = foot_l.z,
					frx = foot_r.x,
					fry = foot_r.y,
					frz = foot_r.z,
					px = pelvis.x,
					py = pelvis.y,
					pz = pelvis.z,
					ox = effects.0.translation.x,
					oy = effects.0.translation.y,
					oz = effects.0.translation.z,
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
