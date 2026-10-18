//! Two-footed jump with a tucked forward flip during the airborne segment.

use character_rigs::humanoid::LegSegmentLengths;

use crate::animations::{JumpSegment, JumpTiming, TuckedFlip, TwoFootedJump};

#[derive(Debug, Clone)]
pub struct TwoFootedTuckedFlip {
	pub jump: TwoFootedJump,
	pub flip: TuckedFlip,
}

impl Default for TwoFootedTuckedFlip {
	fn default() -> Self {
		Self { jump: TwoFootedJump::default(), flip: TuckedFlip::default() }
	}
}

impl TwoFootedTuckedFlip {
	pub fn with_jump(mut self, jump: TwoFootedJump) -> Self {
		self.jump = jump;
		self
	}

	pub fn with_flip(mut self, flip: TuckedFlip) -> Self {
		self.flip = flip;
		self
	}

	pub fn timings(&self, lengths: LegSegmentLengths) -> JumpTiming {
		self.jump.timings(lengths)
	}

	pub fn cycle_duration(&self, lengths: LegSegmentLengths) -> f32 {
		self.jump.cycle_duration(lengths)
	}

	pub fn segment(&self, lengths: LegSegmentLengths, elapsed: f32) -> (JumpSegment, f32) {
		self.jump.segment(lengths, elapsed)
	}

	pub fn vertical_offset(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		self.jump.vertical_offset(lengths, elapsed)
	}

	/// Flip pitch during the airborne segment; zero on ground phases.
	pub fn flip_pitch_radians(&self, lengths: LegSegmentLengths, elapsed: f32) -> f32 {
		let (segment, local) = self.segment(lengths, elapsed);
		if segment == JumpSegment::Fall {
			self.flip.pitch_radians(local)
		} else {
			0.0
		}
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::TAU;

	use character_rigs::humanoid::LegSegmentLengths;

	use super::*;
	use crate::animations::FlipDirection;

	#[test]
	fn flip_pitch_zero_on_ground() -> anyhow::Result<()> {
		let flip = TwoFootedTuckedFlip::default();
		let lengths = LegSegmentLengths::default();
		assert!(flip.flip_pitch_radians(lengths, 0.0).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn flip_pitch_completes_turn_mid_air() -> anyhow::Result<()> {
		let flip = TwoFootedTuckedFlip::default();
		let lengths = LegSegmentLengths::default();
		let timings = flip.timings(lengths);
		let mid_air = timings.spring_end() + timings.air_duration * 0.5;
		let pitch = flip.flip_pitch_radians(lengths, mid_air);
		assert!((pitch - TAU * 0.5).abs() < 1e-3);
		Ok(())
	}

	#[test]
	fn backward_flip_negates_airborne_pitch() -> anyhow::Result<()> {
		let forward = TwoFootedTuckedFlip::default();
		let mut backward = TwoFootedTuckedFlip::default();
		backward.flip.direction = FlipDirection::Backward;
		let lengths = LegSegmentLengths::default();
		let timings = forward.timings(lengths);
		let mid_air = timings.spring_end() + timings.air_duration * 0.5;
		let fwd = forward.flip_pitch_radians(lengths, mid_air);
		let bwd = backward.flip_pitch_radians(lengths, mid_air);
		assert!((fwd + bwd).abs() < 1e-3);
		Ok(())
	}
}
