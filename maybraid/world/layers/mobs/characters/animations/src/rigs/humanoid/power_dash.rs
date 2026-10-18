//! Humanoid mapping for [`UprightPowerDash`](crate::animations::UprightPowerDash).
//!
//! Asymmetric earthbending dash: one fist beside the face, opposite arm on a
//! rear diagonal, a bent supporting leg, and a trailing leg. The held silhouette
//! is authored first; other keys blend toward and away from it.

use bevy::prelude::Vec3;
use character_rigs::Side;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;

use crate::animations::{
	BRAKE_END, COMPRESS_END, HOLD_END, PUSH_END, UprightPowerDash, smoothstep,
};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects, Progress};

const HELD_DROP: f32 = 0.28;
/// Visual follow-through only. Capsule travel is Tanz `Move` / `MoveSpeedCap`.
const HELD_FORWARD: f32 = 0.22;
/// Compact guard / ready starts at identity so locomotion entry does not pop.
const READY_DROP: f32 = 0.0;
/// Mid-push key so the rear arm sweeps back instead of slerping out to the side.
const PUSH_MID: f32 = 0.175;
/// Mid-recovery key so arms drop forward instead of passing through a T-pose.
const RECOVER_MID: f32 = 0.925;

fn apply_aimed_arm(
	pose: &mut HumanoidPose,
	side: Side,
	along: Vec3,
	roll: f32,
	shoulder_carry: f32,
	elbow: f32,
) {
	if along.length_squared() < 1e-6 {
		return;
	}
	let arm = pose.arm_mut(side);
	arm.shoulder_forward += shoulder_carry;
	arm.elbow_flexion += elbow;
	arm.aim = Some(ArmAim { along: along.normalize(), roll });
}

impl UprightPowerDash {
	fn lead(&self) -> Side {
		self.jab.side
	}

	fn trail(&self) -> Side {
		self.jab.side.opposite()
	}

	/// Shoulder-to-elbow: forward, down and slightly outside — a high-guard humerus,
	/// not a punch.
	fn guard_along(&self) -> Vec3 {
		Vec3::new(0.18 * self.lead().sign(), -0.42, 0.82)
	}

	/// ~120° elbow so the forearm can fold up to the cheek from a lowered elbow.
	fn guard_elbow(&self) -> f32 {
		2.10
	}

	/// Right-lead wants a *positive* humerus roll (not the jab −π/2 punch plane).
	fn guard_roll(&self) -> f32 {
		-0.48 * self.lead().sign()
	}

	/// Rear diagonal: outward, slightly up, back.
	fn rear_along(&self) -> Vec3 {
		Vec3::new(0.58 * self.trail().sign(), 0.12, -0.82)
	}

	fn rear_elbow(&self) -> f32 {
		0.25
	}

	fn rear_roll(&self) -> f32 {
		0.22 * self.trail().sign()
	}

	fn apply_look_forward(&self, pose: &mut HumanoidPose, lean: f32, yaw: f32) {
		// Cancel hip lean so the face looks along +Z, not into the ground.
		let nod = (-0.50 * lean).clamp(-0.32, 0.0);
		// Counter-rotate the neck against torso yaw.
		pose.apply_neck_twisted(-yaw * 0.85, 0.0, nod, -yaw * 0.40, 0.0, nod * 0.45);
	}

	fn apply_torso_yaw(&self, pose: &mut HumanoidPose, yaw: f32) {
		// Right-lead: positive authored yaw sends the right shoulder forward.
		let signed = yaw * -self.lead().sign();
		pose.spine.add_turn(signed);
		pose.leg_mut(self.lead()).pelvis_turn += signed * 0.45;
		pose.leg_mut(self.trail()).pelvis_turn += signed * 0.20;
	}

	/// Held earthbending-dash silhouette. Sampled as-is from 0.25 through 0.68.
	fn held_pose(&self) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		let drive = self.drive;

		pose.apply_leg(lead, self.support_femur * drive, self.support_knee * drive);
		pose.apply_leg(trail, self.trail_femur * drive, self.trail_knee);
		pose.leg_mut(lead).hip_abduction += self.hip_abduct * 0.55 * lead.sign();
		pose.leg_mut(trail).hip_abduction += self.hip_abduct * trail.sign();
		pose.apply_hip_fold(lead, 0.26 * drive);
		pose.apply_hip_fold(trail, 0.12);
		pose.apply_root(self.lean * drive);
		self.apply_torso_yaw(&mut pose, self.yaw);

