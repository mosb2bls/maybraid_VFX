//! Caole proportion layers on the quadruped rig.

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::caole::{assets::CaoleBodyMesh, sliders::CaoleSliders, CaoleConfig},
};
use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

/// Extra front/hind length the Lanky build stacks on the species baseline.
const LANKY_LIMB_SCALE: f32 = 1.05;

/// Gumbus: shorter back ridge and slimmer legs than the stock quadruped baseline.
const GUMBUS_BACK_RIDGE_LENGTH: f32 = 0.84;
const GUMBUS_LEG_THICKNESS: f32 = 0.75;

/// Rumbler: longer back ridge and thicker legs than the stock quadruped baseline.
const RUMBLER_BACK_RIDGE_LENGTH: f32 = 1.6;
const TORSO_THICKNESS: f32 = 1.6;
const RUMBLER_LEG_THICKNESS: f32 = 1.3;
const RUMBLER_BELLEY_LENGTH: f32 = 2.2;

/// Resolved proportional intent for Caole's quadruped rig.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CaolePose {
	pub body: CaoleBodyMesh,
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: CaoleSliders,
}

impl CaolePose {
	pub fn from_config(config: &CaoleConfig) -> Self {
		Self {
			body: config.body,
			gender: config.gender,
			build: config.build,
			sliders: config.sliders.clamped(),
		}
	}

	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(self.body_mesh_layer())
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	/// Rest-pose support height relative to the stock quadruped hull (`1.0`).
	pub fn rest_limb_scale(self) -> f32 {
		let sliders = self.sliders.clamped();
		crate::species::common::quadruped_rest_limb_scale(
			1.0,
			sliders.arm_length,
			sliders.leg_length,
			LANKY_LIMB_SCALE,
			self.build,
		)
	}

	fn species_baseline(self) -> RigPoseLayer {
		RigPoseLayer::new("caole species baseline")
			.with_scale(BoneScale::uniform("chest_thickness", 1.0))
			.with_scale(BoneScale::uniform("belly", 1.0))
	}

	fn body_mesh_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("body mesh baseline");
		match self.body {
			CaoleBodyMesh::Gumbus => {
				layer = layer.with_scale(BoneScale::length("back_ridge", GUMBUS_BACK_RIDGE_LENGTH));
				layer = CaoleSliders::apply_leg_thickness(layer, GUMBUS_LEG_THICKNESS);
			}
			CaoleBodyMesh::Rumbler => {
				layer =
					layer.with_scale(BoneScale::length("back_ridge", RUMBLER_BACK_RIDGE_LENGTH));
				layer = layer.with_scale(BoneScale::thickness("anterior_midback", TORSO_THICKNESS));
				layer =
					layer.with_scale(BoneScale::thickness("posterior_midback", TORSO_THICKNESS));
				layer = layer.with_scale(BoneScale::uniform("waist.L", TORSO_THICKNESS));
				layer = layer.with_scale(BoneScale::uniform("waist.R", TORSO_THICKNESS));
				layer = layer.with_scale(BoneScale::uniform("lower_chest.L", TORSO_THICKNESS));
				layer = layer.with_scale(BoneScale::uniform("lower_chest.R", TORSO_THICKNESS));
				layer = CaoleSliders::apply_leg_thickness(layer, RUMBLER_LEG_THICKNESS);
				layer = layer.with_scale(BoneScale::length("belly", RUMBLER_BELLEY_LENGTH));
			}
		}
		layer
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = CaoleSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = CaoleSliders::apply_shoulder_width(layer, 0.95);
				layer = CaoleSliders::apply_chest_thickness(layer, 1.1);
				layer = CaoleSliders::apply_hip_width(layer, 1.1);
				layer = CaoleSliders::apply_hip_thickness(layer, 1.05);
				layer = CaoleSliders::apply_buttocks_thickness(layer, 1.1);
			}
			GenderPreset::NonBinary => {
				layer = CaoleSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = CaoleSliders::apply_chest_thickness(layer, 0.95);
				layer = CaoleSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = CaoleSliders::apply_chest_thickness(layer, 1.05);
				layer = CaoleSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = CaoleSliders::apply_shoulder_width(layer, 1.05);
				layer = CaoleSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = CaoleSliders::apply_leg_length(layer, LANKY_LIMB_SCALE);
				layer = CaoleSliders::apply_arm_length(layer, LANKY_LIMB_SCALE);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
