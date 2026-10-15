//! Rig-agnostic leap (running jump) parameters.
//!
//! Progress is owned by the controller; [`Leap`] is a one-shot sampler that
//! clamps to `[0, 1]`. Humanoid rigs convert to [`UprightLeap`](super::UprightLeap);
//! quadruped rigs convert to [`QuadrupedLeap`](super::QuadrupedLeap).
//!
//! Unlike [`TwoFootedJump`](super::TwoFootedJump), a leap keeps a run split at
//! takeoff, gathers in the air, and lands ready to run. It does not emit
//! root-motion Y — physics owns the capsule lift.

/// Takeoff occupies the first slice of normalized progress.
pub const TAKEOFF_END: f32 = 0.18;
/// Air occupies the middle slice; land is the remainder.
pub const AIR_END: f32 = 0.72;

/// Takeoff, air, or land slice of a [`Leap`] shot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeapPhase {
	Takeoff,
	Air,
	Land,
}

/// Maps normalized leap progress to a phase and smoothstepped local weight.
pub struct LeapTimeline;

impl LeapTimeline {
	pub fn sample(t: f32) -> (LeapPhase, f32) {
		use super::mix::smoothstep;
		if t < TAKEOFF_END {
			(LeapPhase::Takeoff, smoothstep(t / TAKEOFF_END))
		} else if t < AIR_END {
			(LeapPhase::Air, smoothstep((t - TAKEOFF_END) / (AIR_END - TAKEOFF_END)))
		} else {
			(LeapPhase::Land, smoothstep((t - AIR_END) / (1.0 - AIR_END).max(f32::EPSILON)))
		}
	}
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Leap {
	/// Air tuck amount; 1.0 = tuned default.
	pub gather: f32,
	/// Forward torso lean; 1.0 = tuned default.
	pub lean: f32,
	/// Takeoff lead/trail split; 1.0 = tuned default.
	pub stride: f32,
}

impl Default for Leap {
	fn default() -> Self {
		Self { gather: 1.0, lean: 1.0, stride: 1.0 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_knobs_are_unity() {
		let leap = Leap::default();
		assert_eq!(leap.gather, 1.0);
		assert_eq!(leap.lean, 1.0);
		assert_eq!(leap.stride, 1.0);
	}

	#[test]
	fn phase_windows_cover_the_shot() {
		assert!(TAKEOFF_END > 0.0);
		assert!(AIR_END > TAKEOFF_END);
		assert!(AIR_END < 1.0);
	}

	#[test]
	fn timeline_phase_boundaries() {
		assert_eq!(LeapTimeline::sample(0.0).0, LeapPhase::Takeoff);
		assert_eq!(LeapTimeline::sample(TAKEOFF_END - 1e-4).0, LeapPhase::Takeoff);
		assert_eq!(LeapTimeline::sample(TAKEOFF_END).0, LeapPhase::Air);
		assert_eq!(LeapTimeline::sample(AIR_END - 1e-4).0, LeapPhase::Air);
		assert_eq!(LeapTimeline::sample(AIR_END).0, LeapPhase::Land);
		assert_eq!(LeapTimeline::sample(1.0).0, LeapPhase::Land);
	}
}
