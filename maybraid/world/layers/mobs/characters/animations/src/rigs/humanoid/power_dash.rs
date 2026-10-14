//! Humanoid mapping for [`UprightPowerDash`](crate::animations::UprightPowerDash).

use bevy::prelude::Vec3;
use character_rigs::authoring::{ArmatureOffset, HumanoidPose};
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use character_rigs::Side;

use crate::animations::{
	smoothstep, PowerDash, UprightPowerDash, JAB_PHASE_END, JAB_PROGRESS_AT_HANDOFF,
	STRIDE_PHASE_END,
};
use crate::rigs::mix::blend_clips;
use crate::{Animation, Effects, Progress};

const STRIDE_BLEND: f32 = 0.22;
const JAB_SNAP_PROGRESS: f32 = 0.40;

fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t
}

impl UprightPowerDash {
	fn apply_stride_pose(&self, rig: &mut HumanoidV0Rig, u: f32) {
		let u = u.clamp(0.0, 1.0);
		let deepen = (u * std::f32::consts::PI * 0.5).sin();
		let ease_out = 1.0 - (1.0 - u).powi(2);
		let mut pose = HumanoidPose::default();

		let lead = Side::Right;
		let trail = Side::Left;
		let lead_femur = -self.lead_stride * deepen;
		let trail_femur = self.trail_stride * deepen;
		let lead_knee = self.lunge_knee_lead * deepen;
		let trail_knee = self.lunge_knee_trail * deepen * 0.85;

		pose.apply_leg(lead, lead_femur, lead_knee);
		pose.apply_leg(trail, trail_femur, trail_knee);
		pose.apply_root(self.lunge_lean * ease_out);

		let reach = self.arm_reach * deepen;
		pose.apply_arm(lead, 0.0, 0.0, reach, 0.0, 0.85);
		pose.apply_arm(trail, 0.0, 0.0, -reach * 0.75, 0.0, 1.05);
		rig.write_pose(&pose);
	}

