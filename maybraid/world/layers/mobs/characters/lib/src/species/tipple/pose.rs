//! Tipple stature.
//!
//! Whelp-body bind proportions are left alone. Overall size is body-rig
//! [`AssetNormalization`](crate::assets::AssetNormalization) (~1 ft from a ~2 m biped).

use character_rigs::ResolvedRigPose;

/// ~1 ft / ~2 m biped ≈ 0.15 overall scale.
pub const TIPPLE_OVERALL_SCALE: f32 = 0.15;

/// Resolved proportional intent for Tipple's humanoid rig (identity layers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TipplePose;

impl TipplePose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
	}
}
