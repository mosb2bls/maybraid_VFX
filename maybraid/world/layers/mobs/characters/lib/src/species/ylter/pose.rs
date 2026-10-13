//! Yilter proportion layers: Hars neck pitch with longer limbs.

use std::f32::consts::FRAC_PI_4;

use crate::{
	presets::{BuildPreset, GenderPreset},
	species::ylter::{sliders::YilterSliders, YilterConfig},
};
use character_rigs::{BoneRotation, BoneScale, ResolvedRigPose, RigPoseLayer};

/// Raise the body `head_socket` (neck follows).
pub const NECK_PITCH: f32 = FRAC_PI_4;
/// Counter-pitch on the neck tip `head_socket` so the head stays level.
pub const HEAD_SOCKET_PITCH: f32 = -NECK_PITCH;

/// Limb length (~0.8× the prior 1.75 lanky baseline).
const LIMB_LENGTH: f32 = 1.4;
/// Extra front/hind length the Lanky build stacks on the species baseline.
const LANKY_LIMB_SCALE: f32 = 1.1;
const TORSO_THICKNESS: f32 = 1.2;
const RUMBLER_BACK_RIDGE_LENGTH: f32 = 1.35;
const RUMBLER_LEG_THICKNESS: f32 = 1.05;
const RUMBLER_BELLY_LENGTH: f32 = 1.85;

/// Resolved proportional intent for Yilter's long-neck quadruped.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct YilterPose {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub sliders: YilterSliders,
}

impl YilterPose {
	pub fn from_config(config: &YilterConfig) -> Self {
		Self { gender: config.gender, build: config.build, sliders: config.sliders.clamped() }
	}

	pub fn resolve(self) -> ResolvedRigPose {
		ResolvedRigPose::new()
			.with_layer(self.species_baseline())
			.with_layer(
				RigPoseLayer::new("ylter body head socket pitch")
					.with_rotation(BoneRotation::pitch_x("head_socket", NECK_PITCH)),
			)
			.with_layer(self.gender_layer())
			.with_layer(self.build_layer())
			.with_layer(self.slider_layer())
	}

	pub fn neck_pose(self) -> ResolvedRigPose {
		ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("ylter neck tip counterpitch")
				.with_rotation(BoneRotation::pitch_x("head_socket", HEAD_SOCKET_PITCH)),
		)
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
		let mut layer = RigPoseLayer::new("ylter species baseline")
			.with_scale(BoneScale::length("back_ridge", RUMBLER_BACK_RIDGE_LENGTH))
			.with_scale(BoneScale::thickness("anterior_mid_back", TORSO_THICKNESS))
			.with_scale(BoneScale::thickness("posterior_mid_back", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("waist.L", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("waist.R", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("lower_chest_width.L", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("lower_chest_width.R", TORSO_THICKNESS))
			.with_scale(BoneScale::uniform("chest_thickness", 1.0))
			.with_scale(BoneScale::length("belly", RUMBLER_BELLY_LENGTH))
			.with_scale(BoneScale::uniform("belly", 1.0));
		layer = YilterSliders::apply_leg_thickness(layer, RUMBLER_LEG_THICKNESS);
		layer = YilterSliders::apply_arm_length(layer, LIMB_LENGTH);
		YilterSliders::apply_leg_length(layer, LIMB_LENGTH)
	}

	fn gender_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("gender preset");
		match self.gender {
			GenderPreset::Male => {
				layer = YilterSliders::apply_shoulder_width(layer, 1.05);
			}
			GenderPreset::Female => {
				layer = YilterSliders::apply_shoulder_width(layer, 0.95);
				layer = YilterSliders::apply_chest_thickness(layer, 1.05);
				layer = YilterSliders::apply_hip_width(layer, 1.05);
			}
			GenderPreset::NonBinary => {
				layer = YilterSliders::apply_shoulder_width(layer, 0.98);
			}
			GenderPreset::Neutral => {}
		}
		layer
	}

	fn build_layer(self) -> RigPoseLayer {
		let mut layer = RigPoseLayer::new("build preset");
		match self.build {
			BuildPreset::Slender => {
				layer = YilterSliders::apply_chest_thickness(layer, 0.95);
				layer = YilterSliders::apply_hip_width(layer, 0.95);
			}
			BuildPreset::Athletic | BuildPreset::Heavy => {
				layer = YilterSliders::apply_chest_thickness(layer, 1.05);
				layer = YilterSliders::apply_hip_width(layer, 1.05);
			}
			BuildPreset::Stocky => {
				layer = YilterSliders::apply_shoulder_width(layer, 1.05);
				layer = YilterSliders::apply_chest_thickness(layer, 1.05);
			}
			BuildPreset::Lanky => {
				layer = YilterSliders::apply_leg_length(layer, LANKY_LIMB_SCALE);
				layer = YilterSliders::apply_arm_length(layer, LANKY_LIMB_SCALE);
			}
			BuildPreset::Average => {}
		}
		layer
	}

	fn slider_layer(self) -> RigPoseLayer {
		self.sliders.apply_slider_layer()
	}
}
