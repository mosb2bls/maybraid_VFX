//! Humanoid mapping for [`UprightPowerDash`](crate::animations::UprightPowerDash).
//!
//! Cartoon earthbending dash: chambered horse-stance wind-up, a stomping
//! straight-arm strike, then a low hip-driven dash that settles back to rest.

use std::f32::consts::FRAC_PI_2;

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmAim, ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{
	smoothstep, UprightPowerDash, JAB_PHASE_END, JAB_PROGRESS_AT_HANDOFF, STRIDE_PHASE_END,
};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects, Progress};

const STRIDE_BLEND: f32 = 0.18;
/// Slide-phase progress where the dash hold yields to settle.
const SLIDE_SETTLE_START: f32 = 0.48;
const WINDUP_DROP: f32 = 0.34;
const STOMP_DROP: f32 = 0.42;
const DASH_DROP: f32 = 0.38;
const DASH_FORWARD: f32 = 0.10;
const STOMP_LIFT_START: f32 = 0.22;
const STOMP_LIFT_PEAK: f32 = 0.28;
const STOMP_LIFT_END: f32 = 0.34;

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

/// Brief trail-foot stomp lift. Zero at the requested review frames (0.20, 0.35+).
fn stomp_lift(progress: f32) -> f32 {
	if progress <= STOMP_LIFT_START || progress >= STOMP_LIFT_END {
		0.0
	} else if progress < STOMP_LIFT_PEAK {
		smoothstep((progress - STOMP_LIFT_START) / (STOMP_LIFT_PEAK - STOMP_LIFT_START))
	} else {
		1.0 - smoothstep((progress - STOMP_LIFT_PEAK) / (STOMP_LIFT_END - STOMP_LIFT_PEAK))
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

	fn punch_roll(&self, side: Side) -> f32 {
		FRAC_PI_2 * side.sign()
	}

	/// Punching hand drawn back. Roll 0 keeps the fist on its own side.
	fn chamber_along(&self) -> Vec3 {
		Vec3::new(-0.12 * self.lead().sign(), -0.18, -1.0)
	}

	/// Straight-arm punch / palm along +Z at shoulder height, inboard.
	fn strike_along(&self) -> Vec3 {
		Vec3::new(-0.22 * self.lead().sign(), 0.28, 1.0)
	}

	/// Off-hand at the hip: inboard, slightly down, slightly back.
	fn hip_chamber_along(&self, side: Side) -> Vec3 {
		Vec3::new(-0.22 * side.sign(), -0.22, -0.18)
	}

	/// Strike arm stays forward and inboard through the dash.
	fn dash_lead_along(&self) -> Vec3 {
		Vec3::new(-0.18 * self.lead().sign(), 0.16, 0.88)
	}

	fn apply_hip_chamber(&self, pose: &mut HumanoidPose, side: Side, amount: f32) {
		if amount < 1e-4 {
			return;
		}
		apply_aimed_arm(
			pose,
			side,
			self.hip_chamber_along(side),
			0.0,
			0.04 * amount,
			1.05 * amount,
		);
	}

	fn apply_horse_legs(
		&self,
		pose: &mut HumanoidPose,
		lead_f: f32,
		trail_f: f32,
		lead_k: f32,
		trail_k: f32,
		abduct: f32,
		lead_fold: f32,
		trail_fold: f32,
	) {
		let lead = self.lead();
		let trail = self.trail();
		pose.apply_leg(lead, lead_f, lead_k);
		pose.apply_leg(trail, trail_f, trail_k);
		// Femur flex X is mirrored on the right; sign so both feet go wide.
		pose.leg_mut(lead).hip_abduction += abduct * lead.sign();
		pose.leg_mut(trail).hip_abduction += abduct * trail.sign();
		pose.apply_hip_fold(lead, lead_fold);
		pose.apply_hip_fold(trail, trail_fold);
	}

	fn apply_look_forward(&self, pose: &mut HumanoidPose, lean: f32) {
		// Negative nod counters a hip lean so the head stays up, looking +Z.
		let nod = (-0.55 * lean).clamp(-0.28, 0.0);
		pose.apply_neck_twisted(0.0, 0.0, nod, 0.0, 0.0, nod * 0.65);
	}

	fn apply_punch_turn(&self, pose: &mut HumanoidPose, amount: f32) {
		let yaw = amount * -self.lead().sign();
		pose.spine.add_turn(yaw);
		pose.leg_mut(self.lead()).pelvis_turn += yaw * 0.55;
		pose.leg_mut(self.trail()).pelvis_turn += yaw * 0.28;
	}

	fn apply_windup_pose(&self, rig: &mut HumanoidV0Rig, u: f32, lift: f32) {
		let u = u.clamp(0.0, 1.0);
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		// Squat-signed femurs plant both feet; a short positive trail swing is the stomp lift.
		let lead_f = -0.64 * u;
		let trail_f = lerp(-0.54 * u, 0.22, lift);
		let lead_k = self.windup_knee * u;
		let trail_k = lerp(self.windup_knee * 0.96 * u, 0.26, lift);
		self.apply_horse_legs(
			&mut pose,
			lead_f,
			trail_f,
			lead_k,
			trail_k,
			self.hip_abduct * u,
			0.22 * u,
			0.20 * u * (1.0 - lift),
		);
		pose.apply_root(0.10 * u);
		if u > 1e-4 {
			apply_aimed_arm(&mut pose, lead, self.chamber_along(), 0.0, -0.20 * u, 0.95 * u);
			self.apply_hip_chamber(&mut pose, trail, u);
		}
		self.apply_look_forward(&mut pose, 0.08 * u);
		rig.write_pose(&pose);
	}

	fn apply_stride_pose(&self, rig: &mut HumanoidV0Rig, u: f32, lift: f32) {
		let u = u.clamp(0.0, 1.0);
		let stomp = 1.0 - (1.0 - u).powi(3);
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();

		// Both femurs stay squat-signed so the feet stay on the ground. The
		// trail only lifts during `lift`. Modest extra lead squat reads as a
		// short forward bias, not a long lunge.
		let lead_f = lerp(-0.64, -self.lead_stride, stomp);
		let trail_f = lerp(-0.54, -self.trail_stride, stomp) + 0.72 * lift;
		let lead_k = lerp(self.windup_knee, self.lunge_knee_lead, stomp);
		let trail_k = lerp(self.windup_knee * 0.96, self.lunge_knee_trail, stomp)
			* (1.0 - 0.70 * lift)
			+ 0.24 * lift;
		self.apply_horse_legs(
			&mut pose,
			lead_f,
			trail_f,
			lead_k,
			trail_k,
			self.hip_abduct,
			lerp(0.22, 0.16, stomp),
			lerp(0.20, 0.14, stomp) * (1.0 - lift),
		);
		pose.apply_root(lerp(0.10, self.lunge_lean, stomp));

		let strike = 1.0 - (1.0 - stomp).powi(2);
		apply_aimed_arm(
			&mut pose,
			lead,
			lerp_vec3(self.chamber_along(), self.strike_along(), strike),
			lerp(0.0, self.punch_roll(lead), strike),
			lerp(-0.20, 0.08 * self.arm_reach, strike),
			lerp(0.95, 0.05, strike),
		);
		self.apply_hip_chamber(&mut pose, trail, 1.0);
		self.apply_punch_turn(&mut pose, 0.12 * strike);
		self.apply_look_forward(&mut pose, lerp(0.10, self.lunge_lean, stomp));
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
		if settle >= 0.999 {
			rig.write_pose(&HumanoidPose::default());
			return;
		}
		let mut pose = HumanoidPose::default();
		let lead = self.lead();
		let trail = self.trail();
		let hold = 1.0 - settle;

		let lead_f = lerp(-self.lead_stride, 0.0, settle);
		let trail_f = lerp(-self.trail_stride, 0.0, settle);
		let lead_k = lerp(lerp(self.lunge_knee_lead, self.slide_knee, enter), 0.0, settle);
		let trail_k = lerp(lerp(self.lunge_knee_trail, self.slide_knee * 0.92, enter), 0.0, settle);
		self.apply_horse_legs(
			&mut pose,
			lead_f,
			trail_f,
			lead_k,
			trail_k,
			self.hip_abduct * hold,
			0.20 * hold,
			0.18 * hold,
		);

		let lean = lerp(self.lunge_lean, self.slide_lean, enter) * hold;
		pose.apply_root(lean);

		if settle < 0.90 {
			apply_aimed_arm(
				&mut pose,
				lead,
				lerp_vec3(self.strike_along(), self.dash_lead_along(), enter),
				self.punch_roll(lead),
				0.08 * self.arm_reach * hold,
				lerp(0.05, 0.22, enter) * hold,
			);
			self.apply_hip_chamber(&mut pose, trail, hold);
		}
		self.apply_punch_turn(&mut pose, 0.18 * hold);
		self.apply_look_forward(&mut pose, lean);
		rig.write_pose(&pose);
	}
}

struct WindUpAt<'a> {
	dash: &'a UprightPowerDash,
	lift: f32,
}

