//! Kaller stature — Kispar-scale with light arm stretch.
//!
//! Overall size is body-rig asset normalization (~0.30×). A light arm-length
//! layer opens the sparrow wingspan slightly (same intent as Kispar).

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{ResolvedRigPose, RigPoseLayer};

/// ~2 ft / ~2 m biped ≈ 0.30 overall scale.
pub const KALLER_OVERALL_SCALE: f32 = 0.30;

/// Resolved proportional intent for Kaller's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KallerPose;

impl KallerPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("kaller species baseline");
		layer = BraidmanSliders::apply_arm_length(layer, 1.15);
		layer
	}
}
