//! Two-footed jump timing and vertical motion.
//!
//! The jump is parameterized by four values:
//!
//! - **`jump_height`** — apex height above the take-off point (world units).
//! - **`gravity`** — constant downward acceleration (units/s²).
//! - **`pre_squat_speed`** — how fast the character squats down before take-off
//!   (stand to bottom in `1/pre_squat_speed` seconds). The return to stand before
//!   spring is matched to launch speed from `jump_height` and `gravity`.
//! - **`landing_squat_speed`** — landing compression and recovery rate (each half-cycle
//!   takes `1/landing_squat_speed` seconds).

use character_rigs::humanoid::LegSegmentLengths;

use crate::animations::{Land, Squat};

pub const DEFAULT_GRAVITY: f32 = 9.8;
pub const DEFAULT_JUMP_HEIGHT: f32 = 1.5;
/// Default pre-jump squat-down rate (bottom in ~0.33 s).
pub const DEFAULT_PRE_SQUAT_SPEED: f32 = 3.0;
/// Default landing recovery rate; also scales impact-matched compression.
pub const DEFAULT_LANDING_SQUAT_SPEED: f32 = 3.0;
pub const DEFAULT_SPRING_DURATION: f32 = 0.15;

/// Fraction of the airborne segment used to blend from spring into fall spread.
pub const FALL_BLEND_FRACTION: f32 = 0.25;
/// Fraction of the **compression** half used to blend fall arms into landing pose.
pub const LAND_BLEND_FRACTION: f32 = 0.25;
/// Upper cap on fall-to-land pose blend so compression is not delayed by recovery timing.
pub const LAND_POSE_BLEND_MAX_SECS: f32 = 0.5;

const MIN_SEGMENT_DURATION: f32 = 1e-3;
const MIN_SPEED: f32 = 1e-3;

/// Constant-gravity parabolic jump from a standing launch height.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BallisticJump {
	pub gravity: f32,
	pub jump_height: f32,
}

impl BallisticJump {
	pub const fn new(gravity: f32, jump_height: f32) -> Self {
		Self { gravity, jump_height }
	}

	/// Impact speed at touch-down for a jump of the given height under constant gravity.
	pub fn launch_speed(self) -> f32 {
		(2.0 * self.gravity * self.jump_height).sqrt()
	}

	/// Ballistic air time from take-off back to launch height.
	pub fn air_duration(self) -> f32 {
		2.0 * self.launch_speed() / self.gravity
	}

	/// Time after take-off when [`ballistic_height`] returns to zero on descent.
	pub fn touchdown_time_since_launch(self) -> f32 {
		self.air_duration()
	}

