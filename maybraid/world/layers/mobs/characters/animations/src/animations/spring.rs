use crate::animations::Squat;
use crate::Progress;

const SHOULDER_SWING_BACK: f32 = -0.55;
const HUMERUS_FLEX_BACK: f32 = 0.65;
const FOREARM_EXTEND: f32 = -0.35;

#[derive(Debug, Clone)]
pub struct Spring {
	pub squat: Squat,
}

impl Spring {
	/// Ease-out 0 at crouch, 1 at full extension.
	pub fn extend_amount(&self, progress: f32) -> f32 {
		let t = Progress(progress).clamp();
		1.0 - (1.0 - t).powi(2)
	}

	pub fn femur_swing(&self, progress: f32) -> f32 {
		self.squat.femur_peak * (1.0 - self.extend_amount(progress))
	}

	pub fn shin_flex(&self, progress: f32) -> f32 {
		self.squat.shin_peak * (1.0 - self.extend_amount(progress))
	}

	pub fn root_swing(&self, progress: f32) -> f32 {
		self.squat.root_peak * (1.0 - self.extend_amount(progress))
	}

	pub fn arm_amount(&self, progress: f32) -> f32 {
		self.extend_amount(progress)
	}

	pub fn shoulder_swing(&self, progress: f32) -> f32 {
		self.arm_amount(progress) * SHOULDER_SWING_BACK
	}

	pub fn humerus_flex(&self, progress: f32) -> f32 {
		self.arm_amount(progress) * HUMERUS_FLEX_BACK
	}

	pub fn forearm_flex(&self, progress: f32) -> f32 {
		self.arm_amount(progress) * FOREARM_EXTEND
	}

	/// Squat-depth vertical drop at spring start (legs fully bent).
	pub fn start_drop(&self, lengths: character_rigs::humanoid::LegSegmentLengths) -> f32 {
		self.squat.vertical_drop(0.0, lengths)
	}
}

impl Default for Spring {
	fn default() -> Self {
		Self { squat: Squat::default() }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn spring_end_straens_legs() -> anyhow::Result<()> {
		let spring = Spring::default();
		assert!(spring.femur_swing(0.99).abs() < 1e-2);
		assert!(spring.shin_flex(0.99).abs() < 1e-2);
		assert!(spring.root_swing(0.99).abs() < 1e-2);
		Ok(())
	}

	#[test]
	fn spring_start_matches_full_squat_angles() -> anyhow::Result<()> {
		let squat = Squat::default();
		let spring = Spring::default();
		assert!((spring.femur_swing(0.0) - squat.femur_peak).abs() < 1e-5);
		assert!((spring.shin_flex(0.0) - squat.shin_peak).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn spring_arms_reach_back_at_full_extension() -> anyhow::Result<()> {
		let spring = Spring::default();
		assert!(spring.shoulder_swing(0.99) < -0.3);
		assert!(spring.humerus_flex(0.99) > 0.0);
		Ok(())
	}
}
