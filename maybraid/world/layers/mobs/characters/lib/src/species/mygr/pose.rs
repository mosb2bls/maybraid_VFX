//! Mygr proportion baseline.
//!
//! Catlike silhouette: ~2/3 human height with thick thighs and buttocks.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Mygr's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MygrPose;

impl MygrPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("mygr species baseline");

		// ~2/3 human stature via shorter legs and a slightly shorter torso.
		layer = BraidmanSliders::apply_leg_length(layer, 0.67);
		layer = layer
			.with_scale(BoneScale::length("lumbar", 0.85))
			.with_scale(BoneScale::length("chest", 0.9));

		// Catlike lower-body emphasis.
		layer = BraidmanSliders::apply_leg_thickness(layer, 1.25);
		layer = BraidmanSliders::apply_buttocks_thickness(layer, 1.3);
		layer = BraidmanSliders::apply_hip_thickness(layer, 1.15);

		layer
	}
}
