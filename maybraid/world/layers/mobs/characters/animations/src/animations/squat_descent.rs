//! One-shot stand-to-held-squat descent.
//!
//! Connects upright locomotion ([`Still`](super::Idle), walk, run) to the held
//! [`Squat`](super::Squat) stance. Progress `0` is upright; `1` matches
//! [`Squat::held`] at full depth. Sampling is stateless: progress alone sets depth.

use crate::animations::{smoothstep, Squat};
use crate::Progress;

/// Default descent rate: full transition in ~0.35 s at speed `1.0`.
pub const DEFAULT_DESCENT_SPEED: f32 = 2.85;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SquatDescent;

impl SquatDescent {
	pub fn held() -> Self {
		Self
	}

	/// Eased squat depth: 0 at upright, 1 at held bottom.
	pub fn depth(&self, progress: f32) -> f32 {
		smoothstep(Progress(progress).clamp())
	}

	fn held_squat(&self) -> Squat {
		Squat::held()
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().femur_peak * depth
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().shin_peak * depth
	}

	pub fn root_swing(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().root_peak * depth
	}

	pub fn hip_fold(&self, progress: f32) -> f32 {
		let depth = self.depth(progress);
		self.held_squat().hip_peak * depth
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn depth_endpoints_match_held_squat() -> anyhow::Result<()> {
		let descent = SquatDescent::default();
		assert!(descent.depth(0.0).abs() < 1e-5);
		assert!((descent.depth(1.0) - 1.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn depth_eases_in_the_middle() -> anyhow::Result<()> {
		let descent = SquatDescent::default();
		let mid = descent.depth(0.5);
		assert!(mid > 0.35 && mid < 0.65, "smoothstep midpoint, got {mid}");
		Ok(())
	}

	#[test]
	fn depth_clamps_past_one() -> anyhow::Result<()> {
		let descent = SquatDescent::default();
		assert!((descent.depth(1.5) - 1.0).abs() < 1e-5);
		Ok(())
	}
}