impl Animation<HumanoidV0Rig> for WindUpAt<'_> {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		self.dash.apply_windup_pose(rig, progress, self.lift);
	}
}

struct StrideAt<'a> {
	dash: &'a UprightPowerDash,
	lift: f32,
}

impl Animation<HumanoidV0Rig> for StrideAt<'_> {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		self.dash.apply_stride_pose(rig, progress, self.lift);
	}
}

impl Animation<HumanoidV0Rig> for UprightPowerDash {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, progress: f32) {
		let t = Progress(progress).clamp();
		let lift = stomp_lift(t);
		if t < JAB_PHASE_END {
			let u = t / JAB_PHASE_END;
			let wind = attack_hold(u, 0.72);
			if u < 0.48 {
				blend_clips(
					rig,
					&self.jab,
					JAB_PROGRESS_AT_HANDOFF * (u / 0.48),
					&WindUpAt { dash: self, lift },
					wind,
					smoothstep(u / 0.48),
				);
				return;
			}
			self.apply_windup_pose(rig, wind, lift);
			return;
		}
		if t < STRIDE_PHASE_END {
			let phase = (t - JAB_PHASE_END) / (STRIDE_PHASE_END - JAB_PHASE_END);
			if phase < STRIDE_BLEND {
				let weight = smoothstep(phase / STRIDE_BLEND);
				blend_clips(
					rig,
					&WindUpAt { dash: self, lift },
					1.0,
					&StrideAt { dash: self, lift },
					0.0,
					weight,
				);
				return;
			}
			let stride_u = attack_hold((phase - STRIDE_BLEND) / (1.0 - STRIDE_BLEND), 0.42);
			self.apply_stride_pose(rig, stride_u, lift);
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
	use crate::animations::{PowerDash, Run, Transition};
	use crate::rigs::transition::capture_animation_pose;
	use character_rigs::authoring::ArmatureOffset;

	const REST_FOOT_Y: f32 = -1.5;
	const PLANTED_FOOT_SLACK: f32 = 0.20;

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

	fn world_of(tail: Vec3, offset: Vec3) -> Vec3 {
		tail + offset
	}

	fn pelvis_center(rig: &HumanoidV0Rig) -> Vec3 {
		rig.character_point("lumbar")
	}

	fn head_tail(rig: &HumanoidV0Rig) -> Vec3 {
		bone_tail(rig, "upper_neck")
	}

	fn rest_rig() -> HumanoidV0Rig {
		HumanoidV0Rig::for_clip_test()
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
	fn stride_drops_hips_and_opens_a_horse_stance() {
		let (rig, effects) = apply(0.42);
		assert!(effects.0.translation.y < -0.22, "stomp lowers the armature");
		let foot_l = bone_tail(&rig, "shin.L");
		let foot_r = bone_tail(&rig, "shin.R");
		assert!(
			(foot_l.x - foot_r.x).abs() > 0.55,
			"horse stance is laterally wide, L={foot_l:?} R={foot_r:?}"
		);
		assert!(rig.posed_angle("shin.R") > 0.35 && rig.posed_angle("shin.L") > 0.35);
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
		assert!(rig.posed_angle("femur.R") < 0.12);
		assert!(
			effects.is_identity(),
			"clip end offset must be identity for AnimId hand-off, got {:?}",
			effects.0.translation
		);
	}

	#[test]
	fn posed_joint_evidence_at_progress_samples() {
		let rest = rest_rig();
		let rest_hand_r = bone_tail(&rest, "forearm.R");
		let rest_hand_l = bone_tail(&rest, "forearm.L");
		let rest_foot_r = bone_tail(&rest, "shin.R");
		let rest_foot_l = bone_tail(&rest, "shin.L");
		let (start_rig, _) = apply(0.0);
		let start_hand_r = bone_tail(&start_rig, "forearm.R");
		let samples = [0.0, 0.1, 0.2, 0.35, 0.5, 0.65, 0.8, 1.0];

		eprintln!(
			"\n| progress | hand.R world | hand.L world | foot.R world | foot.L world | pelvis | head | offset (x,y,z) |"
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

		let (windup_rig, windup_fx) = apply(0.20);
		let windup_hand_r = world_of(bone_tail(&windup_rig, "forearm.R"), windup_fx.0.translation);
		let (strike_rig, strike_fx) = apply(0.50);
		let strike_hand_r = world_of(bone_tail(&strike_rig, "forearm.R"), strike_fx.0.translation);
		let (dash_rig, dash_fx) = apply(0.65);
		let dash_head = world_of(head_tail(&dash_rig), dash_fx.0.translation);
		let dash_pelvis = world_of(pelvis_center(&dash_rig), dash_fx.0.translation);

		for progress in samples {
			let (rig, effects) = apply(progress);
			let off = effects.0.translation;
			let hand_r = world_of(bone_tail(&rig, "forearm.R"), off);
			let hand_l = world_of(bone_tail(&rig, "forearm.L"), off);
			let foot_r = world_of(bone_tail(&rig, "shin.R"), off);
			let foot_l = world_of(bone_tail(&rig, "shin.L"), off);
			let pelvis = world_of(pelvis_center(&rig), off);
			let head = world_of(head_tail(&rig), off);
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

			let planted = |foot: Vec3| (foot.y - REST_FOOT_Y).abs() <= PLANTED_FOOT_SLACK;
			let expect_planted = progress <= 0.20 + f32::EPSILON
				|| (progress - 0.35).abs() < f32::EPSILON
				|| progress >= 0.50 - f32::EPSILON;

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
					planted(foot_r) && planted(foot_l),
					"horse stance keeps both feet near the ground: R={} L={}",
					foot_r.y,
					foot_l.y
				);
				assert!(
					(foot_r.x - foot_l.x).abs() > 0.55,
					"wind-up horse is laterally wide: R.x={} L.x={}",
					foot_r.x,
					foot_l.x
				);
				assert!(
					effects.0.translation.y < -0.18,
					"wind-up drops the hips y={}",
					effects.0.translation.y
				);
			}
			if (progress - 0.35).abs() < f32::EPSILON || (progress - 0.5).abs() < f32::EPSILON {
				assert!(
					planted(foot_r) && planted(foot_l),
					"stomp/strike keeps both feet planted: R={} L={}",
					foot_r.y,
					foot_l.y
				);
				assert!((foot_r.x - foot_l.x).abs() > 0.55, "stomp/strike horse is laterally wide");
				assert!(
					effects.0.translation.y < -0.28,
					"stomp drops hips via offset y={}",
					effects.0.translation.y
				);
				assert!(
					effects.0.translation.y > -0.52,
					"hip drop stays in the 0.3–0.5 band, y={}",
					effects.0.translation.y
				);
			}
			if (progress - 0.5).abs() < f32::EPSILON {
				assert!(
					hand_r.z > windup_hand_r.z + 0.45,
					"strike extends the lead fist past the chamber: chamber z={} strike z={}",
					windup_hand_r.z,
					hand_r.z
				);
				assert!(
					hand_r.x.abs() < 0.3,
					"strike fist stays in line with the shoulder, |x|={}",
					hand_r.x.abs()
				);
				assert!(hand_r.y > 0.45, "strike is near shoulder height, y={}", hand_r.y);
				assert!(
					hand_l.y < hand_r.y - 0.25,
					"off-hand is chambered at the hip, L.y={} R.y={}",
					hand_l.y,
					hand_r.y
				);
				assert!(
					hand_l.x.abs() < 0.55,
					"off-hand is not flared out to a T, L.x={}",
					hand_l.x
				);
				assert!(
					rig.posed_angle("forearm.R") < 0.35,
					"earthbending strike keeps the lead elbow near straight"
				);
			}
			if (progress - 0.65).abs() < f32::EPSILON {
				assert!(
					head.z > pelvis.z + 0.08,
					"dash pitches from the hips: head z={} pelvis z={}",
					head.z,
					pelvis.z
				);
				assert!(
					head.y - pelvis.y > 1.15,
					"dash keeps a straight back and head up: head y={} pelvis y={}",
					head.y,
					pelvis.y
				);
				assert!(head.y > 1.05, "dash head stays up looking forward, y={}", head.y);
				assert!(
					hand_l.z < hand_r.z - 0.15,
					"dash keeps the trailing arm back: trail z={} lead z={}",
					hand_l.z,
					hand_r.z
				);
				assert!(
					hand_l.x.abs() < 0.70 && hand_r.x.abs() < 0.55,
					"dash arms are not both out sideways: L.x={} R.x={}",
					hand_l.x,
					hand_r.x
				);
				assert!(
					planted(foot_r) && planted(foot_l),
					"dash stays rooted: R={} L={}",
					foot_r.y,
					foot_l.y
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
				assert!(
					planted(foot_r) && planted(foot_l),
					"held dash stays rooted: R={} L={}",
					foot_r.y,
					foot_l.y
				);
			}
			if progress == 1.0 {
				assert!(effects.is_identity());
				assert!(
					planted(foot_r) && planted(foot_l),
					"settle plants both feet: R={} L={}",
					foot_r.y,
					foot_l.y
				);
			}
			if expect_planted && progress > 0.0 && progress < 1.0 {
				let _ = planted;
			}
		}

		let (end_rig, effects_at_end) = apply(1.0);
		let mut max_angle = 0.0f32;
		let mut max_angle_bone = "";
		eprintln!("\n| bone | posed_angle vs rest @ 1.0 | tail delta |");
		eprintln!("| --- | --- | --- |");
		for name in end_rig.animation_bone_names() {
			let angle = end_rig.posed_angle(name);
			let tail = bone_tail(&end_rig, name);
			let rest_tail = bone_tail(&rest, name);
			let delta = (tail - rest_tail).length();
			if angle > max_angle {
				max_angle = angle;
				max_angle_bone = name;
			}
			eprintln!("| {name} | {angle:.4} rad | {delta:.4} m |");
			assert!(angle < 0.12, "{name} should settle near rest, posed_angle={angle}");
		}
		let end_hand_r = bone_tail(&end_rig, "forearm.R");
		let end_hand_l = bone_tail(&end_rig, "forearm.L");
		eprintln!(
			"\nrest hand.R {:?}; end hand.R {:?}; Δ={}",
			rest_hand_r,
			end_hand_r,
			(end_hand_r - rest_hand_r).length()
		);
		eprintln!(
			"rest hand.L {:?}; end hand.L {:?}; Δ={}",
			rest_hand_l,
			end_hand_l,
			(end_hand_l - rest_hand_l).length()
		);
		eprintln!("rest foot.R {:?}; rest foot.L {:?}", rest_foot_r, rest_foot_l);
		eprintln!("max posed_angle vs rest @ 1.0: {max_angle:.4} rad on {max_angle_bone}");

		let (_, effects_before_end) = apply(59.0 / 60.0);
		let handoff_delta =
			(effects_at_end.0.translation - effects_before_end.0.translation).length();
		eprintln!(
			"\nmax per-frame ArmatureOffset translation delta @ 60 Hz: {max_offset_delta_60hz:.4} m"
		);
		eprintln!("offset delta last frame -> progress 1.0: {handoff_delta:.4} m");
		eprintln!(
			"wind-up hand.R z={:.3}; strike hand.R z={:.3}; dash head-pelvis y={:.3} z={:.3}",
			windup_hand_r.z,
			strike_hand_r.z,
			dash_head.y - dash_pelvis.y,
			dash_head.z - dash_pelvis.z
		);
		assert!(handoff_delta < 0.02, "hand-off frame should not pop, delta={handoff_delta}");
		assert!(max_offset_delta_60hz < 0.05, "offset delta={max_offset_delta_60hz}");
		assert!((end_hand_r - rest_hand_r).length() < 0.25, "end hand.R should be near rest");
	}
}
