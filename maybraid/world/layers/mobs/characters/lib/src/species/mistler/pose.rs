//! Mistler stature — ~1 ft from a ~2 m authored forelimbed body.

use character_rigs::ResolvedRigPose;

/// ~0.305 m / 2 m authored length ≈ 0.15.
pub const MISTLER_OVERALL_SCALE: f32 = 0.15;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MistlerPose;

impl MistlerPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
	}
}
