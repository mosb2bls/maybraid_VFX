//! Shared tuck pose derived from a single [`TuckProfile::tightness`] knob.

use character_rigs::Side;

/// Unit-tightness magnitudes at full tuck (`tightness = 1.0`).
const FEMUR_AT_FULL: f32 = -0.55;
const SHIN_AT_FULL: f32 = 2.2;
/// Small inward shoulder roll on local Y; side signs mirror via [`Side`], not axis flip.
const SHOULDER_ROLL_AT_FULL: f32 = 0.1;
/// Humerus elevation on local Y (swing).
const HUMERUS_SWING_AT_FULL: f32 = -0.0;
/// Humerus medial/lateral on local X (flex).
const HUMERUS_FLEX_AT_FULL: f32 = 1.5;
/// Humerus long-axis spin on local Y (twist); sets forearm orientation.
const HUMERUS_TWIST_AT_FULL: f32 = 0.95;
/// Forearm hinge on Z after humerus articulation.
const FOREARM_AT_FULL: f32 = 2.0;

/// Joint targets for a tucked pose, scaled from one tightness value.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuckProfile {
	tightness: f32,
}

impl TuckProfile {
	pub const DEFAULT_TIGHTNESS: f32 = 1.0;

	pub fn new(tightness: f32) -> Self {
		Self { tightness }
	}

	pub fn tightness(&self) -> f32 {
		self.tightness
	}

	pub fn femur_swing(&self, amount: f32) -> f32 {
		amount * FEMUR_AT_FULL * self.tightness
	}

	pub fn shin_flex(&self, amount: f32) -> f32 {
		amount * SHIN_AT_FULL * self.tightness
	}

	pub fn shoulder_roll(&self, side: Side, amount: f32) -> f32 {
		let sign = match side {
			Side::Left => 1.0,
			Side::Right => -1.0,
		};
		amount * SHOULDER_ROLL_AT_FULL * self.tightness * sign
	}

	pub fn humerus_swing(&self, side: Side, amount: f32) -> f32 {
		let sign = match side {
			Side::Left => -1.0,
			Side::Right => 1.0,
		};
		amount * HUMERUS_SWING_AT_FULL * self.tightness * sign
	}

	pub fn humerus_flex(&self, side: Side, amount: f32) -> f32 {
		let sign = match side {
			Side::Left => 1.0,
			Side::Right => -1.0,
		};
		amount * HUMERUS_FLEX_AT_FULL * self.tightness * sign
	}

	pub fn humerus_twist(&self, side: Side, amount: f32) -> f32 {
		let sign = match side {
			Side::Left => 1.0,
			Side::Right => -1.0,
		};
		amount * HUMERUS_TWIST_AT_FULL * self.tightness * sign
	}

	pub fn forearm_flex(&self, amount: f32) -> f32 {
		amount * FOREARM_AT_FULL * self.tightness
	}
}

#[cfg(test)]
mod tests {
	use std::f32::consts::FRAC_PI_2;

	use character_rigs::Side;

	use super::*;
	use crate::animations::Fall;

	#[test]
	fn default_tightness_matches_legacy_full_tuck_shape() -> anyhow::Result<()> {
		let profile = TuckProfile::new(TuckProfile::DEFAULT_TIGHTNESS);
		assert!((profile.femur_swing(1.0) - FEMUR_AT_FULL).abs() < 1e-5);
		assert!((profile.shin_flex(1.0) - SHIN_AT_FULL).abs() < 1e-5);
		assert!((profile.humerus_flex(Side::Left, 1.0) - HUMERUS_FLEX_AT_FULL).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn profile_scales_linearly_with_tightness() -> anyhow::Result<()> {
		let loose = TuckProfile::new(0.5);
		let tight = TuckProfile::new(1.0);
		assert!((loose.shin_flex(1.0) - tight.shin_flex(1.0) * 0.5).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn tuck_arms_mirror_fall_axes_with_opposite_signs() -> anyhow::Result<()> {
		let profile = TuckProfile::new(TuckProfile::DEFAULT_TIGHTNESS);
		let fall = Fall::default();
		for side in [Side::Left, Side::Right] {
			assert!(
				profile.humerus_swing(side, 1.0).signum() != fall.humerus_swing(side, 0.5).signum()
			);
		}
		assert!(
			profile.shoulder_roll(Side::Left, 1.0).signum()
				!= profile.shoulder_roll(Side::Right, 1.0).signum()
		);
		assert!(
			profile.humerus_swing(Side::Left, 1.0).signum()
				!= profile.humerus_swing(Side::Right, 1.0).signum()
		);
		assert!(
			profile.humerus_twist(Side::Left, 1.0).signum()
				!= profile.humerus_twist(Side::Right, 1.0).signum()
		);
		Ok(())
	}

	#[test]
	fn profile_flexes_knees_beyond_squat_depth() -> anyhow::Result<()> {
		let profile = TuckProfile::new(TuckProfile::DEFAULT_TIGHTNESS);
		assert!(profile.shin_flex(1.0).abs() > FRAC_PI_2);
		assert!(profile.femur_swing(1.0).abs() > 0.2);
		Ok(())
	}
}
