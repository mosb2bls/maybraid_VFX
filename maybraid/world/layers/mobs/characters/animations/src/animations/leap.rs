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
}
