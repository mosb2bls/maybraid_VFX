//! Humanoid mapping for [`UprightPowerDash`](crate::animations::UprightPowerDash).
//!
//! Cartoon earthbending dash: chambered horse-stance wind-up, a stomping lunge
//! and straight-arm strike, then a low held anime dash that settles.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{
	smoothstep, PowerDash, UprightPowerDash, JAB_PHASE_END, JAB_PROGRESS_AT_HANDOFF,
	STRIDE_PHASE_END,
};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects, Progress};

const STRIDE_BLEND: f32 = 0.18;
/// Slide-phase progress where the dash hold yields to settle.
const SLIDE_SETTLE_START: f32 = 0.58;
const WINDUP_DROP: f32 = 0.05;
const STOMP_DROP: f32 = 0.13;
const DASH_DROP: f32 = 0.08;
const DASH_FORWARD: f32 = 0.10;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

fn lerp_vec3(a: Vec3, b: Vec3, t: f32) -> Vec3 {
	a + (b - a) * t
}

/// Reach 1 by `attack` and hold. `t` is 0..1 inside a phase.
fn attack_hold(t: f32, attack: f32) -> f32 {
	let t = t.clamp(0.0, 1.0);
	if t < attack {
		smoothstep(t / attack.max(f32::EPSILON))
	} else {
		1.0
	}
}

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

	fn chamber_along(&self) -> Vec3 {
		// Out on the punching side and drawn back, kept at rib height.
		Vec3::new(-0.72 * self.lead().sign(), 0.12, -0.68)
	}

	fn strike_along(&self) -> Vec3 {
		Vec3::new(0.05 * self.lead().sign(), -0.20, 1.0)
	}

	fn dash_trail_along(&self) -> Vec3 {
		Vec3::new(0.10 * self.lead().sign(), -0.10, -0.92)
	}

	fn dash_lead_along(&self) -> Vec3 {
		Vec3::new(0.04 * self.lead().sign(), -0.16, 0.62)
	}

	fn windup_guard_along(&self) -> Vec3 {
		Vec3::new(0.20 * self.lead().sign(), -0.30, 0.35)
	}

	fn apply_windup_pose(&self, rig: &mut HumanoidV0Rig, u: f32) {
		let u = u.clamp(0.0, 1.0);
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		// Squat-signed femurs so both feet stay planted while the knees sink.
		pose.apply_leg(lead, -0.58 * u, self.windup_knee * u);
		pose.apply_leg(trail, -0.50 * u, self.windup_knee * 0.96 * u);
		pose.leg_mut(lead).hip_abduction += self.hip_abduct * u;
		pose.leg_mut(trail).hip_abduction += self.hip_abduct * u;
		pose.apply_hip_fold(lead, 0.38 * u);
		pose.apply_hip_fold(trail, 0.42 * u);
		pose.apply_root(0.12 * u);
		pose.apply_spine_pitch(0.20 * u);
		if u > 1e-4 {
			apply_aimed_arm(
				&mut pose,
				lead,
				self.chamber_along(),
				FRAC_PI_2 * lead.sign(),
				-0.08 * u,
				1.48 * u,
			);
			apply_aimed_arm(
				&mut pose,
				trail,
				self.windup_guard_along(),
				FRAC_PI_2 * trail.sign(),
				0.05 * u,
				1.15 * u,
			);
		}
		pose.apply_neck_twisted(0.0, 0.0, 0.10 * u, 0.0, 0.0, 0.06 * u);
		rig.write_pose(&pose);
	}

	fn apply_stride_pose(&self, rig: &mut HumanoidV0Rig, u: f32) {
		let u = u.clamp(0.0, 1.0);
		let stomp = 1.0 - (1.0 - u).powi(3);
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();

		pose.apply_leg(
			lead,
			lerp(-0.58, -self.lead_stride, stomp),
			lerp(self.windup_knee, self.lunge_knee_lead, stomp),
		);
		pose.apply_leg(
			trail,
			lerp(-0.50, self.trail_stride, stomp),
			lerp(self.windup_knee * 0.96, self.lunge_knee_trail, stomp),
		);
		pose.leg_mut(lead).hip_abduction += lerp(self.hip_abduct, self.hip_abduct * 0.80, stomp);
		pose.leg_mut(trail).hip_abduction += lerp(self.hip_abduct, self.hip_abduct * 0.65, stomp);
		pose.apply_hip_fold(lead, lerp(0.38, 0.18, stomp));
		pose.apply_hip_fold(trail, lerp(0.42, 0.12, stomp));
		pose.apply_root(lerp(0.12, self.lunge_lean, stomp));
		pose.apply_spine_pitch(lerp(0.20, 0.12, stomp));

		let strike = 1.0 - (1.0 - stomp).powi(2);
		apply_aimed_arm(
			&mut pose,
			lead,
			lerp_vec3(self.chamber_along(), self.strike_along(), strike),
			FRAC_PI_2 * lead.sign(),
			lerp(-0.08, 0.20 * self.arm_reach, strike),
			lerp(1.48, 0.06, strike),
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			lerp_vec3(self.windup_guard_along(), self.dash_trail_along(), strike),
			FRAC_PI_2 * trail.sign(),
			lerp(0.04, -0.08, strike),
			lerp(1.12, 0.72, strike),
		);
		pose.apply_neck_twisted(
			0.0,
			0.0,
			lerp(0.12, 0.18, stomp),
			0.0,
			0.0,
			lerp(0.08, 0.10, stomp),
		);
		rig.write_pose(&pose);
	}

	fn apply_slide_pose(&self, rig: &mut HumanoidV0Rig, u: f32) {
		let u = u.clamp(0.0, 1.0);
		let enter = attack_hold(u, 0.22);
		let settle = if u < SLIDE_SETTLE_START {
			0.0
		} else {
			smoothstep((u - SLIDE_SETTLE_START) / (1.0 - SLIDE_SETTLE_START).max(f32::EPSILON))
		};
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();

		let lead_f = lerp(lerp(-self.lead_stride, -0.62, enter), -0.20, settle);
		let trail_f = lerp(lerp(self.trail_stride, -0.48, enter), 0.10, settle);
		let lead_k = lerp(lerp(self.lunge_knee_lead, self.slide_knee, enter), 0.28, settle);
		let trail_k =
			lerp(lerp(self.lunge_knee_trail, self.slide_knee * 0.95, enter), 0.24, settle);
		pose.apply_leg(lead, lead_f, lead_k);
		pose.apply_leg(trail, trail_f, trail_k);
		pose.leg_mut(lead).hip_abduction += self.hip_abduct * 0.55 * (1.0 - settle);
		pose.leg_mut(trail).hip_abduction += self.hip_abduct * 0.45 * (1.0 - settle);
		pose.apply_hip_fold(lead, 0.10 * (1.0 - settle));
		pose.apply_hip_fold(trail, 0.08 * (1.0 - settle));

		let lean = lerp(self.lunge_lean, self.slide_lean, enter) * (1.0 - settle) + 0.06 * settle;
		pose.apply_root(lean);
		pose.apply_spine_pitch(lerp(0.12, 0.24, enter) * (1.0 - settle));

		apply_aimed_arm(
			&mut pose,
			lead,
			lerp_vec3(self.strike_along(), self.dash_lead_along(), enter)
				.lerp(Vec3::new(0.08 * lead.sign(), -0.25, 0.20), settle),
			FRAC_PI_2 * lead.sign(),
			lerp(0.20 * self.arm_reach, 0.06, enter) * (1.0 - settle),
			lerp(0.06, 0.85, enter) * (1.0 - settle) + 0.35 * settle,
		);
		apply_aimed_arm(
			&mut pose,
			trail,
			lerp_vec3(self.dash_trail_along(), self.dash_trail_along(), enter)
				.lerp(Vec3::new(-0.10 * lead.sign(), -0.20, -0.15), settle),
			FRAC_PI_2 * trail.sign(),
			lerp(-0.08, -0.12, enter) * (1.0 - settle),
			lerp(0.72, 0.48, enter) * (1.0 - settle) + 0.30 * settle,
		);
		pose.apply_neck_twisted(
			0.0,
			0.0,
			lerp(0.18, 0.28, enter) * (1.0 - settle),
			0.0,
			0.0,
			lerp(0.10, 0.16, enter) * (1.0 - settle),
		);
		rig.write_pose(&pose);
	}
}

