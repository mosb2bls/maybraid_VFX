//! Kispar stature / subtle kite stretch.
//!
//! Overall size is body-rig asset normalization (~2 ft). A light arm-length
//! layer slightly opens the authored sparrow wingspan without Brokker-scale
//! bone exaggeration.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{ResolvedRigPose, RigPoseLayer};

/// ~2 ft / ~2 m biped ≈ 0.30 overall scale.
pub const KISPAR_OVERALL_SCALE: f32 = 0.30;

/// Resolved proportional intent for Kispar's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KisparPose;

impl KisparPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("kispar species baseline");
		layer = BraidmanSliders::apply_arm_length(layer, 1.15);
		layer
	}
}
