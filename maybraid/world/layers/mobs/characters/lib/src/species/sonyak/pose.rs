//! Sonyak proportion layers on the gumbus quadruped (no neck pitch).

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::sonyak::{sliders::SonyakSliders, SonyakConfig},
};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

const LIMB_LENGTH: f32 = 1.0;
/// Extra front/hind length the Lanky build stacks on the species baseline.
const LANKY_LIMB_SCALE: f32 = 1.1;
const TORSO_THICKNESS: f32 = 1.15;
const LEG_THICKNESS: f32 = 1.1;

/// Resolved proportional intent for Sonyak's gumbus quadruped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SonyakPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: SonyakSliders,
}

impl SonyakPose {
	pub fn from_config(config: &SonyakConfig) -> Self {
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
		let mut layer = RigPoseLayer::new("sonyak species baseline")
			.with_scale(BoneScale::thickness("anterior_mid_back", TORSO_THICKNESS))
			.with_scale(BoneScale::thickness("posterior_mid_back", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("chest_thickness", 1.0))
			.with_scale(BoneScale::uniform("belly", 1.0));
		layer = SonyakSliders::apply_leg_thickness(layer, LEG_THICKNESS);
		layer = SonyakSliders::apply_arm_length(layer, LIMB_LENGTH);
		SonyakSliders::apply_leg_length(layer, LIMB_LENGTH)
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = SonyakSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = SonyakSliders::apply_shoulder_width(layer, 0.95);
				layer = SonyakSliders::apply_chest_thickness(layer, 1.05);
				layer = SonyakSliders::apply_hip_width(layer, 1.05);
			}
			GenderPreset::NonBinary => {
				layer = SonyakSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = SonyakSliders::apply_chest_thickness(layer, 0.95);
				layer = SonyakSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = SonyakSliders::apply_chest_thickness(layer, 1.05);
				layer = SonyakSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = SonyakSliders::apply_shoulder_width(layer, 1.05);
				layer = SonyakSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = SonyakSliders::apply_leg_length(layer, LANKY_LIMB_SCALE);
				layer = SonyakSliders::apply_arm_length(layer, LANKY_LIMB_SCALE);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
