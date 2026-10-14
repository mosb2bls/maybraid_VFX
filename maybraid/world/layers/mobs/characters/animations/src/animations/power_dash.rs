//! Rig-agnostic PowerDash parameters.
//!
//! A forward burst in three phases: chambered wind-up, stomping strike, low dash.
//! Progress is owned by the controller; [`PowerDash`] clamps to `[0, 1]`.
//! Humanoid rigs convert to [`UprightPowerDash`](super::UprightPowerDash).

use character_rigs::Side;

/// Opening wind-up occupies the first slice of normalized progress.
pub const JAB_PHASE_END: f32 = 0.26;
/// Stomp / lunge / strike occupies the middle slice; dash-and-settle is the remainder.
pub const STRIDE_PHASE_END: f32 = 0.58;
/// Jab sampler progress at the hand-off into the stomp (peak chamber, arm drawn back).
pub const JAB_PROGRESS_AT_HANDOFF: f32 = 0.18;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerDash {
	/// Punching side for the opening chamber and the earthbending strike.
	pub side: Side,
	/// Lunge depth and leg drive; 1.0 = tuned default.
	pub drive: f32,
	/// Forward visual slide scale; 1.0 = tuned default.
	pub slide: f32,
}

impl Default for PowerDash {
	fn default() -> Self {
		Self { side: Side::Right, drive: 1.0, slide: 1.0 }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn phase_windows_cover_the_shot() {
		assert!(JAB_PHASE_END > 0.0);
		assert!(STRIDE_PHASE_END > JAB_PHASE_END);
		assert!(STRIDE_PHASE_END < 1.0);
	}

	#[test]
	fn handoff_stays_in_the_jab_chamber() {
		assert!(JAB_PROGRESS_AT_HANDOFF < 0.22);
		assert!(JAB_PROGRESS_AT_HANDOFF > 0.10);
	}
}
