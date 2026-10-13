//! Topple stature.
//!
//! Soft ~2 ft bird via body-rig asset normalization (~0.30× a ~2 m biped).

use character_rigs::ResolvedRigPose;

/// ~2 ft / ~2 m biped ≈ 0.30 overall scale.
pub const TOPPLE_OVERALL_SCALE: f32 = 0.30;

/// Resolved proportional intent for Topple's humanoid rig (identity layers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TopplePose;

impl TopplePose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
	}
}
