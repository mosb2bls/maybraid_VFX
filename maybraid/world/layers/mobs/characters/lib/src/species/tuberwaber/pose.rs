//! Tuberwaber proportion layers on the humanoid biped (Braidman-shaped baseline).

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::tuberwaber::{sliders::TuberwaberSliders, TuberwaberConfig},
};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Tuberwaber's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TuberwaberPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: TuberwaberSliders,
}

impl TuberwaberPose {
	pub fn from_config(config: &TuberwaberConfig) -> Self {
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
		RigPoseLayer::new("tuberwaber species baseline")
			.with_scale(BoneScale::uniform("chest.L", 0.85))
			.with_scale(BoneScale::uniform("chest.R", 0.85))
			.with_scale(BoneScale::uniform("lat.L", 0.25))
			.with_scale(BoneScale::uniform("lat.R", 0.25))
			.with_scale(BoneScale::uniform("waist.L", 1.05))
			.with_scale(BoneScale::uniform("waist.R", 1.05))
			.with_scale(BoneScale::length("pelvis.L", 0.85))
			.with_scale(BoneScale::length("pelvis.R", 0.85))
			.with_scale(BoneScale::uniform("lumbar", 0.85))
			.with_scale(BoneScale::uniform("buttocks", 0.9))
			.with_scale(BoneScale::thickness("humerus.L", 0.85))
			.with_scale(BoneScale::thickness("humerus.R", 0.85))
			.with_scale(BoneScale::uniform("thigh_thickness.L", 0.55))
			.with_scale(BoneScale::uniform("thigh_thickness.R", 0.55))
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 0.9);
				layer = TuberwaberSliders::apply_chest_thickness(layer, 1.5);
				layer = TuberwaberSliders::apply_hip_width(layer, 1.2);
				layer = TuberwaberSliders::apply_hip_thickness(layer, 1.1);
				layer = TuberwaberSliders::apply_leg_thickness(layer, 1.2);
				layer = TuberwaberSliders::apply_buttocks_thickness(layer, 1.2);
				layer = TuberwaberSliders::apply_lower_trunk_thickness(layer, 0.9);
				layer = TuberwaberSliders::apply_waist_thickness(layer, 0.7);
				layer = TuberwaberSliders::apply_arm_length(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 0.95);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 0.95);
				layer = TuberwaberSliders::apply_chest_thickness(layer, 0.9);
				layer = TuberwaberSliders::apply_hip_width(layer, 0.9);
			}
			BuildPreset::Athletic => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 1.05);
				layer = TuberwaberSliders::apply_chest_thickness(layer, 1.1);
				layer = TuberwaberSliders::apply_hip_width(layer, 1.1);
			}
			BuildPreset::Heavy => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 1.05);
				layer = TuberwaberSliders::apply_chest_thickness(layer, 1.1);
				layer = TuberwaberSliders::apply_hip_width(layer, 1.1);
			}
			BuildPreset::Stocky => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 1.1);
				layer = TuberwaberSliders::apply_chest_thickness(layer, 1.1);
			}
			BuildPreset::Lanky => {
				layer = TuberwaberSliders::apply_shoulder_width(layer, 0.95);
				layer = TuberwaberSliders::apply_chest_thickness(layer, 0.9);
				layer = TuberwaberSliders::apply_hip_width(layer, 0.9);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
