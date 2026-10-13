//! Braidman proportion layers.
//!
//! This module documents the first slice of Braidman's species silhouette. It
//! does not overwrite bind-pose transforms; it produces named bone scale layers
//! that the preview applies to the loaded rest pose.

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::braidman::{sliders::BraidmanSliders, BraidmanConfig},
};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Resolved proportional intent for Braidman's humanoid rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BraidmanPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: BraidmanSliders,
}

impl BraidmanPose {
	pub fn from_config(config: &BraidmanConfig) -> Self {
		Self { gender: config.gender, build: config.build, sliders: config.sliders.clamped() }
	}

	pub fn resolve(self) -> ResolvedRigPose {
		// Order matches spec Stage 1: baseline → gender → build → user sliders.
		// Each layer multiplies; later layers never replace earlier bone scales.
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	fn species_baseline(self) -> RigPoseLayer {
		// Authored humanoid rest pose is not Braidman's silhouette; these constants
		// are the species-owned baseline before presets or command sliders run.
		RigPoseLayer::new("braidman species baseline")
			.with_scale(BoneScale::uniform("chest.L", 0.8))
			.with_scale(BoneScale::uniform("chest.R", 0.8))
			.with_scale(BoneScale::uniform("lat.L", 0.2))
			.with_scale(BoneScale::uniform("lat.R", 0.2))
			.with_scale(BoneScale::uniform("waist.L", 1.0))
			.with_scale(BoneScale::uniform("waist.R", 1.0))
			.with_scale(BoneScale::length("pelvis.L", 0.8))
			.with_scale(BoneScale::length("pelvis.R", 0.8))
			.with_scale(BoneScale::uniform("lumbar", 0.8))
			.with_scale(BoneScale::uniform("buttocks", 0.8))
			.with_scale(BoneScale::thickness("humerus.L", 0.8))
			.with_scale(BoneScale::thickness("humerus.R", 0.8))
			.with_scale(BoneScale::uniform("thigh_thickness.L", 0.5))
			.with_scale(BoneScale::uniform("thigh_thickness.R", 0.5))
	}

	fn gender_layer(self) -> RigPoseLayer {
		// Lean shortcut: preset effects are bone scales here. Full pass should
		// apply spec percent offsets to `BraidmanSliders` in `presets` first.
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 0.9);
				layer = BraidmanSliders::apply_chest_thickness(layer, 1.5);
				layer = BraidmanSliders::apply_hip_width(layer, 1.2);
				layer = BraidmanSliders::apply_hip_thickness(layer, 1.1);
				layer = BraidmanSliders::apply_leg_thickness(layer, 1.2);
				layer = BraidmanSliders::apply_buttocks_thickness(layer, 1.2);
				layer = BraidmanSliders::apply_lower_trunk_thickness(layer, 0.9);
				layer = BraidmanSliders::apply_waist_thickness(layer, 0.7);
				// Narrower shoulders shorten reach unless arm bones lengthen too.
				layer = BraidmanSliders::apply_arm_length(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 0.95);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 0.95);
				layer = BraidmanSliders::apply_chest_thickness(layer, 0.9);
				layer = BraidmanSliders::apply_hip_width(layer, 0.9);
			}
			BuildPreset::Athletic => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 1.05);
				layer = BraidmanSliders::apply_chest_thickness(layer, 1.1);
				layer = BraidmanSliders::apply_hip_width(layer, 1.1);
			}
			BuildPreset::Heavy => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 1.05);
				layer = BraidmanSliders::apply_chest_thickness(layer, 1.1);
				layer = BraidmanSliders::apply_hip_width(layer, 1.1);
			}
			BuildPreset::Stocky => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 1.1);
				layer = BraidmanSliders::apply_chest_thickness(layer, 1.1);
			}
			BuildPreset::Lanky => {
				layer = BraidmanSliders::apply_shoulder_width(layer, 0.95);
				layer = BraidmanSliders::apply_chest_thickness(layer, 0.9);
				layer = BraidmanSliders::apply_hip_width(layer, 0.9);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
