//! Croconot proportion layers on the quadruped rig.

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::croconot::{sliders::CroconotSliders, CroconotConfig},
};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Low-slung stature relative to the stock quadruped.
const LIMB_LENGTH: f32 = 0.8;
/// Extra front/hind length the Lanky build stacks on the species baseline.
const LANKY_LIMB_SCALE: f32 = 1.05;

/// Resolved proportional intent for Croconot's low-slung quadruped rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CroconotPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: CroconotSliders,
}

impl CroconotPose {
	pub fn from_config(config: &CroconotConfig) -> Self {
		Self { gender: config.gender, build: config.build, sliders: config.sliders.clamped() }
	}

	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	/// Rest-pose support height relative to the stock quadruped hull (`1.0`).
	pub fn rest_limb_scale(self) -> f32 {
		let sliders = self.sliders.clamped();
		crate::species::common::quadruped_rest_limb_scale(
			LIMB_LENGTH,
			sliders.arm_length,
			sliders.leg_length,
			LANKY_LIMB_SCALE,
			self.build,
		)
	}

	fn species_baseline(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("croconot species baseline")
			.with_scale(BoneScale::uniform("chest_thickness", 0.8))
			.with_scale(BoneScale::uniform("belly", 0.8))
			.with_scale(BoneScale::thickness("", 0.8));
		// Low-slung stature and narrower frame; leg length is applied here once so
		// slider defaults stay at 1.0 and front/hind legs stay matched.
		layer = CroconotSliders::apply_shoulder_width(layer, 0.6);
		layer = CroconotSliders::apply_chest_width(layer, 0.6);
		layer = CroconotSliders::apply_hip_width(layer, 0.9);
		layer = CroconotSliders::apply_arm_length(layer, LIMB_LENGTH);
		CroconotSliders::apply_leg_length(layer, LIMB_LENGTH)
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = CroconotSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = CroconotSliders::apply_shoulder_width(layer, 0.95);
				layer = CroconotSliders::apply_chest_thickness(layer, 1.1);
				layer = CroconotSliders::apply_hip_width(layer, 1.1);
				layer = CroconotSliders::apply_hip_thickness(layer, 1.05);
				layer = CroconotSliders::apply_buttocks_thickness(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = CroconotSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = CroconotSliders::apply_chest_thickness(layer, 0.95);
				layer = CroconotSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = CroconotSliders::apply_chest_thickness(layer, 1.05);
				layer = CroconotSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = CroconotSliders::apply_shoulder_width(layer, 1.05);
				layer = CroconotSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = CroconotSliders::apply_leg_length(layer, LANKY_LIMB_SCALE);
				layer = CroconotSliders::apply_arm_length(layer, LANKY_LIMB_SCALE);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
