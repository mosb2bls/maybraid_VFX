//! Dui proportion baseline.
//!
//! Tall slender silhouette: ~1.5× Braidman height with long arms.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Dui's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DuiPose;

impl DuiPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("dui species baseline");

		// ~1.5× Braidman stature via longer legs and torso.
		layer = BraidmanSliders::apply_leg_length(layer, 1.5);
		layer = layer
			.with_scale(BoneScale::length("lumbar", 1.2))
			.with_scale(BoneScale::length("chest", 1.15));

		// Long arms.
		layer = BraidmanSliders::apply_arm_length(layer, 1.25);

		// Slender frame.
		layer = BraidmanSliders::apply_shoulder_width(layer, 0.9);
		layer = BraidmanSliders::apply_hip_width(layer, 0.85);
		layer = BraidmanSliders::apply_chest_thickness(layer, 0.9);
		layer = BraidmanSliders::apply_hip_thickness(layer, 0.9);
		layer = BraidmanSliders::apply_leg_thickness(layer, 0.85);
		layer = BraidmanSliders::apply_waist_thickness(layer, 0.8);
		layer = BraidmanSliders::apply_arm_thickness(layer, 0.9);

		layer
	}
}