	/// Height above launch point `time_since_launch` seconds after take-off.
	pub fn ballistic_height(self, time_since_launch: f32) -> f32 {
		if time_since_launch <= 0.0 {
			return 0.0;
		}
		let v0 = self.launch_speed();
		(v0 * time_since_launch - 0.5 * self.gravity * time_since_launch * time_since_launch)
			.max(0.0)
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JumpTiming {
	/// Pre-jump stand-to-bottom duration (seconds).
	pub squat_descent_duration: f32,
	/// Pre-jump bottom-to-stand duration before spring (seconds).
	pub squat_ascent_duration: f32,
	/// Leg extension / push-off window (seconds).
	pub spring_duration: f32,
	/// Fall segment duration: spring end until ballistic touchdown (seconds).
	pub air_duration: f32,
	/// Landing compression duration (seconds).
	pub land_descent_duration: f32,
	/// Stand-up after landing compression (seconds).
	pub land_ascent_duration: f32,
}

impl JumpTiming {
	pub fn squat_duration(&self) -> f32 {
		self.squat_descent_duration + self.squat_ascent_duration
	}

	pub fn land_duration(&self) -> f32 {
		self.land_descent_duration + self.land_ascent_duration
	}

	pub fn cycle_duration(&self) -> f32 {
		self.squat_duration() + self.spring_duration + self.air_duration + self.land_duration()
	}

	pub fn squat_end(&self) -> f32 {
		self.squat_duration()
	}

	pub fn spring_end(&self) -> f32 {
		self.squat_end() + self.spring_duration
	}

	pub fn air_end(&self) -> f32 {
		self.spring_end() + self.air_duration
	}

	/// Seconds spent blending fall spread into landing at touch-down.
	pub fn land_pose_blend_duration(&self) -> f32 {
		(self.land_descent_duration * LAND_BLEND_FRACTION).min(LAND_POSE_BLEND_MAX_SECS)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpSegment {
	Squat,
	Spring,
	Fall,
	Land,
}

#[derive(Debug, Clone)]
pub struct TwoFootedJump {
	/// Downward acceleration for ballistic motion (units/s²).
	pub gravity: f32,
	/// Apex height above the take-off point (world units).
	pub jump_height: f32,
	/// Stand-to-bottom rate before take-off (`1/speed` = descent seconds).
	pub pre_squat_speed: f32,
	/// Landing compression scale and recovery rate after touch-down.
	pub landing_squat_speed: f32,
}

impl Default for TwoFootedJump {
	fn default() -> Self {
		Self {
			gravity: DEFAULT_GRAVITY,
			jump_height: DEFAULT_JUMP_HEIGHT,
			pre_squat_speed: DEFAULT_PRE_SQUAT_SPEED,
			landing_squat_speed: DEFAULT_LANDING_SQUAT_SPEED,
		}
	}
}

impl TwoFootedJump {
	pub fn ballistic(&self) -> BallisticJump {
		BallisticJump::new(self.gravity, self.jump_height)
	}

	pub fn with_gravity(mut self, gravity: f32) -> Self {
		self.gravity = gravity;
		self
	}

	pub fn with_jump_height(mut self, jump_height: f32) -> Self {
		self.jump_height = jump_height;
		self
	}

	pub fn with_pre_squat_speed(mut self, pre_squat_speed: f32) -> Self {
		self.pre_squat_speed = pre_squat_speed;
		self
	}

	pub fn with_landing_squat_speed(mut self, landing_squat_speed: f32) -> Self {
		self.landing_squat_speed = landing_squat_speed;
		self
	}

	fn squat_configs(&self, lengths: LegSegmentLengths) -> (Squat, Land) {
		let impact = self.ballistic().launch_speed();
		let squat_peak = Squat::default().peak_vertical_drop(lengths);

		let windup_descent = self.pre_squat_speed.max(MIN_SPEED);
		let windup_ascent =
			if squat_peak > f32::EPSILON { impact / squat_peak } else { windup_descent };

		let land_half_speed = self.landing_squat_speed.max(MIN_SPEED);

		let windup = Squat::with_speeds(windup_descent, windup_ascent.max(MIN_SPEED));
		let landing = Land::with_speeds(land_half_speed, land_half_speed, Squat::default());
		(windup, landing)
	}

	pub fn timings(&self, lengths: LegSegmentLengths) -> JumpTiming {
		let (windup, landing) = self.squat_configs(lengths);
		let touchdown = self.ballistic().touchdown_time_since_launch();
		// Fall runs from spring end until ballistic height returns to zero. Spring already
		// consumes part of the post-launch clock, so subtract it from the full arc.
		let fall_duration = (touchdown - DEFAULT_SPRING_DURATION).max(MIN_SEGMENT_DURATION);
		JumpTiming {
			squat_descent_duration: windup.descent_duration().max(MIN_SEGMENT_DURATION),
			squat_ascent_duration: windup.ascent_duration().max(MIN_SEGMENT_DURATION),
			spring_duration: DEFAULT_SPRING_DURATION,
			air_duration: fall_duration,
			land_descent_duration: landing.descent_duration().max(MIN_SEGMENT_DURATION),
			land_ascent_duration: landing.ascent_duration().max(MIN_SEGMENT_DURATION),
		}
	}

	/// Seconds after take-off when [`ballistic_height`] returns to zero on descent.
	pub fn touchdown_time_since_launch(&self) -> f32 {
		self.ballistic().touchdown_time_since_launch()
	}

	pub fn cycle_duration(&self, lengths: LegSegmentLengths) -> f32 {
		self.timings(lengths).cycle_duration()
	}

	pub fn time_in_cycle(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let cycle = self.cycle_duration(lengths);
		if cycle <= f32::EPSILON {
			return 0.0;
		}
		elapsed % cycle
	}

	pub fn launch_speed(&self) -> f32 {
		self.ballistic().launch_speed()
	}

	pub fn ballistic_height(&self, time_since_launch: f32) -> f32 {
		self.ballistic().ballistic_height(time_since_launch)
	}

	pub fn segment_at_time(time: f32, timings: &JumpTiming) -> (JumpSegment, f32) {
		let mut t = time;
		if t < timings.squat_duration() {
			return (JumpSegment::Squat, t);
		}
		t -= timings.squat_duration();
		if t < timings.spring_duration {
			return (JumpSegment::Spring, t / timings.spring_duration);
		}
		t -= timings.spring_duration;
		if t < timings.air_duration {
			return (JumpSegment::Fall, t / timings.air_duration.max(f32::EPSILON));
		}
		t -= timings.air_duration;
		(JumpSegment::Land, t)
	}

	pub fn segment(&self, lengths: LegSegmentLengths, elapsed: f32) -> (JumpSegment, f32) {
		Self::segment_at_time(self.time_in_cycle(lengths, elapsed), &self.timings(lengths))
	}

	pub fn time_since_launch(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		(self.time_in_cycle(lengths, elapsed) - self.timings(lengths).squat_end()).max(0.0)
	}

	pub fn prejump_squat(&self, lengths: LegSegmentLengths) -> Squat {
		self.squat_configs(lengths).0
	}

	pub fn landing_squat(&self, lengths: LegSegmentLengths) -> Land {
		self.squat_configs(lengths).1
	}

	pub fn landing_depth(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let land = self.landing_squat(lengths);
		let (segment, local) = self.segment(lengths, elapsed);
		if segment != JumpSegment::Land {
			return 0.0;
		}
		let duration = land.cycle_duration().max(f32::EPSILON);
		land.depth(local / duration)
	}

	pub fn vertical_offset(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let (segment, local) = self.segment(lengths, elapsed);
		let timings = self.timings(lengths);

		match segment {
			JumpSegment::Squat => {
				let squat = self.prejump_squat(lengths);
				let p = local / timings.squat_duration().max(f32::EPSILON);
				-squat.vertical_drop(p, lengths)
			}
			JumpSegment::Spring | JumpSegment::Fall => {
				let since_launch = self.time_since_launch(lengths, elapsed);
				self.ballistic_height(since_launch.min(self.touchdown_time_since_launch()))
			}
			JumpSegment::Land => {
				let land = self.landing_squat(lengths);
				let p = local / timings.land_duration().max(f32::EPSILON);
				-land.vertical_drop(p, lengths)
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn default_jump() -> TwoFootedJump {
		TwoFootedJump::default()
	}

	#[test]
	fn fall_duration_ends_at_ballistic_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let touchdown = jump.touchdown_time_since_launch();
		assert!(
			(timings.spring_duration + timings.air_duration - touchdown).abs() < 1e-3,
			"spring + fall should equal ballistic touchdown"
		);
		Ok(())
	}

	#[test]
	fn vertical_offset_near_zero_at_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let y = jump.vertical_offset(lengths, timings.air_end());
		assert!(y.abs() < 0.05, "expected near-ground at land start, y={y}");
		Ok(())
	}

	#[test]
	fn segment_routing_at_boundaries() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);

		let (seg, _) = TwoFootedJump::segment_at_time(0.0, &timings);
		assert_eq!(seg, JumpSegment::Squat);

		let (seg, _) = TwoFootedJump::segment_at_time(timings.squat_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Spring);

		let (seg, _) = TwoFootedJump::segment_at_time(timings.spring_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Fall);

		let (seg, _) = TwoFootedJump::segment_at_time(timings.air_end() + 1e-4, &timings);
		assert_eq!(seg, JumpSegment::Land);

		Ok(())
	}

	#[test]
	fn windup_ascent_matches_impact_speed() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let impact = BallisticJump::new(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT).launch_speed();
		let squat_peak = Squat::default().peak_vertical_drop(lengths);

		assert!((timings.squat_ascent_duration - squat_peak / impact).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn landing_squat_speed_controls_compression_and_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let slow = TwoFootedJump::default().with_landing_squat_speed(1.0);
		let fast = TwoFootedJump::default().with_landing_squat_speed(4.0);
		let slow_timings = slow.timings(lengths);
		let fast_timings = fast.timings(lengths);
		assert!((slow_timings.land_descent_duration - 1.0).abs() < 1e-3);
		assert!((slow_timings.land_ascent_duration - 1.0).abs() < 1e-3);
		assert!(fast_timings.land_descent_duration < slow_timings.land_descent_duration);
		assert!(fast_timings.land_ascent_duration < slow_timings.land_ascent_duration);
		Ok(())
	}

	#[test]
	fn land_pose_blend_shorter_than_recovery() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let timings = default_jump().timings(lengths);
		assert!(timings.land_pose_blend_duration() < timings.land_ascent_duration);
		assert!(timings.land_pose_blend_duration() <= LAND_POSE_BLEND_MAX_SECS);
		Ok(())
	}

	#[test]
	fn land_starts_at_stand() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let elapsed = jump.timings(lengths).air_end();
		assert!(jump.landing_depth(lengths, elapsed).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn compression_visible_shortly_after_touchdown() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		assert!(jump.landing_depth(lengths, timings.air_end() + 0.05) > 0.1);
		Ok(())
	}

	#[test]
	fn compression_does_not_snap_to_bottom_on_first_frame() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		let timings = jump.timings(lengths);
		let one_frame = 1.0 / 120.0;
		assert!(jump.landing_depth(lengths, timings.air_end() + one_frame) < 0.5);
		Ok(())
	}

	#[test]
	fn pre_squat_speed_controls_windup_descent() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let slow = TwoFootedJump::default().with_pre_squat_speed(0.5);
		let fast = TwoFootedJump::default().with_pre_squat_speed(2.0);
		assert!(
			slow.timings(lengths).squat_descent_duration
				> fast.timings(lengths).squat_descent_duration
		);
		Ok(())
	}

	#[test]
	fn ballistic_reaches_configured_apex() -> anyhow::Result<()> {
		let arc = BallisticJump::new(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT);
		let t_peak = arc.launch_speed() / arc.gravity;
		let apex = arc.ballistic_height(t_peak);
		assert!((apex - DEFAULT_JUMP_HEIGHT).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn vertical_profile_endpoints_and_peak() -> anyhow::Result<()> {
		let lengths = LegSegmentLengths::default();
		let jump = default_jump();
		assert!(jump.vertical_offset(lengths, 0.0).abs() < 1e-5);

		let timings = jump.timings(lengths);
		assert!(jump.vertical_offset(lengths, timings.squat_descent_duration * 0.99) < 0.0);
		assert!(jump.vertical_offset(lengths, timings.squat_end() - 0.001).abs() < 0.08);

		let apex_time = timings.squat_end()
			+ BallisticJump::new(DEFAULT_GRAVITY, DEFAULT_JUMP_HEIGHT).launch_speed()
				/ DEFAULT_GRAVITY;
		assert!((jump.vertical_offset(lengths, apex_time) - DEFAULT_JUMP_HEIGHT).abs() < 0.05);

		Ok(())
	}
}
