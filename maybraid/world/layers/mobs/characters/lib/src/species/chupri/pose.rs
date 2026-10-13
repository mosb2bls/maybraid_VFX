//! Chupri stature.
//!
//! Crane-body bind proportions are left alone. Overall size is **not** a bone
//! scale on `root`: on the humanoid armature, `pelvis.L` / `pelvis.R` /
//! `buttocks` are siblings of `root`, so root-only scale leaves the legs full
//! size. Stature is instead the body-rig [`AssetNormalization`](crate::assets::AssetNormalization)
//! applied to the whole armature scene root (~1 ft from a ~2 m biped).

use character_rigs::ResolvedRigPose;

/// ~1 ft / ~2 m biped ≈ 0.15 overall scale.
pub const CHUPRI_OVERALL_SCALE: f32 = 0.15;

/// Resolved proportional intent for Chupri's humanoid rig (identity layers).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChupriPose;

impl ChupriPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
	}
}