		apply_aimed_arm(
			&mut pose,
			lead,
			self.guard_along(),
			self.guard_roll(),
			0.10 * drive,
			self.guard_elbow(),
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			self.rear_along(),
			self.rear_roll(),
			-0.06,
			self.rear_elbow(),
		);
		self.apply_look_forward(&mut pose, self.lean * drive, self.yaw);
		pose
	}

	fn ready_pose(&self) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		pose.apply_leg(lead, -0.18, 0.32);
		pose.apply_leg(trail, -0.14, 0.28);
		pose.leg_mut(lead).hip_abduction += 0.06 * lead.sign();
		pose.leg_mut(trail).hip_abduction += 0.06 * trail.sign();
		pose.apply_root(0.08);
		apply_aimed_arm(
			&mut pose,
			lead,
			Vec3::new(0.12 * lead.sign(), -0.35, 0.55),
			self.guard_roll(),
			0.04,
			0.85,
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			Vec3::new(0.22 * trail.sign(), -0.32, 0.12),
			self.rear_roll(),
			0.02,
			0.70,
		);
		self.apply_look_forward(&mut pose, 0.08, 0.0);
		pose
	}

	fn compress_pose(&self) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		// Coil the pushing (trail) leg; load the support.
		pose.apply_leg(lead, -0.50, 0.85);
		pose.apply_leg(trail, -0.70, 0.70);
		pose.leg_mut(lead).hip_abduction += 0.08 * lead.sign();
		pose.leg_mut(trail).hip_abduction += 0.10 * trail.sign();
		pose.apply_root(0.18);
		apply_aimed_arm(
			&mut pose,
			lead,
			Vec3::new(0.12 * lead.sign(), -0.22, 0.75),
			self.guard_roll(),
			0.06,
			1.15,
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			Vec3::new(0.28 * trail.sign(), -0.28, -0.35),
			self.rear_roll(),
			-0.02,
			0.40,
		);
		self.apply_look_forward(&mut pose, 0.18, 0.0);
		pose
	}

	/// Mid-push: trail leg extends back, guard arm rises, rear arm already behind the body.
	fn sweep_pose(&self) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		let drive = self.drive;
		pose.apply_leg(lead, self.support_femur * 0.70 * drive, self.support_knee * 0.85 * drive);
		pose.apply_leg(trail, self.trail_femur * 0.55 * drive, self.trail_knee * 0.80);
		pose.leg_mut(lead).hip_abduction += self.hip_abduct * 0.40 * lead.sign();
		pose.leg_mut(trail).hip_abduction += self.hip_abduct * 0.70 * trail.sign();
		pose.apply_root(self.lean * 0.70 * drive);
		self.apply_torso_yaw(&mut pose, self.yaw * 0.55);
		apply_aimed_arm(
			&mut pose,
			lead,
			Vec3::new(0.14 * lead.sign(), -0.14, 0.92),
			self.guard_roll(),
			0.08 * drive,
			self.guard_elbow() * 0.85,
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			Vec3::new(0.42 * trail.sign(), -0.10, -0.55),
			self.rear_roll(),
			-0.04,
			0.30,
		);
		self.apply_look_forward(&mut pose, self.lean * 0.70 * drive, self.yaw * 0.55);
		pose
	}

	fn brake_pose(&self) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		let drive = self.drive;
		pose.apply_leg(lead, self.support_femur * 0.55 * drive, self.support_knee * 0.60 * drive);
		pose.apply_leg(trail, self.support_femur * 0.35, self.support_knee * 0.40);
		pose.leg_mut(lead).hip_abduction += self.hip_abduct * 0.30 * lead.sign();
		pose.leg_mut(trail).hip_abduction += 0.08 * trail.sign();
		pose.apply_root(self.lean * 0.55 * drive);
		self.apply_torso_yaw(&mut pose, self.yaw * 0.40);
		apply_aimed_arm(
			&mut pose,
			lead,
			Vec3::new(0.13 * lead.sign(), -0.22, 0.70),
			self.guard_roll(),
			0.06,
			1.20,
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			Vec3::new(0.32 * trail.sign(), -0.22, -0.28),
			self.rear_roll(),
			-0.02,
			0.40,
		);
		self.apply_look_forward(&mut pose, self.lean * 0.55 * drive, self.yaw * 0.40);
		pose
	}

	/// Down-and-in recovery so the rear arm does not pass through a T-pose.
	fn recover_pose(&self) -> HumanoidPose {
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		pose.apply_leg(lead, -0.28, 0.48);
		pose.apply_leg(trail, -0.22, 0.40);
		pose.leg_mut(lead).hip_abduction += 0.07 * lead.sign();
		pose.leg_mut(trail).hip_abduction += 0.07 * trail.sign();
		pose.apply_root(0.14);
		apply_aimed_arm(
			&mut pose,
			lead,
			Vec3::new(0.12 * lead.sign(), -0.30, 0.50),
			self.guard_roll(),
			0.04,
			1.05,
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			Vec3::new(0.22 * trail.sign(), -0.32, 0.10),
			self.rear_roll(),
			0.0,
			0.55,
		);
		self.apply_look_forward(&mut pose, 0.14, self.yaw * 0.25);
		pose
	}

	fn guard_pose(&self) -> HumanoidPose {
		self.ready_pose()
	}

	fn write(&self, rig: &mut HumanoidV0Rig, pose: &HumanoidPose) {
		rig.write_pose(pose);
	}

	fn held_offset(&self) -> Vec3 {
		Vec3::new(0.0, -HELD_DROP * self.drive, HELD_FORWARD * self.slide)
	}

	fn ready_offset(&self) -> Vec3 {
		Vec3::new(0.0, -READY_DROP * self.drive, 0.0)
	}

	fn compress_offset(&self) -> Vec3 {
		Vec3::new(0.0, -0.14 * self.drive, 0.08 * self.slide)
	}

	fn sweep_offset(&self) -> Vec3 {
		Vec3::new(0.0, -0.22 * self.drive, 0.16 * self.slide)
	}

	fn brake_offset(&self) -> Vec3 {
		Vec3::new(0.0, -0.12 * self.drive, 0.12 * self.slide)
	}

	fn recover_offset(&self) -> Vec3 {
		Vec3::new(0.0, -0.06 * self.drive, 0.05 * self.slide)
	}
}

