//! Brokker proportion baseline.
//!
//! Full ~2 m stature (no overall asset normalization) with a large wingspan on
//! the bipedal arm bones and a slightly broader shoulder frame.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Brokker's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BrokkerPose;

impl BrokkerPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("brokker species baseline");

		// Large pterosaur wingspan on the bipedal arm bones.
		layer = BraidmanSliders::apply_arm_length(layer, 1.75);
		layer = BraidmanSliders::apply_arm_thickness(layer, 0.9);
		layer = BraidmanSliders::apply_shoulder_width(layer, 1.2);

		// Thin legs for a lean reptilian silhouette.
		layer = BraidmanSliders::apply_leg_thickness(layer, 0.75);
		layer = BraidmanSliders::apply_hip_width(layer, 0.85);

		layer
	}
}