struct WindUpAt<'a> {
	dash: &'a UprightPowerDash,
}

impl Animation<HumanoidV0Rig> for WindUpAt<'_> {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		self.dash.apply_windup_pose(rig, progress);
	}
}

struct StrideAt<'a> {
	dash: &'a UprightPowerDash,
}

impl Animation<HumanoidV0Rig> for StrideAt<'_> {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		self.dash.apply_stride_pose(rig, progress);
	}
}

impl Animation<HumanoidV0Rig> for UprightPowerDash {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		if t < JAB_PHASE_END {
			let u = t / JAB_PHASE_END;
			let wind = attack_hold(u, 0.72);
			if u < 0.48 {
				blend_clips(
					rig,
					&self.jab,
					JAB_PROGRESS_AT_HANDOFF * (u / 0.48),
					&WindUpAt { dash: self },
					wind,
					smoothstep(u / 0.48),
				);
				return;
			}
			self.apply_windup_pose(rig, wind);
			return;
		}
		if t < STRIDE_PHASE_END {
			let phase = (t - JAB_PHASE_END) / (STRIDE_PHASE_END - JAB_PHASE_END);
			if phase < STRIDE_BLEND {
				let weight = smoothstep(phase / STRIDE_BLEND);
				blend_clips(
					rig,
					&WindUpAt { dash: self },
					1.0,
					&StrideAt { dash: self },
					0.0,
					weight,
				);
				return;
			}
			let stride_u = attack_hold((phase - STRIDE_BLEND) / (1.0 - STRIDE_BLEND), 0.42);
			self.apply_stride_pose(rig, stride_u);
			return;
		}
		let slide_u = (t - STRIDE_PHASE_END) / (1.0 - STRIDE_PHASE_END).max(f32::EPSILON);
		self.apply_slide_pose(rig, slide_u);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let t = Progress(progress).clamp();
		if t < JAB_PHASE_END {
			let drop = WINDUP_DROP * self.drive * attack_hold(t / JAB_PHASE_END, 0.72);
			return offset_or_identity(Vec3::new(0.0, -drop, 0.0));
		}
		if t < STRIDE_PHASE_END {
			let phase = (t - JAB_PHASE_END) / (STRIDE_PHASE_END - JAB_PHASE_END);
			let stomp = attack_hold(phase, 0.35);
			let drop = lerp(WINDUP_DROP, STOMP_DROP, stomp) * self.drive;
			return offset_or_identity(Vec3::new(0.0, -drop, 0.0));
		}
		let u = (t - STRIDE_PHASE_END) / (1.0 - STRIDE_PHASE_END).max(f32::EPSILON);
		let enter = attack_hold(u, 0.22);
		let settle = if u < SLIDE_SETTLE_START {
			0.0
		} else {
			smoothstep((u - SLIDE_SETTLE_START) / (1.0 - SLIDE_SETTLE_START).max(f32::EPSILON))
		};
		let hold = enter * (1.0 - settle);
		let drop = lerp(STOMP_DROP, DASH_DROP, enter) * self.drive * (1.0 - settle);
		let forward = DASH_FORWARD * self.slide * hold;
		offset_or_identity(Vec3::new(0.0, -drop, forward))
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
	use crate::animations::{Run, Transition};
	use crate::rigs::transition::capture_animation_pose;
	use character_rigs::authoring::ArmatureOffset;

