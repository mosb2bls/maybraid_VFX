//! Grener stature — ~3 m from a ~2 m authored forelimbed body.

use character_rigs::ResolvedRigPose;

/// 3 m / 2 m authored length.
pub const GRENER_OVERALL_SCALE: f32 = 1.5;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GrenerPose;

impl GrenerPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
	}
}
