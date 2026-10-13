//! Claber proportion layers on the quadruped rig.

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::claber::{sliders::ClaberSliders, ClaberConfig},
};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Claber's oversized low-slung quadruped rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClaberPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: ClaberSliders,
}

impl ClaberPose {
	pub fn from_config(config: &ClaberConfig) -> Self {
		Self { gender: config.gender, build: config.build, sliders: config.sliders.clamped() }
	}

	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	fn species_baseline(self) -> RigPoseLayer {
		// Overall size through midback length/thickness. `back_ridge` is
		// perpendicular to the spine, so scaling it only raises a dorsal bump.
		// Limbs stay croconot-short (0.8) so the enlarged body sits low.
		let mut layer = RigPoseLayer::new("claber species baseline")
			.with_scale(BoneScale::length("anterior_mid_back", 2.0))
			.with_scale(BoneScale::length("posterior_mid_back", 2.0))
			// Thinned relative to the 2× length so the Gumbus reads leaner.
			.with_scale(BoneScale::thickness("anterior_mid_back", 1.6))
			.with_scale(BoneScale::thickness("posterior_mid_back", 1.6))
			.with_scale(BoneScale::uniform("chest_thickness", 0.8))
			.with_scale(BoneScale::uniform("belly", 0.8));
		layer = ClaberSliders::apply_shoulder_width(layer, 0.6);
		layer = ClaberSliders::apply_chest_width(layer, 0.6);
		layer = ClaberSliders::apply_hip_width(layer, 0.9);
		layer = ClaberSliders::apply_arm_length(layer, 0.8);
		ClaberSliders::apply_leg_length(layer, 0.8)
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = ClaberSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = ClaberSliders::apply_shoulder_width(layer, 0.95);
				layer = ClaberSliders::apply_chest_thickness(layer, 1.1);
				layer = ClaberSliders::apply_hip_width(layer, 1.1);
				layer = ClaberSliders::apply_hip_thickness(layer, 1.05);
				layer = ClaberSliders::apply_buttocks_thickness(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = ClaberSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = ClaberSliders::apply_chest_thickness(layer, 0.95);
				layer = ClaberSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = ClaberSliders::apply_chest_thickness(layer, 1.05);
				layer = ClaberSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = ClaberSliders::apply_shoulder_width(layer, 1.05);
				layer = ClaberSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = ClaberSliders::apply_leg_length(layer, 1.05);
				layer = ClaberSliders::apply_arm_length(layer, 1.05);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