	fn dash() -> UprightPowerDash {
		UprightPowerDash::from_power_dash(&PowerDash::default())
	}

	fn apply(progress: f32) -> (HumanoidV0Rig, Effects) {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let anim = dash();
		let effects = anim.apply(&mut rig, progress);
		(rig, effects)
	}

	fn bone_tail(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		rig.character_point(bone) + rig.character_length(bone)
	}

	fn pelvis_center(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("lumbar")
	}

	fn head_tail(rig: &HumanoidV0Rig) -> Vec3 {
		bone_tail(rig, "upper_neck")
	}

	#[test]
	fn opening_windup_chambers_the_lead_arm() {
		let (rig, effects) = apply(0.12);
		let idle = HumanoidV0Rig::for_clip_test();
		assert!(effects.0.translation.y <= 0.0);
		assert!(
			rig.posed_angle("humerus.R") > idle.posed_angle("humerus.R") + 0.02,
			"wind-up drives the lead humerus"
		);
		assert!(rig.posed_angle("femur.R") > 0.05);
		assert!(rig.posed_angle("shin.R") > 0.08, "weight sinks into a horse stance");
	}

	#[test]
	fn stride_drops_hips_and_splits_legs() {
		let (rig, effects) = apply(0.42);
		assert!(effects.0.translation.y < -0.04, "stomp lowers the armature");
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!((left.z - right.z).abs() > 0.2, "lunge keeps a deep split, L={left:?} R={right:?}");
	}

	#[test]
	fn slide_carries_forward_offset() {
		let (_, effects) = apply(0.75);
		assert!(effects.0.translation.z > 0.04, "dash offsets fight-forward");
	}

