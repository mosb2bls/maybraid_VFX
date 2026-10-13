use bevy::prelude::Vec3;
use character_rigs::authoring::ArmatureOffset;
use character_rigs::rigs::humanoid_v0::HumanoidV0Rig;
use log::info;

use crate::animations::{
	smoothstep, Fall, JumpSegment, Spring, Squat, TwoFootedJump, FALL_BLEND_FRACTION,
};
use crate::rigs::mix::blend_clips;
use crate::rigs::segment_debug::segment_debug_enabled;
use crate::{Animation, Effects};

impl Animation<HumanoidV0Rig> for TwoFootedJump {
	fn apply_for(&self, rig: &mut HumanoidV0Rig, elapsed: f32) {
		let lengths = rig.segment_lengths;
		let (segment, local) = self.segment(lengths, elapsed);
		let timings = self.timings(lengths);

		match segment {
			JumpSegment::Squat => {
				let squat = self.prejump_squat(lengths);
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
				let fall = Fall::default();
				let blend_end = FALL_BLEND_FRACTION;
				if segment_debug_enabled() && local > 0.9 {
					info!(
						"jump fall end: elapsed={:.3} cycle_t={:.3} air_end={:.3} fall_local={:.4} fall_femur=0 fall_shoulder_flex={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						timings.air_end(),
						local,
						fall.shoulder_flex(character_rigs::Side::Left, local),
					);
				}
				if local < blend_end {
					let transition_progress = (local / blend_end).clamp(0.0, 1.0);
					blend_clips(
						rig,
						&Spring::default(),
						1.0,
						&fall,
						local,
						smoothstep(transition_progress),
					);
				} else {
					fall.apply_for(rig, local);
				}
			}
			JumpSegment::Land => {
				let land = self.landing_squat(lengths);
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
						"jump land start: elapsed={:.3} cycle_t={:.3} land_local={:.4} land_depth={:.4} land_femur={:.4} transition={:.4} land_desc_d={:.4} y={:.4}",
						elapsed,
						self.time_in_cycle(lengths, elapsed),
						local,
						land.depth(land_progress),
						land.femur_swing(land_progress),
						transition_progress,
						timings.land_descent_duration,
						self.vertical_offset(lengths, elapsed),
					);
				}
				if transition_progress < 1.0 {
					blend_clips(
						rig,
						&Fall::default(),
						1.0,
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
		if y.abs() > f32::EPSILON {
			ArmatureOffset::from_translation(Vec3::new(0.0, y, 0.0))
		} else {
			ArmatureOffset::IDENTITY
		}
	}
}

impl TwoFootedJump {
	pub fn log_landing_debug(&self, rig: &HumanoidV0Rig, elapsed: f32, label: &str) {
		let lengths = rig.segment_lengths;
		let timings = self.timings(lengths);
		let time_in_cycle = self.time_in_cycle(lengths, elapsed);
		let (segment, local) = self.segment(lengths, elapsed);
		let land = self.landing_squat(lengths);
		let land_progress = local / timings.land_duration().max(f32::EPSILON);
		let y = self.vertical_offset(lengths, elapsed);

		info!(
			"{label}: elapsed={:.3} cycle_t={:.3} segment={:?} local={:.4} land_depth={:.4} y={:.4} timings[squat=({:.3},{:.3}) spring={:.3} air={:.3} land=({:.4},{:.3})] speeds[pre={:.3} landing={:.3}]",
			elapsed,
			time_in_cycle,
			segment,
			local,
			land.depth(land_progress),
			y,
			timings.squat_descent_duration,
			timings.squat_ascent_duration,
			timings.spring_duration,
			timings.air_duration,
			timings.land_descent_duration,
			timings.land_ascent_duration,
			self.pre_squat_speed,
			self.landing_squat_speed,
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::animations::DEFAULT_SPRING_DURATION;

	fn default_jump() -> TwoFootedJump {
		TwoFootedJump::default()
	}

	#[test]
	fn spring_end_legs_straight() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let elapsed = jump.timings(lengths).squat_end() + DEFAULT_SPRING_DURATION * 0.99;
		jump.apply(&mut rig, elapsed);

		assert!(rig.posed_angle("femur.L") < 0.05, "femur should be straight");
		assert!(rig.posed_angle("shin.L") < 0.05, "shin should be straight");
		Ok(())
	}

	#[test]
	fn land_starts_compression_after_touchdown() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let timings = jump.timings(lengths);
		jump.apply(&mut rig, timings.air_end() + timings.land_descent_duration * 0.25);

		assert!(rig.posed_angle("femur.L") > 0.01, "land should start folding");
		Ok(())
	}

	#[test]
	fn land_peak_below_full_squat() -> anyhow::Result<()> {
		let mut rig_squat = HumanoidV0Rig::imported();
		Squat::for_loop(1.0, 1.0).apply(&mut rig_squat, 0.5);
		let squat_femur = rig_squat.posed_angle("femur.L");

		let mut rig_land = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig_land.segment_lengths;
		let timings = jump.timings(lengths);
		jump.apply(&mut rig_land, timings.air_end() + timings.land_descent_duration * 0.99);
		let land_femur = rig_land.posed_angle("femur.L");

		assert!(land_femur < squat_femur);
		Ok(())
	}

	#[test]
	fn windup_still_drops_the_armature() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let mid_windup = jump.timings(lengths).squat_descent_duration * 0.99;
		let effects = jump.apply(&mut rig, mid_windup);
		if effects.0.translation.y >= 0.0 {
			return Err(anyhow::anyhow!(
				"windup drop should be negative Y, got {}",
				effects.0.translation.y
			));
		}
		Ok(())
	}

	#[test]
	fn land_transition_blends_arms_from_fall() -> anyhow::Result<()> {
		let mut rig = HumanoidV0Rig::imported();
		let jump = default_jump();
		let lengths = rig.segment_lengths;
		let timings = jump.timings(lengths);
		let blend = timings.land_pose_blend_duration();
		jump.apply(&mut rig, timings.air_end() + blend * 0.5);

		let mut fall_rig = HumanoidV0Rig::imported();
		Fall::default().apply(&mut fall_rig, 1.0);
		assert!(rig.posed_angle("shoulder.L") > 0.05, "blended shoulder leaves rest");
		assert!(
			rig.posed_angle("shoulder.L") < fall_rig.posed_angle("shoulder.L"),
			"blend is short of the full fall spread"
		);
		Ok(())
	}
}