struct KeyAt<'a> {
	dash: &'a UprightPowerDash,
	key: Key,
}

#[derive(Clone, Copy)]
enum Key {
	Ready,
	Compress,
	Sweep,
	Held,
	Brake,
	Recover,
	Guard,
}

impl KeyAt<'_> {
	fn pose(&self) -> HumanoidPose {
		match self.key {
			Key::Ready => self.dash.ready_pose(),
			Key::Compress => self.dash.compress_pose(),
			Key::Sweep => self.dash.sweep_pose(),
			Key::Held => self.dash.held_pose(),
			Key::Brake => self.dash.brake_pose(),
			Key::Recover => self.dash.recover_pose(),
			Key::Guard => self.dash.guard_pose(),
		}
	}

	fn offset(&self) -> Vec3 {
		match self.key {
			Key::Ready => self.dash.ready_offset(),
			Key::Compress => self.dash.compress_offset(),
			Key::Sweep => self.dash.sweep_offset(),
			Key::Held => self.dash.held_offset(),
			Key::Brake => self.dash.brake_offset(),
			Key::Recover => self.dash.recover_offset(),
			Key::Guard => self.dash.ready_offset(),
		}
	}
}

impl Animation<HumanoidV0Rig> for KeyAt<'_> {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, _progress: f32) {
		self.dash.write(rig, &self.pose());
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, _progress: f32) -> Effects {
		offset_or_identity(self.offset())
	}
}

fn crossfade(rig: &mut HumanoidV0Rig, dash: &UprightPowerDash, from: Key, to: Key, weight: f32) {
	blend_clips(rig, &KeyAt { dash, key: from }, 1.0, &KeyAt { dash, key: to }, 1.0, weight);
}

