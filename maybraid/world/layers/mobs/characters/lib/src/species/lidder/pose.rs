//! Lidder proportion baseline.
//!
//! Compact bird silhouette: ~½ Braidman height with a modest wingspan on the
//! bipedal arm bones (crane mesh authored as wings on those chains).

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Lidder's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LidderPose;

impl LidderPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("lidder species baseline");

		// ~½ Braidman stature via shorter legs and a compact torso.
		layer = BraidmanSliders::apply_leg_length(layer, 0.5);
		layer = layer
			.with_scale(BoneScale::length("lumbar", 0.75))
			.with_scale(BoneScale::length("chest", 0.8));

		// Modest wingspan on the bipedal arm bones (kept close to human reach).
		layer = BraidmanSliders::apply_arm_length(layer, 1.1);
		layer = BraidmanSliders::apply_arm_thickness(layer, 0.85);

		// Narrow avian frame.
		layer = BraidmanSliders::apply_shoulder_width(layer, 1.05);
		layer = BraidmanSliders::apply_hip_width(layer, 0.75);
		layer = BraidmanSliders::apply_chest_thickness(layer, 0.85);
		layer = BraidmanSliders::apply_hip_thickness(layer, 0.8);
		layer = BraidmanSliders::apply_leg_thickness(layer, 0.8);
		layer = BraidmanSliders::apply_waist_thickness(layer, 0.75);

		layer
	}
}
