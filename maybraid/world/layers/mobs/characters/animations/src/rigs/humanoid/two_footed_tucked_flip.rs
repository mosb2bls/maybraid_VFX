use bevy::prelude::{Quat, Transform, Vec3};
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use log::info;

use crate::animations::{
	smoothstep, FixedPosition, JumpSegment, Spring, Squat, Tuck, TwoFootedTuckedFlip,
	FALL_BLEND_FRACTION,
};
use crate::rigs::mix::blend_clips;
use crate::rigs::segment_debug::segment_debug_enabled;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for TwoFootedTuckedFlip {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let lengths = rig.segment_lengths;
		let (segment, local) = self.segment(lengths, elapsed);
		let timings = self.timings(lengths);
		let jump = &self.jump;
		let flip = &self.flip;

		match segment {
			JumpSegment::Squat => {
				let squat = jump.prejump_squat(lengths);
				let progress = local / timings.squat_duration().max(f32::EPSILON);
				squat.apply_for(rig, progress);
			}
			JumpSegment::Spring => {
				blend_clips(
					rig,
					&Squat::for_loop(1.0, 1.0),
					0.0,
					&Spring::default(),
					local,
					smoothstep(local),
				);
			}
			JumpSegment::Fall => {
				let tuck = Tuck::new(flip.tuck.tightness());
				let blend_end = FALL_BLEND_FRACTION;
				if segment_debug_enabled() && local > 0.9 {
					info!(
						"tucked flip air end: elapsed={:.3} flip_local={:.4} pitch={:.4}",
						elapsed,
						local,
						flip.pitch_radians(local),
					);
				}
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					blend_clips(
						rig,
						&Spring::default(),
						1.0,
						&tuck,
						1.0,
						smoothstep(transition_progress),
					);
				} else {
					let _ = flip.tuck.apply_fixed(rig);
				}
			}
			JumpSegment::Land => {
				let land = jump.landing_squat(lengths);
				let land_duration = timings.land_duration().max(f32::EPSILON);
				let land_progress = local / land_duration;
				let blend_window = timings.land_pose_blend_duration();
				let transition_progress = if blend_window > f32::EPSILON {
					(local / blend_window).clamp(0.0, 1.0)
				} else {
					1.0
				};
				if segment_debug_enabled() && local < timings.land_descent_duration + 0.05 {
					info!(
						"tucked flip land start: elapsed={:.3} land_local={:.4} transition={:.4}",
						elapsed, local, transition_progress,
					);
				}
				if transition_progress < 1.0 {
					blend_clips(
						rig,
						&flip.tuck,
						0.0,
						&land,
						land_progress,
						smoothstep(transition_progress),
					);
				} else {
					land.apply_for(rig, land_progress);
				}
			}
		}
	}

	fn effects_for(&self, rig: &HumanoidV0Rig, elapsed: f32) -> Effects {
		let y = self.vertical_offset(rig.segment_lengths, elapsed);
		let pitch = self.flip_pitch_radians(rig.segment_lengths, elapsed);
		if y.abs() <= f32::EPSILON && pitch.abs() <= f32::EPSILON {
			return ArmatureOffset::IDENTITY;
		}
		ArmatureOffset(Transform {
			translation: Vec3::new(0.0, y, 0.0),
			rotation: Quat::from_rotation_x(pitch),
			scale: Vec3::ONE,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::DEFAULT_SPRING_DURATION;

	fn default_flip() -> TwoFootedTuckedFlip {
		TwoFootedTuckedFlip::default()
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let elapsed = flip.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		flip.apply(&mut rig, elapsed);

		assert!(rig.posed_angle("femur.L") < 0.05, "femur should be straight");
		assert!(rig.posed_angle("shin.L") < 0.05, "shin should be straight");
		Ok(())
	}

	#[test]
	fn mid_air_applies_tuck_and_forward_pitch() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let timings = flip.timings(lengths);
		let elapsed = timings.spring_end() + timings.air_duration * 0.5;
		let effects = flip.apply(&mut rig, elapsed);

		assert!(rig.posed_angle("shin.L") > 0.5, "knee folds in the tuck");

		assert!(effects.0.translation.y > 0.0);
		assert!(effects.0.rotation.to_euler(bevy::prelude::EulerRot::XYZ).0 > 0.0);
		Ok(())
	}

	#[test]
	fn land_blends_leg_compression_with_tuck_arms() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let timings = flip.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		flip.apply(&mut rig, timings.air_end() + blend * 0.5);

		let mut tucked = HumanoidV0Rig::imported();
		flip.flip.tuck.apply_fixed(&mut tucked);
		assert!(
			rig.posed_angle("shoulder.L") < tucked.posed_angle("shoulder.L"),
			"landing blend is short of the held tuck"
		);
		assert!(rig.posed_angle("femur.L") > 0.01, "legs should be partway into landing squat");
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let flip = default_flip();
		let lengths = rig.segment_lengths;
		let timings = flip.timings(lengths);
		flip.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		assert!(rig.posed_angle("femur.L") > 0.01);
		Ok(())
	}
}
