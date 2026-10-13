//! Wumbus proportion baseline.
//!
//! Bearlike silhouette: ~1.2× human height with thicker front-to-back depth.

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Wumbus's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WumbusPose;

impl WumbusPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("wumbus species baseline");

		layer = BraidmanSliders::apply_leg_length(layer, 1.2);
		layer = layer
			.with_scale(BoneScale::length("lumbar", 1.1))
			.with_scale(BoneScale::length("chest", 1.08));

		const TRUNK_DEPTH: f32 = 1.58;
		const LEG_DEPTH: f32 = 1.72;
		const ARM_DEPTH: f32 = 1.05;
		layer = BraidmanSliders::apply_chest_thickness(layer, TRUNK_DEPTH);
		layer = BraidmanSliders::apply_hip_thickness(layer, TRUNK_DEPTH);
		layer = BraidmanSliders::apply_leg_thickness(layer, LEG_DEPTH);
		layer = BraidmanSliders::apply_buttocks_thickness(layer, TRUNK_DEPTH);
		layer = BraidmanSliders::apply_waist_thickness(layer, TRUNK_DEPTH);
		layer = BraidmanSliders::apply_lower_trunk_thickness(layer, TRUNK_DEPTH);
		layer = BraidmanSliders::apply_arm_thickness(layer, ARM_DEPTH);

		layer
	}
}
