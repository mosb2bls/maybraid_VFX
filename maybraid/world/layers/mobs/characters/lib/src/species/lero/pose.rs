//! Lero proportion baseline.
//!
//! Reptilian silhouette: near-human scale with a longer neck and a slightly
//! longer, slimmer lumbar and waist.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Lero's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LeroPose;

impl LeroPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("lero species baseline");

		layer = layer
			.with_scale(BoneScale::length("lumbar", 1.1))
			.with_scale(BoneScale::length("lower_neck", 1.15))
			.with_scale(BoneScale::length("upper_neck", 1.15));

		layer = BraidmanSliders::apply_waist_thickness(layer, 0.9);
		layer = BraidmanSliders::apply_lower_trunk_thickness(layer, 0.9);

		layer
	}
}