	#[test]
	fn phase_boundaries_stay_continuous() {
		let boundaries = [
			(JAB_PHASE_END - 0.01, JAB_PHASE_END + 0.01),
			(STRIDE_PHASE_END - 0.01, STRIDE_PHASE_END + 0.01),
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
	fn entry_from_mid_run_uses_visible_pose() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		Run::default().apply(&mut rig, 0.35);
		let from_pose = rig.pose.clone();
		let from_offset = ArmatureOffset::IDENTITY;
		let transition = Transition::from_visible(dash(), from_pose, from_offset);
		let effects = transition.apply(&mut rig, 0.05, 0.0);
		assert!(effects.is_identity() || effects.0.translation.y <= 0.0);
		assert!(rig.posed_angle("femur.R") > 0.02);
	}

	#[test]
	fn entry_from_idle_via_transition() {
		let mut rig = HumanoidV0Rig::for_clip_test();
		let from_pose = capture_animation_pose(&crate::animations::Idle::default(), &mut rig, 0.0);
		let transition = Transition::from_visible(dash(), from_pose, ArmatureOffset::IDENTITY);
		transition.apply(&mut rig, 0.0, 0.0);
		assert!(rig.posed_angle("humerus.R") > 0.02);
	}

	#[test]
	fn progress_clamps_past_one() {
		let (a, eff_a) = apply(1.0);
		let (b, eff_b) = apply(1.4);
		assert_eq!(eff_a, eff_b);
		assert!(a.rotation("femur.L").dot(b.rotation("femur.L")).abs() > 1.0 - 1e-5);
	}

	#[test]
	fn left_jab_side_flips_stance() {
		let right = UprightPowerDash::from_power_dash(&PowerDash::default());
		let left = UprightPowerDash::from_power_dash(&PowerDash {
			side: Side::Left,
			drive: 1.0,
			slide: 1.0,
		});
		let mut rig_r = HumanoidV0Rig::for_clip_test();
		let mut rig_l = HumanoidV0Rig::for_clip_test();
		right.apply(&mut rig_r, 0.14);
		left.apply(&mut rig_l, 0.14);
		assert!(
			rig_r.posed_angle("forearm.R") < rig_l.posed_angle("forearm.L") + 0.2,
			"lead elbow chamber follows jab side"
		);
	}

	#[test]
	fn endpoint_relaxes_toward_neutral() {
		let (rig, effects) = apply(1.0);
		assert!(rig.posed_angle("femur.R") < 0.75);
		assert!(
			effects.is_identity(),
			"clip end offset must be identity for AnimId hand-off, got {:?}",
			effects.0.translation
		);
	}

	#[test]
	fn posed_joint_evidence_at_progress_samples() {
		let (start_rig, _) = apply(0.0);
		let start_hand_r = bone_tail(&start_rig, "forearm.R");
		let start_foot_r_y = bone_tail(&start_rig, "shin.R").y;
		let samples = [0.0, 0.1, 0.2, 0.35, 0.5, 0.65, 0.8, 1.0];

		eprintln!(
			"\n| progress | hand.R (x,y,z) | hand.L | foot.R | foot.L | pelvis | head | offset (x,y,z) |"
		);
		eprintln!("| --- | --- | --- | --- | --- | --- | --- | --- |");

		let mut prev_offset = Effects::IDENTITY;
		let mut max_offset_delta_60hz = 0.0f32;
		for frame in 0..=60 {
			let t = frame as f32 / 60.0;
			let (_, effects) = apply(t);
			let delta = (effects.0.translation - prev_offset.0.translation).length();
			max_offset_delta_60hz = max_offset_delta_60hz.max(delta);
			prev_offset = effects;
		}

		let (windup_rig, _) = apply(0.20);
		let windup_hand_r = bone_tail(&windup_rig, "forearm.R");
		let (strike_rig, _) = apply(0.50);
		let strike_hand_r = bone_tail(&strike_rig, "forearm.R");
		let (dash_rig, _) = apply(0.65);
		let dash_head = head_tail(&dash_rig);
		let dash_pelvis = pelvis_center(&dash_rig);

		for progress in samples {
			let (rig, effects) = apply(progress);
			let hand_r = bone_tail(&rig, "forearm.R");
			let hand_l = bone_tail(&rig, "forearm.L");
			let foot_r = bone_tail(&rig, "shin.R");
			let foot_l = bone_tail(&rig, "shin.L");
			let pelvis = pelvis_center(&rig);
			let head = head_tail(&rig);
			let off = effects.0.translation;
			eprintln!(
				"| {progress:.2} | ({hrx:.3},{hry:.3},{hrz:.3}) | ({hlx:.3},{hly:.3},{hlz:.3}) | ({frx:.3},{fry:.3},{frz:.3}) | ({flx:.3},{fly:.3},{flz:.3}) | ({px:.3},{py:.3},{pz:.3}) | ({hx:.3},{hy:.3},{hz:.3}) | ({ox:.3},{oy:.3},{oz:.3}) |",
				hrx = hand_r.x,
				hry = hand_r.y,
				hrz = hand_r.z,
				hlx = hand_l.x,
				hly = hand_l.y,
				hlz = hand_l.z,
				frx = foot_r.x,
				fry = foot_r.y,
				frz = foot_r.z,
				flx = foot_l.x,
				fly = foot_l.y,
				flz = foot_l.z,
				px = pelvis.x,
				py = pelvis.y,
				pz = pelvis.z,
				hx = head.x,
				hy = head.y,
				hz = head.z,
				ox = off.x,
				oy = off.y,
				oz = off.z,
			);

			if (progress - 0.2).abs() < f32::EPSILON {
				assert!(
					hand_r.z < start_hand_r.z - 0.08,
					"wind-up draws the lead hand back: start z={} wind-up z={}",
					start_hand_r.z,
					hand_r.z
				);
				assert!(
					rig.posed_angle("shin.R") > 0.35 && rig.posed_angle("shin.L") > 0.35,
					"wind-up sinks both knees"
				);
				assert!(
					foot_r.y < start_foot_r_y + 0.35,
					"horse stance keeps the lead foot near the ground: start y={} wind-up y={}",
					start_foot_r_y,
					foot_r.y
				);
				assert!(
					effects.0.translation.y < -0.01,
					"wind-up drops the armature y={}",
					effects.0.translation.y
				);
			}
			if (progress - 0.35).abs() < f32::EPSILON || (progress - 0.5).abs() < f32::EPSILON {
				let split = (foot_r.z - foot_l.z).abs();
				assert!(split > 0.35, "stomp/lunge splits feet forward/back: {split}");
				assert!(
					effects.0.translation.y < -0.05,
					"stomp drops hips via offset y={}",
					effects.0.translation.y
				);
			}
			if (progress - 0.5).abs() < f32::EPSILON {
				assert!(
					hand_r.z > windup_hand_r.z + 0.25,
					"strike extends the lead fist past the chamber: chamber z={} strike z={}",
					windup_hand_r.z,
					hand_r.z
				);
				assert!(
					hand_r.z > start_hand_r.z + 0.15,
					"strike sends the lead hand fight-forward: start z={} strike z={}",
					start_hand_r.z,
					hand_r.z
				);
				assert!(
					rig.posed_angle("forearm.R") < 0.35,
					"earthbending strike keeps the lead elbow near straight"
				);
			}
			if (progress - 0.65).abs() < f32::EPSILON {
				assert!(
					head.z > pelvis.z + 0.08,
					"dash pitches the torso: head z={} pelvis z={}",
					head.z,
					pelvis.z
				);
				assert!(
					hand_l.z < hand_r.z - 0.15,
					"dash keeps the trailing arm back: trail z={} lead z={}",
					hand_l.z,
					hand_r.z
				);
				assert!(
					rig.posed_angle("shin.R") > 0.25 && rig.posed_angle("shin.L") > 0.25,
					"dash stays low"
				);
				assert!(
					effects.0.translation.z > 0.04,
					"mid-dash still carries forward offset z={}",
					effects.0.translation.z
				);
			}
			if (progress - 0.8).abs() < f32::EPSILON {
				assert!(
					effects.0.translation.z > 0.02,
					"held dash still carries a modest forward offset z={}",
					effects.0.translation.z
				);
			}
			if progress == 1.0 {
				assert!(effects.is_identity());
			}
		}

		let (_, effects_before_end) = apply(59.0 / 60.0);
		let (_, effects_at_end) = apply(1.0);
		let handoff_delta =
			(effects_at_end.0.translation - effects_before_end.0.translation).length();
		eprintln!(
			"\nmax per-frame ArmatureOffset translation delta @ 60 Hz: {max_offset_delta_60hz:.4} m"
		);
		eprintln!("offset delta last frame -> progress 1.0: {handoff_delta:.4} m");
		eprintln!(
			"wind-up hand.R z={:.3}; strike hand.R z={:.3}; dash head-pelvis z={:.3}",
			windup_hand_r.z,
			strike_hand_r.z,
			dash_head.z - dash_pelvis.z
		);
		assert!(handoff_delta < 0.02, "hand-off frame should not pop, delta={handoff_delta}");
		assert!(max_offset_delta_60hz < 0.05, "offset delta={max_offset_delta_60hz}");
	}
}
