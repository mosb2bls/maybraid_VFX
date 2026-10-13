//! Tapp stature — thin ~2 ft Topple sibling.
//!
//! Overall size is body-rig asset normalization (~0.30×). Thin proportions come
//! from BraidmanSliders (narrow shoulders/hips, lean limbs).

use crate::species::braidman::sliders::BraidmanSliders;
use character_rigs::{ResolvedRigPose, RigPoseLayer};

/// ~2 ft / ~2 m biped ≈ 0.30 overall scale.
pub const TAPP_OVERALL_SCALE: f32 = 0.30;

/// Resolved proportional intent for Tapp's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TappPose;

impl TappPose {
	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(self.species_baseline())
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("tapp species baseline");
		layer = BraidmanSliders::apply_shoulder_width(layer, 0.8);
		layer = BraidmanSliders::apply_hip_width(layer, 0.72);
		layer = BraidmanSliders::apply_chest_thickness(layer, 0.78);
		layer = BraidmanSliders::apply_arm_thickness(layer, 0.72);
		layer = BraidmanSliders::apply_leg_thickness(layer, 0.72);
		layer = BraidmanSliders::apply_waist_thickness(layer, 0.7);
		layer = BraidmanSliders::apply_hip_thickness(layer, 0.75);
		layer
	}
}
