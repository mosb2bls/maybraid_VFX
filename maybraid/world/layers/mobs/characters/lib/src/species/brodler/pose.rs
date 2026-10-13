//! Brodler proportion baseline.
//!
//! Cartoon silhouette: ~1.5× height, broad shoulders, long torso, big arms.
//! Applied as bone scale layers on the shared humanoid rig.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Brodler's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrodlerPose;

impl BrodlerPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("brodler species baseline");

		// Overall height ~1.5× via leg and spine length.
		layer = BraidmanSliders::apply_leg_length(layer, 1.5);
		layer = layer
			.with_scale(BoneScale::length("lumbar", 1.5))
			.with_scale(BoneScale::length("chest", 1.25));

		// Broad shoulders and long upper torso.
		layer = BraidmanSliders::apply_shoulder_width(layer, 2.0);
		layer = BraidmanSliders::apply_chest_thickness(layer, 1.25);
		layer = layer
			.with_scale(BoneScale::uniform("lat.L", 1.4))
			.with_scale(BoneScale::uniform("lat.R", 1.4));

		// Big arms.
		layer = BraidmanSliders::apply_arm_length(layer, 0.9);
		layer = BraidmanSliders::apply_arm_thickness(layer, 1.2);

		layer = BraidmanSliders::apply_leg_length(layer, 0.6);

		// Narrower hips relative to shoulders for cartoon upper-body emphasis.
		layer = BraidmanSliders::apply_hip_width(layer, 0.9);
		layer = BraidmanSliders::apply_hip_thickness(layer, 0.85);

		layer
	}
}
