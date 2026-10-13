//! Thumplus stature — ~6 m from a ~2 m authored forelimbed body.

use character_rigs::ResolvedRigPose;

/// 6 m / 2 m authored length.
pub const THUMPLUS_OVERALL_SCALE: f32 = 3.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ThumplusPose;

impl ThumplusPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
	}
}
