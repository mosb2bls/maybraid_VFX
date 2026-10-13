//! Spibmom proportion baseline.
//!
//! Wumbus body silhouette with a longer neck for the enlarged meerkat head.

use crate::species::{braidman::sliders::BraidmanSliders, wumbus::pose::WumbusPose};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Spibmom's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpibmomPose;

impl SpibmomPose {
	pub fn resolve(self) -> ResolvedRigPose {
		WumbusPose.resolve().with_layer(self.neck_layer())
	}

	fn neck_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("spibmom neck");

		layer = layer
			.with_scale(BoneScale::length("lower_neck", 1.8))
			.with_scale(BoneScale::length("upper_neck", 1.8));

		layer = BraidmanSliders::apply_arm_length(layer, 1.05);

		// increase waist thickness
		layer = layer.with_scale(BoneScale::uniform("lumbar", 1.2));
		layer = layer.with_scale(BoneScale::length("waist.L", 1.2));
		layer = layer.with_scale(BoneScale::length("waist.R", 1.2));
		layer = layer.with_scale(BoneScale::uniform("belly", 1.2));

		// increase hip width and thickness
		layer = layer.with_scale(BoneScale::length("pelvis.L", 2.0));
		layer = layer.with_scale(BoneScale::length("pelvis.R", 2.0));

		layer
	}
}