impl Animation<HumanoidV0Rig> for UprightPowerDash {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		if t < COMPRESS_END {
			let w = smoothstep(t / COMPRESS_END);
			crossfade(rig, self, Key::Ready, Key::Compress, w);
			return;
		}
		if t < PUSH_MID {
			let w = smoothstep((t - COMPRESS_END) / (PUSH_MID - COMPRESS_END));
			crossfade(rig, self, Key::Compress, Key::Sweep, w);
			return;
		}
		if t < PUSH_END {
			let w = smoothstep((t - PUSH_MID) / (PUSH_END - PUSH_MID));
			crossfade(rig, self, Key::Sweep, Key::Held, w);
			return;
		}
		if t < HOLD_END {
			self.write(rig, &self.held_pose());
			return;
		}
		if t < BRAKE_END {
			let w = smoothstep((t - HOLD_END) / (BRAKE_END - HOLD_END));
			crossfade(rig, self, Key::Held, Key::Brake, w);
			return;
		}
		if t < RECOVER_MID {
			let w = smoothstep((t - BRAKE_END) / (RECOVER_MID - BRAKE_END));
			crossfade(rig, self, Key::Brake, Key::Recover, w);
			return;
		}
		let w = smoothstep((t - RECOVER_MID) / (1.0 - RECOVER_MID).max(f32::EPSILON));
		crossfade(rig, self, Key::Recover, Key::Guard, w);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let t = Progress(progress).clamp();
		let translation = if t < COMPRESS_END {
			let w = smoothstep(t / COMPRESS_END);
			self.ready_offset().lerp(self.compress_offset(), w)
		} else if t < PUSH_MID {
			let w = smoothstep((t - COMPRESS_END) / (PUSH_MID - COMPRESS_END));
			self.compress_offset().lerp(self.sweep_offset(), w)
		} else if t < PUSH_END {
			let w = smoothstep((t - PUSH_MID) / (PUSH_END - PUSH_MID));
			self.sweep_offset().lerp(self.held_offset(), w)
		} else if t < HOLD_END {
			self.held_offset()
		} else if t < BRAKE_END {
			let w = smoothstep((t - HOLD_END) / (BRAKE_END - HOLD_END));
			self.held_offset().lerp(self.brake_offset(), w)
		} else if t < RECOVER_MID {
			let w = smoothstep((t - BRAKE_END) / (RECOVER_MID - BRAKE_END));
			self.brake_offset().lerp(self.recover_offset(), w)
		} else {
			let w = smoothstep((t - RECOVER_MID) / (1.0 - RECOVER_MID).max(f32::EPSILON));
			self.recover_offset().lerp(self.ready_offset(), w)
		};
		offset_or_identity(translation)
	}
}

