use std::f32::consts::TAU;

use crate::animations::FixedTuck;
use crate::Progress;

/// Forward pitch is +X; backward is −X.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FlipDirection {
	#[default]
	Forward,
	Backward,
}

#[derive(Debug, Clone)]
pub struct TuckedFlip {
	/// Rotations over the animation (`1.0` = one full turn).
	pub turns: f32,
	pub direction: FlipDirection,
	pub tuck: FixedTuck,
}

impl Default for TuckedFlip {
	fn default() -> Self {
		Self { turns: 1.0, direction: FlipDirection::Forward, tuck: FixedTuck::default() }
	}
}

impl TuckedFlip {
	/// Normalized flip progress in `[0.0, 1.0]`.
	pub fn amount(&self, progress: f32) -> f32 {
		Progress(progress).clamp()
	}

	/// Pitch in radians about the character +X axis at `progress`.
	pub fn pitch_radians(&self, progress: f32) -> f32 {
		let sign = match self.direction {
			FlipDirection::Forward => 1.0,
			FlipDirection::Backward => -1.0,
		};
		sign * self.turns * TAU * self.amount(progress)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tucked_flip_starts_upright() -> anyhow::Result<()> {
		let flip = TuckedFlip::default();
		assert!(flip.pitch_radians(0.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn forward_tucked_flip_completes_one_turn() -> anyhow::Result<()> {
		let flip = TuckedFlip::default();
		assert!((flip.pitch_radians(1.0) - TAU).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn backward_tucked_flip_negates_pitch() -> anyhow::Result<()> {
		let forward = TuckedFlip::default();
		let backward = TuckedFlip { direction: FlipDirection::Backward, ..Default::default() };
		assert!((backward.pitch_radians(0.5) + forward.pitch_radians(0.5)).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn tucked_flip_scales_with_turns() -> anyhow::Result<()> {
		let flip = TuckedFlip::default();
		let double = TuckedFlip { turns: 2.0, ..Default::default() };
		assert!((double.pitch_radians(0.5) - flip.pitch_radians(0.5) * 2.0).abs() < 1e-4);
		Ok(())
	}
}
