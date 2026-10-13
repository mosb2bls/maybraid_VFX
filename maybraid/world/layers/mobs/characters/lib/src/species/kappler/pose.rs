//! Kappler stature — ~1 m Topple with very short legs.
//!
//! Overall size is body-rig asset normalization (~0.50×). Leg length is crushed
//! via BraidmanSliders; lumbar is slightly shortened so the torso still reads
//! balanced over the stubby stance.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// ~1 m / ~2 m biped ≈ 0.50 overall scale.
pub const KAPPLER_OVERALL_SCALE: f32 = 0.50;

/// Resolved proportional intent for Kappler's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct KapplerPose;

impl KapplerPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("kappler species baseline");
		layer = BraidmanSliders::apply_leg_length(layer, 0.4);
		// Mild lumbar shorten so the trunk doesn't tower over stubby legs.
		layer = layer.with_scale(BoneScale::length("lumbar", 0.9));
		layer
	}
}