	fn apply_slide_pose(&self, rig: &mut HumanoidV0Rig, u: f32) {
		let u = smoothstep(u.clamp(0.0, 1.0));
		let low = 1.0 - u * 0.35;
		let mut pose = HumanoidPose::default();

		let stride_lead_f = -self.lead_stride;
		let stride_trail_f = self.trail_stride;
		let stride_lead_k = self.lunge_knee_lead;
		let stride_trail_k = self.lunge_knee_trail * 0.85;
		let slide_lead_f = -0.55 * low;
		let slide_trail_f = slide_lead_f * 0.95;
		let slide_knee = self.slide_knee * low;

		pose.apply_leg(
			Side::Right,
			lerp(stride_lead_f, slide_lead_f, u),
			lerp(stride_lead_k, slide_knee * 1.05, u),
		);
		pose.apply_leg(
			Side::Left,
			lerp(stride_trail_f, slide_trail_f, u),
			lerp(stride_trail_k, slide_knee, u),
		);
		pose.apply_root(lerp(self.lunge_lean, self.slide_lean * low, u));

		let reach = self.arm_reach;
		let tuck = 0.35 * low;
		pose.apply_arm(Side::Right, 0.0, 0.0, lerp(reach, tuck * 0.8, u), 0.0, lerp(0.85, 1.0, u));
		pose.apply_arm(
			Side::Left,
			0.0,
			0.0,
			lerp(-reach * 0.75, -tuck, u),
			0.0,
			lerp(1.05, 1.1, u),
		);
		rig.write_pose(&pose);
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
			let u = smoothstep(t / JAB_PHASE_END);
			let jab_progress = u * JAB_SNAP_PROGRESS;
			self.jab.apply_for(rig, jab_progress);
			return;
		}
		if t < STRIDE_PHASE_END {
			let phase = (t - JAB_PHASE_END) / (STRIDE_PHASE_END - JAB_PHASE_END);
			if phase < STRIDE_BLEND {
				let weight = smoothstep(phase / STRIDE_BLEND);
				blend_clips(
					rig,
					&self.jab,
					JAB_PROGRESS_AT_HANDOFF,
					&StrideAt { dash: self },
					0.0,
					weight,
				);
				return;
			}
			let stride_u = smoothstep((phase - STRIDE_BLEND) / (1.0 - STRIDE_BLEND));
			self.apply_stride_pose(rig, stride_u);
			return;
		}
		let slide_u =
			smoothstep((t - STRIDE_PHASE_END) / (1.0 - STRIDE_PHASE_END).max(f32::EPSILON));
		self.apply_slide_pose(rig, slide_u);
	}

	fn effects_for(&self, _rig: &HumanoidV0Rig, progress: f32) -> Effects {
		let t = Progress(progress).clamp();
		if t < JAB_PHASE_END {
			return Effects::IDENTITY;
		}
		if t < STRIDE_PHASE_END {
			let phase = (t - JAB_PHASE_END) / (STRIDE_PHASE_END - JAB_PHASE_END);
			let gather = (phase * std::f32::consts::PI).sin().max(0.0);
			let drop = 0.11 * self.drive * gather;
			if drop > f32::EPSILON {
				return ArmatureOffset::from_translation(Vec3::new(0.0, -drop, 0.0));
			}
			return Effects::IDENTITY;
		}
		let u = smoothstep((t - STRIDE_PHASE_END) / (1.0 - STRIDE_PHASE_END).max(f32::EPSILON));
		// Bell envelope: zero at slide entry/exit so mailbox hand-off to locomotion clips
		// does not fight capsule travel with a lingering +Z offset.
		let envelope = (u * std::f32::consts::PI).sin().max(0.0);
		let forward = 0.24 * self.slide * envelope;
		let drop = 0.09 * self.drive * envelope;
		ArmatureOffset::from_translation(Vec3::new(0.0, -drop, forward))
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

	#[test]
	fn opening_jab_moves_the_lead_arm() {
		let (rig, effects) = apply(0.12);
		let idle = HumanoidV0Rig::for_clip_test();
		assert!(effects.is_identity());
		assert!(
			rig.posed_angle("humerus.R") > idle.posed_angle("humerus.R") + 0.02,
			"jab phase drives the lead humerus"
		);
		assert!(rig.posed_angle("femur.R") > 0.05);
	}

	#[test]
	fn stride_drops_hips_and_splits_legs() {
		let (rig, effects) = apply(0.42);
		assert!(effects.0.translation.y < -0.02, "stride lowers the armature");
		let left = rig.character_length("femur.L");
		let right = rig.character_length("femur.R");
		assert!(
			(left.z - right.z).abs() > 0.2,
			"stride keeps a deep split, L={left:?} R={right:?}"
		);
	}

	#[test]
	fn slide_carries_forward_offset() {
		let (_, effects) = apply(0.75);
		assert!(effects.0.translation.z > 0.08, "slide offsets fight-forward");
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
			rig_r.posed_angle("forearm.R") < rig_l.posed_angle("forearm.L") + 0.1,
			"lead elbow extension follows jab side"
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

	fn bone_tail(rig: &HumanoidV0Rig, bone: &str) -> Vec3 {
		rig.character_point(bone) + rig.character_length(bone)
	}

	fn hips_center(rig: &HumanoidV0Rig) -> Vec3 {
		(rig.character_point("femur.L") + rig.character_point("femur.R")) * 0.5
	}

	fn sample_row(progress: f32) -> (HumanoidV0Rig, Effects) {
		apply(progress)
	}

	#[test]
	fn posed_joint_evidence_at_progress_samples() {
		let idle = HumanoidV0Rig::for_clip_test();
		let idle_hand_r = bone_tail(&idle, "forearm.R");
		let samples = [0.0, 0.15, 0.3, 0.45, 0.6, 0.8, 1.0];

		eprintln!(
			"\n| progress | hand.R (x,y,z) | hand.L | foot.R | foot.L | hips | offset (x,y,z) |"
		);
		eprintln!("| --- | --- | --- | --- | --- | --- | --- |");

		let mut prev_offset = Effects::IDENTITY;
		let mut max_offset_delta_60hz = 0.0f32;
		for frame in 0..=60 {
			let t = frame as f32 / 60.0;
			let (_, effects) = apply(t);
			let delta = (effects.0.translation - prev_offset.0.translation).length();
			max_offset_delta_60hz = max_offset_delta_60hz.max(delta);
			prev_offset = effects;
		}

		for progress in samples {
			let (rig, effects) = sample_row(progress);
			let hand_r = bone_tail(&rig, "forearm.R");
			let hand_l = bone_tail(&rig, "forearm.L");
			let foot_r = bone_tail(&rig, "shin.R");
			let foot_l = bone_tail(&rig, "shin.L");
			let hips = hips_center(&rig);
			let off = effects.0.translation;
			eprintln!(
				"| {progress:.2} | ({hrx:.3},{hry:.3},{hrz:.3}) | ({hlx:.3},{hly:.3},{hlz:.3}) | ({frx:.3},{fry:.3},{frz:.3}) | ({flx:.3},{fly:.3},{flz:.3}) | ({hx:.3},{hy:.3},{hz:.3}) | ({ox:.3},{oy:.3},{oz:.3}) |",
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
				hx = hips.x,
				hy = hips.y,
				hz = hips.z,
				ox = off.x,
				oy = off.y,
				oz = off.z,
			);

			if progress == 0.15 {
				assert!(
					hand_r.z > idle_hand_r.z + 0.05,
					"jab extends lead hand forward: idle z={} jab z={}",
					idle_hand_r.z,
					hand_r.z
				);
			}
			if (progress - 0.45).abs() < f32::EPSILON {
				let split = foot_r.z - foot_l.z;
				assert!(split > 0.25, "power stride splits feet forward/back: {split}");
				assert!(
					rig.posed_angle("femur.R") > rig.posed_angle("femur.L") + 0.15,
					"lead knee flexed deeper than trail in lunge"
				);
				assert!(
					effects.0.translation.y < -0.02,
					"stride drops hips via offset y={}",
					effects.0.translation.y
				);
			}
			if progress == 0.8 {
				let (stride_rig, _) = apply(0.57);
				let stride_trail_y = bone_tail(&stride_rig, "shin.L").y;
				assert!(
					foot_l.y < stride_trail_y - 0.15,
					"slide drops trail foot toward ground: stride y={} slide y={}",
					stride_trail_y,
					foot_l.y
				);
				assert!(
					rig.posed_angle("shin.R") > 0.2 && rig.posed_angle("shin.L") > 0.2,
					"slide keeps knees flexed"
				);
				assert!(
					effects.0.translation.z > 0.04,
					"mid-slide still carries forward offset z={}",
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
		assert!(handoff_delta < 0.02, "hand-off frame should not pop, delta={handoff_delta}");
		assert!(max_offset_delta_60hz < 0.05, "offset delta={max_offset_delta_60hz}");
	}
}
