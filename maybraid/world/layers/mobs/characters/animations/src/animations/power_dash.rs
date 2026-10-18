//! Rig-agnostic PowerDash parameters.
//!
//! A forward burst: compress, push into an asymmetric earthbending silhouette,
//! hold the slide, then recover to a compact guard. Progress is owned by the
//! controller; [`PowerDash`] clamps to `[0, 1]`. Humanoid rigs convert to
//! [`UprightPowerDash`](super::UprightPowerDash).
//!
//! Normalized windows match Tanz `MOVE_TIMING` (0.04 + 0.22 + 0.12 = 0.38 s):
//! compress through startup (~0.105), hold through active end (~0.684).

use character_rigs::Side;

/// Quick hip compression occupies the first slice of normalized progress.
pub const COMPRESS_END: f32 = 0.10;
/// Push into the held silhouette; hold begins here.
pub const PUSH_END: f32 = 0.25;
/// Held slide yields to braking. Matches Tanz active end (0.26 / 0.38 ≈ 0.684).
pub const HOLD_END: f32 = 0.68;
/// Braking yields to the rise / guard recover.
pub const BRAKE_END: f32 = 0.85;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PowerDash {
	/// Forward (guard / support) side. Default right: right arm near the face,
	/// left arm on the rear diagonal, right leg supporting, left trailing.
	pub side: Side,
	/// Stance depth and leg drive; 1.0 = tuned default.
	pub drive: f32,
	/// Forward visual slide scale; 1.0 = tuned default.
	pub slide: f32,
}

impl Default for PowerDash {
	fn default() -> Self {
		Self { side: Side::Right, drive: 1.0, slide: 1.0 }
	}
}

impl PowerDash {
	/// Held lunge still used for Tanz icons (~mid-stride in the current clip).
	/// Keep this on the readable hold when the humanoid mapping is restyled.
	pub fn representative_progress() -> f32 {
		0.50
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn phase_windows_match_tanz_move_timing() {
		assert!((COMPRESS_END - 0.10).abs() < 1e-6);
		assert!(PUSH_END > COMPRESS_END);
		assert!((HOLD_END - 0.68).abs() < 1e-6);
		assert!(BRAKE_END > HOLD_END);
		assert!(BRAKE_END < 1.0);
	}

	#[test]
	fn representative_progress_is_the_held_stride() {
		let progress = PowerDash::representative_progress();
		assert!(progress > PUSH_END);
		assert!(progress < HOLD_END);
		assert!((progress - 0.50).abs() < 1e-6);
	}
}