fn offset_or_identity(translation: Vec3) -> Effects {
	if translation.length_squared() < 1e-8 {
		ArmatureOffset::IDENTITY
	} else {
		ArmatureOffset::from_translation(translation)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::{PowerDash, Run, Transition};
	use crate::rigs::transition::capture_animation_pose;
	use character_rigs::authoring::ArmatureOffset;

	const REST_FOOT_Y: f32 = -1.5;
	const SUPPORT_FOOT_SLACK: f32 = 0.20;
	const WALL_SECS: f32 = 0.38;
	/// Forearm bind length in the GLB rest. `character_length` is a unit vector.
	const FOREARM_LENGTH: f32 = 0.48;
	/// Visual mesh foot sits near the unit shin tail; joint-to-joint shin is 0.50.
	const FOOT_LENGTH: f32 = 1.0;

	fn dash() -> UprightPowerDash {
		UprightPowerDash::from_power_dash(&PowerDash::default())
	}

	fn apply(progress: f32) -> (HumanoidV0Rig, Effects) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let anim = dash();
		let effects = anim.apply(&mut rig, progress);
		(rig, effects)
	}

	fn joint(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		rig.character_point(bone)
	}

	fn bone_end(rig: &HumanoidV0Rig, bone: &str, length: f32) -> Vec3 {
		rig.character_point(bone) + rig.character_length(bone) * length
	}

	fn wrist(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		bone_end(rig, bone, FOREARM_LENGTH)
	}

	fn foot(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		bone_end(rig, bone, FOOT_LENGTH)
	}

	fn world_of(p: Vec3, offset: Vec3) -> Vec3 {
		p + offset
	}

	fn head_tail(rig: &HumanoidV0Rig) -> Vec3 {
		bone_end(rig, "upper_neck", 0.25)
	}

	fn pelvis_center(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("lumbar")
	}

	#[test]
	fn sampling_is_non_accumulating() {
		let (a, _) = apply(0.5);
		let (b, _) = apply(0.5);
		for name in a.animation_bone_names() {
			assert!(
				a.rotation(name).dot(b.rotation(name)).abs() > 1.0 - 1e-5,
				"repeat sample diverged on {name}"
			);
		}
		let mut twice = HumanoidV0Rig::for_clip_test();
		let anim = dash();
		anim.apply(&mut twice, 0.5);
		anim.apply(&mut twice, 0.5);
		for name in a.animation_bone_names() {
			assert!(
				a.rotation(name).dot(twice.rotation(name)).abs() > 1.0 - 1e-5,
				"in-place resample diverged on {name}"
			);
		}
	}

	#[test]
	fn progress_clamps_past_one() {
		let (a, eff_a) = apply(1.0);
		let (b, eff_b) = apply(1.4);
		assert_eq!(eff_a, eff_b);
		assert!(a.rotation("femur.L").dot(b.rotation("femur.L")).abs() > 1.0 - 1e-5);
	}

	#[test]
	fn entry_from_mid_run_uses_visible_pose() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut rig, 0.35);
		let from_pose = rig.pose.clone();
		let transition = Transition::from_visible(dash(), from_pose, ArmatureOffset::IDENTITY);
		let effects = transition.apply(&mut rig, 0.05, 0.0);
		assert!(effects.is_identity() || effects.0.translation.y <= 0.0);
	}

	#[test]
	fn entry_from_idle_via_transition() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let from_pose = capture_animation_pose(&crate::animations::Idle::default(), &mut rig, 0.0);
		let transition = Transition::from_visible(dash(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.0, 0.0);
		assert!(rig.posed_angle("humerus.R") > 0.01);
	}

	#[test]
	fn left_side_mirrors_the_held_guard() {
		let right = UprightPowerDash::from_power_dash(&PowerDash::default());
		let left = UprightPowerDash::from_power_dash(&PowerDash {
			side: Side::Left,
			drive: 1.0,
			slide: 1.0,
		});
		let mut rig_r = HumanoidV0Rig::for_clip_test();
		let mut rig_l = HumanoidV0Rig::for_clip_test();
		right.apply(&mut rig_r, 0.50);
		left.apply(&mut rig_l, 0.50);
		let fist_r = wrist(&rig_r, "forearm.R");
		let fist_l = wrist(&rig_l, "forearm.L");
		assert!(
			(fist_r.x + fist_l.x).abs() < 0.35,
			"left-side guard should mirror, R={fist_r:?} L={fist_l:?}"
		);
	}

	#[test]
	fn recovery_does_not_snap_to_bind_t_pose() {
		let (rig, effects) = apply(1.0);
		assert!(
			effects.is_identity() || effects.0.translation.length() < 0.08,
			"offset should ease toward identity, got {:?}",
			effects.0.translation
		);
		assert!(
			rig.posed_angle("humerus.R") > 0.05 || rig.posed_angle("forearm.R") > 0.05,
			"end pose is an authored guard, not bind T-pose"
		);
	}

	#[test]
	fn phase_boundaries_stay_continuous() {
		let boundaries = [
			(COMPRESS_END - 0.01, COMPRESS_END + 0.01),
			(PUSH_MID - 0.01, PUSH_MID + 0.01),
			(PUSH_END - 0.01, PUSH_END + 0.01),
			(HOLD_END - 0.01, HOLD_END + 0.01),
			(BRAKE_END - 0.01, BRAKE_END + 0.01),
			(RECOVER_MID - 0.01, RECOVER_MID + 0.01),
		];
		for (a, b) in boundaries {
			let (rig_a, _) = apply(a);
			let (rig_b, _) = apply(b);
			for name in rig_a.animation_bone_names() {
				let delta = rig_a.rotation(name).angle_between(rig_b.rotation(name));
				assert!(delta < 0.55, "pop across {a} -> {b} on {name}: {delta}");
			}
		}
	}

	#[test]
	fn held_pose_joint_relationships() {
		let (rig, effects) = apply(0.50);
		let off = effects.0.translation;
		let shoulder_r = world_of(joint(&rig, "humerus.R"), off);
		let elbow_r = world_of(joint(&rig, "forearm.R"), off);
		let fist_r = world_of(wrist(&rig, "forearm.R"), off);
		let shoulder_l = world_of(joint(&rig, "humerus.L"), off);
		let elbow_l = world_of(joint(&rig, "forearm.L"), off);
		let fist_l = world_of(wrist(&rig, "forearm.L"), off);
		let knee_r = world_of(joint(&rig, "shin.R"), off);
		let knee_l = world_of(joint(&rig, "shin.L"), off);
		let foot_r = world_of(foot(&rig, "shin.R"), off);
		let foot_l = world_of(foot(&rig, "shin.L"), off);
		let pelvis = world_of(pelvis_center(&rig), off);
		let head = world_of(head_tail(&rig), off);

		eprintln!("held 0.50 world (pose+offset):");
		eprintln!("  shoulder.R {shoulder_r:?}");
		eprintln!("  elbow.R    {elbow_r:?}");
		eprintln!("  fist.R     {fist_r:?}");
		eprintln!("  shoulder.L {shoulder_l:?}");
		eprintln!("  elbow.L    {elbow_l:?}");
		eprintln!("  fist.L     {fist_l:?}");
		eprintln!("  knee.R     {knee_r:?}  foot.R {foot_r:?}");
		eprintln!("  knee.L     {knee_l:?}  foot.L {foot_l:?}");
		eprintln!("  pelvis {pelvis:?}  head {head:?}  offset {off:?}");

		assert!(
			elbow_r.z > shoulder_r.z + 0.08,
			"right elbow should sit ahead of the shoulder, elbow z={} shoulder z={}",
			elbow_r.z,
			shoulder_r.z
		);
		assert!(
			fist_r.z < elbow_r.z - 0.03,
			"right fist folds back toward the face, fist z={} elbow z={}",
			fist_r.z,
			elbow_r.z
		);
		assert!(
			fist_r.z > shoulder_r.z + 0.05,
			"right fist stays ahead of the shoulder, fist z={} shoulder z={}",
			fist_r.z,
			shoulder_r.z
		);
		assert!(
			fist_r.x < head.x - 0.12,
			"right fist stays beside the head, not across the face, fist x={} head x={}",
			fist_r.x,
			head.x
		);
		assert!(
			(fist_r.y - head.y).abs() < 0.55,
			"right fist stays near head height, fist y={} head y={}",
			fist_r.y,
			head.y
		);
		assert!(
			fist_l.z < shoulder_l.z - 0.15,
			"left fist trails behind the shoulder, fist z={} shoulder z={}",
			fist_l.z,
			shoulder_l.z
		);
		assert!(fist_l.x > 0.35, "left fist stays outside the torso silhouette, x={}", fist_l.x);
		assert!(
			(foot_r.y - REST_FOOT_Y).abs() <= SUPPORT_FOOT_SLACK,
			"support foot stays near the ground, y={}",
			foot_r.y
		);
		assert!(
			foot_l.y > foot_r.y + 0.12,
			"trailing foot lifts clear of the ground, L={} R={}",
			foot_l.y,
			foot_r.y
		);
		assert!(
			foot_l.z < foot_r.z - 0.15,
			"trailing foot sits behind the support foot, L.z={} R.z={}",
			foot_l.z,
			foot_r.z
		);
		assert!(
			head.y - pelvis.y > 0.70,
			"spine stays fairly straight, head y={} pelvis y={}",
			head.y,
			pelvis.y
		);
		assert!(
			off.z > 0.18 && off.z < 0.28,
			"held visual slide stays a follow-through, not capsule travel, z={}",
			off.z
		);
		assert!(head.z > pelvis.z + 0.12, "torso pitches forward from the hips");
		assert!(
			rig.posed_angle("forearm.R") > 0.8,
			"guard elbow stays bent, angle={}",
			rig.posed_angle("forearm.R")
		);
		assert!(
			rig.posed_angle("forearm.L") < 0.55,
			"rear elbow stays nearly straight, angle={}",
			rig.posed_angle("forearm.L")
		);
	}

	#[test]
	fn choreography_world_joint_table() {
		eprintln!(
			"progress | sh.R | el.R | wr.R | sh.L | el.L | wr.L | knee.R | foot.R | knee.L | foot.L | pelvis | head | offset"
		);
		for progress in [0.00, 0.10, 0.25, 0.50, 0.68, 0.85, 0.95, 1.00] {
			let (rig, effects) = apply(progress);
			let off = effects.0.translation;
			eprintln!(
				"{progress:.2} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?} | {:?}",
				world_of(joint(&rig, "humerus.R"), off),
				world_of(joint(&rig, "forearm.R"), off),
				world_of(wrist(&rig, "forearm.R"), off),
				world_of(joint(&rig, "humerus.L"), off),
				world_of(joint(&rig, "forearm.L"), off),
				world_of(wrist(&rig, "forearm.L"), off),
				world_of(joint(&rig, "shin.R"), off),
				world_of(foot(&rig, "shin.R"), off),
				world_of(joint(&rig, "shin.L"), off),
				world_of(foot(&rig, "shin.L"), off),
				world_of(pelvis_center(&rig), off),
				world_of(head_tail(&rig), off),
				off,
			);
		}
	}

	#[test]
	fn support_foot_stays_planted_through_the_held_slide() {
		for progress in [0.25, 0.40, 0.50, 0.62, 0.68] {
			let (rig, effects) = apply(progress);
			let foot_r = world_of(foot(&rig, "shin.R"), effects.0.translation);
			assert!(
				(foot_r.y - REST_FOOT_Y).abs() <= SUPPORT_FOOT_SLACK,
				"support foot left the ground at {progress}, y={}",
				foot_r.y
			);
		}
	}

	#[test]
	fn real_60hz_offset_is_smooth() {
		let dt = 1.0 / (60.0 * WALL_SECS);
		let mut prev = Effects::IDENTITY;
		let mut max_delta = 0.0f32;
		let mut t = 0.0;
		while t <= 1.0 + dt * 0.5 {
			let (_, effects) = apply(t.min(1.0));
			let delta = (effects.0.translation - prev.0.translation).length();
			max_delta = max_delta.max(delta);
			prev = effects;
			t += dt;
		}
		eprintln!("max per-frame ArmatureOffset Δ @ 60 Hz over 0.38 s: {max_delta:.4} m");
		assert!(max_delta < 0.10, "offset pop {max_delta}");
	}

	#[test]
	fn pose_continuity_at_real_60hz() {
		let dt = 1.0 / (60.0 * WALL_SECS);
		let mut prev = apply(0.0).0;
		let mut t = dt;
		let mut max_delta = 0.0f32;
		let mut max_at = (0.0f32, "");
		while t <= 1.0 + dt * 0.5 {
			let (rig, _) = apply(t.min(1.0));
			for name in prev.animation_bone_names() {
				let delta = prev.rotation(name).angle_between(rig.rotation(name));
				if delta > max_delta {
					max_delta = delta;
					max_at = (t, name);
				}
				assert!(delta < 0.75, "frame pop at t={t:.4} on {name}: {delta}");
			}
			prev = rig;
			t += dt;
		}
		eprintln!(
			"max per-frame pose Δ @ 60 Hz: {max_delta:.3} rad at t={:.3} on {}",
			max_at.0, max_at.1
		);
	}

	fn dump_world(label: &str, rig: &HumanoidV0Rig, off: Vec3) {
		let sh_r = world_of(joint(rig, "humerus.R"), off);
		let el_r = world_of(joint(rig, "forearm.R"), off);
		let wr_r = world_of(wrist(rig, "forearm.R"), off);
		let sh_l = world_of(joint(rig, "humerus.L"), off);
		let el_l = world_of(joint(rig, "forearm.L"), off);
		let wr_l = world_of(wrist(rig, "forearm.L"), off);
		let foot_r = world_of(foot(rig, "shin.R"), off);
		let foot_l = world_of(foot(rig, "shin.L"), off);
		let pelvis = world_of(pelvis_center(rig), off);
		let head = world_of(head_tail(rig), off);
		let neck = world_of(joint(rig, "upper_neck"), off);
		eprintln!(
			"{label:10} sh.R=({:.2},{:.2},{:.2}) el.R=({:.2},{:.2},{:.2}) wr.R=({:.2},{:.2},{:.2}) neck=({:.2},{:.2},{:.2}) head=({:.2},{:.2},{:.2}) sh.L=({:.2},{:.2},{:.2}) el.L=({:.2},{:.2},{:.2}) wr.L=({:.2},{:.2},{:.2}) foot.R=({:.2},{:.2},{:.2}) foot.L=({:.2},{:.2},{:.2}) pelvis=({:.2},{:.2},{:.2}) off={off:?}",
			sh_r.x,
			sh_r.y,
			sh_r.z,
			el_r.x,
			el_r.y,
			el_r.z,
			wr_r.x,
			wr_r.y,
			wr_r.z,
			neck.x,
			neck.y,
			neck.z,
			head.x,
			head.y,
			head.z,
			sh_l.x,
			sh_l.y,
			sh_l.z,
			el_l.x,
			el_l.y,
			el_l.z,
			wr_l.x,
			wr_l.y,
			wr_l.z,
			foot_r.x,
			foot_r.y,
			foot_r.z,
			foot_l.x,
			foot_l.y,
			foot_l.z,
			pelvis.x,
			pelvis.y,
			pelvis.z
		);
	}

	#[test]
	fn rear_arm_does_not_pass_through_a_t_pose() {
		let dt = 1.0 / (60.0 * WALL_SECS);
		let mut t = 0.0;
		let mut max_x = 0.0f32;
		while t <= 1.0 + dt * 0.5 {
			let (rig, effects) = apply(t.min(1.0));
			let wr_l = world_of(wrist(&rig, "forearm.L"), effects.0.translation);
			max_x = max_x.max(wr_l.x);
			assert!(wr_l.x < 1.22, "left wrist spiked sideways at t={t:.3}: x={}", wr_l.x);
			t += dt;
		}
		eprintln!("max left-wrist x @ 60 Hz: {max_x:.3}");
	}

	#[ignore = "authoring probe; run with --ignored --nocapture"]
	#[test]
	fn debug_held_arm_roll_sweep() {
		let dash = dash();
		let rest = HumanoidV0Rig::for_clip_test();
		dump_world("rest", &rest, Vec3::ZERO);
		let (held_rig, held_fx) = apply(0.50);
		dump_world("held", &held_rig, held_fx.0.translation);
		for (label, roll) in [
			("r+0.6", 0.6),
			("r+0.9", 0.9),
			("r+1.05", 1.05),
			("r+1.2", 1.2),
			("r+1.5", 1.5),
			("rear0", 0.0),
		] {
			let mut pose = dash.held_pose();
			if label.starts_with('r') && label != "rear0" {
				apply_aimed_arm(&mut pose, Side::Right, dash.guard_along(), roll, 0.0, 0.0);
			}
			let mut rig = HumanoidV0Rig::for_clip_test();
			rig.write_pose(&pose);
			dump_world(label, &rig, dash.held_offset());
		}
		for (label, sf, tf, drop) in [
			("f-0.10", -0.10, 0.72, 0.22),
			("f-0.40", -0.40, 0.72, 0.22),
			("f-0.70", -0.70, 0.72, 0.22),
			("t+0.40", -0.28, 0.40, 0.22),
			("t+0.90", -0.28, 0.90, 0.22),
			("t+1.10", -0.28, 1.10, 0.22),
			("d0.35", -0.28, 0.72, 0.35),
			("d0.45", -0.28, 0.72, 0.45),
			("combo", -0.55, 0.95, 0.38),
		] {
			let mut pose = HumanoidPose::default();
			pose.apply_leg(Side::Right, sf, dash.support_knee);
			pose.apply_leg(Side::Left, tf, dash.trail_knee);
			pose.leg_mut(Side::Right).hip_abduction += dash.hip_abduct * 0.55 * Side::Right.sign();
			pose.leg_mut(Side::Left).hip_abduction += dash.hip_abduct * Side::Left.sign();
			pose.apply_root(dash.lean);
			let mut rig = HumanoidV0Rig::for_clip_test();
			rig.write_pose(&pose);
			dump_world(label, &rig, Vec3::new(0.0, -drop, HELD_FORWARD));
		}
		for t in [0.00, 0.10, 0.14, 0.18, 0.22, 0.25, 0.68, 0.76, 0.85, 0.92, 1.00] {
			let (rig, fx) = apply(t);
			dump_world(&format!("t{t:.2}"), &rig, fx.0.translation);
		}
	}
}
