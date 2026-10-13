//! Croconot rig and feature slider values for the quadruped skeleton.

use bevy::prelude::*;
use character_rigs::{BoneScale, RigPoseLayer};

use crate::assembly::CharacterPartSlot;

/// Body rig and feature mesh sliders for the concepts pass.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CroconotSliders {
	pub shoulder_width: f32,
	pub hip_width: f32,
	pub chest_thickness: f32,
	pub chest_width: f32,
	pub hip_thickness: f32,
	pub leg_thickness: f32,
	pub buttocks_thickness: f32,
	pub waist_thickness: f32,
	pub lower_trunk_thickness: f32,
	pub arm_length: f32,
	pub arm_thickness: f32,
	pub leg_length: f32,
	pub eye_width: f32,
	pub eye_height: f32,
	pub eye_tilt: f32,
	pub ear_width: f32,
	pub ear_height: f32,
	pub snout_length: f32,
}

impl Default for CroconotSliders {
	fn default() -> Self {
		Self {
			shoulder_width: 1.0,
			hip_width: 1.0,
			chest_thickness: 1.0,
			chest_width: 1.0,
			hip_thickness: 1.0,
			leg_thickness: 1.0,
			buttocks_thickness: 1.0,
			waist_thickness: 1.0,
			lower_trunk_thickness: 1.0,
			arm_length: 1.0,
			arm_thickness: 1.0,
			leg_length: 1.0,
			eye_width: 1.2,
			eye_height: 1.2,
			eye_tilt: 0.0,
			ear_width: 1.0,
			ear_height: 1.0,
			snout_length: 1.0,
		}
	}
}

impl CroconotSliders {
	pub fn clamped(mut self) -> Self {
		self.shoulder_width = self.shoulder_width.clamp(0.8, 1.2);
		self.hip_width = self.hip_width.clamp(0.8, 1.4);
		self.chest_width = self.chest_width.clamp(0.6, 1.0);
		self.chest_thickness = self.chest_thickness.clamp(0.8, 1.2);
		self.hip_thickness = self.hip_thickness.clamp(0.8, 1.2);
		self.leg_thickness = self.leg_thickness.clamp(0.8, 1.2);
		self.buttocks_thickness = self.buttocks_thickness.clamp(0.8, 1.2);
		self.waist_thickness = self.waist_thickness.clamp(0.8, 1.2);
		self.lower_trunk_thickness = self.lower_trunk_thickness.clamp(0.8, 1.2);
		self.arm_length = self.arm_length.clamp(0.8, 1.2);
		self.arm_thickness = self.arm_thickness.clamp(0.8, 1.2);
		self.leg_length = self.leg_length.clamp(0.8, 1.2);
		self.eye_width = self.eye_width.clamp(0.8, 1.2);
		self.eye_height = self.eye_height.clamp(0.8, 1.2);
		self.eye_tilt = self.eye_tilt.clamp(-30.0, 30.0);
		self.ear_width = self.ear_width.clamp(0.8, 1.2);
		self.ear_height = self.ear_height.clamp(0.8, 1.2);
		self.snout_length = self.snout_length.clamp(0.8, 1.2);
		self
	}

	pub fn with_shoulder_width(mut self, value: f32) -> Self {
		self.shoulder_width = value;
		self.clamped()
	}

	pub fn with_hip_width(mut self, value: f32) -> Self {
		self.hip_width = value;
		self.clamped()
	}

	pub fn with_chest_thickness(mut self, value: f32) -> Self {
		self.chest_thickness = value;
		self.clamped()
	}

	pub fn apply_slider_layer(self) -> RigPoseLayer {
		let layer = RigPoseLayer::new("command sliders");
		let layer = Self::apply_shoulder_width(layer, self.shoulder_width);
		let layer = Self::apply_hip_width(layer, self.hip_width);
		let layer = Self::apply_chest_thickness(layer, self.chest_thickness);
		let layer = Self::apply_hip_thickness(layer, self.hip_thickness);
		let layer = Self::apply_leg_thickness(layer, self.leg_thickness);
		let layer = Self::apply_buttocks_thickness(layer, self.buttocks_thickness);
		let layer = Self::apply_waist_thickness(layer, self.waist_thickness);
		let layer = Self::apply_lower_trunk_thickness(layer, self.lower_trunk_thickness);
		let layer = Self::apply_arm_length(layer, self.arm_length);
		let layer = Self::apply_arm_thickness(layer, self.arm_thickness);
		Self::apply_leg_length(layer, self.leg_length)
	}

	pub fn apply_shoulder_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("shoulder.L", value))
			.with_scale(BoneScale::length("shoulder.R", value))
			.with_scale(BoneScale::uniform("lateral_shoulder_protrusion.L", value))
			.with_scale(BoneScale::uniform("lateral_shoulder_protrusion.R", value))
	}

	pub fn apply_hip_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("hip.L", value))
			.with_scale(BoneScale::length("hip.R", value))
			.with_scale(BoneScale::uniform("lateral_hip_protrusion.L", value))
			.with_scale(BoneScale::uniform("later_hip_protrusion.R", value))
	}

	pub fn apply_hip_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::uniform("haunch_vertical_thickness", value))
	}

	pub fn apply_chest_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::uniform("chest_thickness", value))
	}

	pub fn apply_leg_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::thickness("anterior_thigh.L", value))
			.with_scale(BoneScale::thickness("anterior_thigh.R", value))
			.with_scale(BoneScale::thickness("posterior_thigh.L", value))
			.with_scale(BoneScale::thickness("posterior_thigh.R", value))
			.with_scale(BoneScale::thickness("anterior_shin.L", value))
			.with_scale(BoneScale::thickness("anterior_shin.R", value))
			.with_scale(BoneScale::thickness("posterior_shin.L", value))
			.with_scale(BoneScale::thickness("posterior_shin.R", value))
	}

	pub fn apply_buttocks_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::uniform("belly", value))
	}

	pub fn apply_waist_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("waist.L", value))
			.with_scale(BoneScale::length("waist.R", value))
	}

	pub fn apply_lower_trunk_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::thickness("lumbar", value))
	}

	/// Front-leg length on the quadruped rig.
	pub fn apply_arm_length(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("anterior_thigh.L", value))
			.with_scale(BoneScale::length("anterior_thigh.R", value))
			.with_scale(BoneScale::length("anterior_shin.L", value))
			.with_scale(BoneScale::length("anterior_shin.R", value))
	}

	pub fn apply_arm_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::thickness("anterior_thigh.L", value))
			.with_scale(BoneScale::thickness("anterior_thigh.R", value))
			.with_scale(BoneScale::thickness("anterior_shin.L", value))
			.with_scale(BoneScale::thickness("anterior_shin.R", value))
	}

	/// Hind-leg length on the quadruped rig.
	pub fn apply_leg_length(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("posterior_thigh.L", value))
			.with_scale(BoneScale::length("posterior_thigh.R", value))
			.with_scale(BoneScale::length("posterior_shin.L", value))
			.with_scale(BoneScale::length("posterior_shin.R", value))
	}

	pub fn apply_chest_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::uniform("lower_chest_width.L", value))
			.with_scale(BoneScale::uniform("lower_chest_width.R", value))
	}

	pub fn feature_transform(self, slot: CharacterPartSlot) -> Transform {
		match slot {
			CharacterPartSlot::EyeLeft => Transform {
				scale: Vec3::new(self.eye_width, self.eye_height, 1.0),
				rotation: Quat::from_rotation_z(self.eye_tilt.to_radians()),
				..default()
			},
			CharacterPartSlot::EyeRight => Transform {
				scale: Vec3::new(self.eye_width, self.eye_height, 1.0),
				rotation: Quat::from_rotation_z(-self.eye_tilt.to_radians()),
				..default()
			},
			CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
				Transform::from_scale(Vec3::new(self.ear_width, self.ear_height, 1.0))
			}
			// Snout length is along local Z (matches the fixed SNOUT_Z_SCALE on the mouth socket).
			CharacterPartSlot::Mouth => {
				Transform::from_scale(Vec3::new(1.0, 1.0, self.snout_length))
			}
			_ => Transform::IDENTITY,
		}
	}

	pub fn status_label(self) -> String {
		format!(
			"shoulder_width={:.2} hip_width={:.2} chest_thickness={:.2} \
			 hip_thickness={:.2} leg_thickness={:.2} buttocks_thickness={:.2} \
			 waist_thickness={:.2} lower_trunk_thickness={:.2} \
			 arm_length={:.2} arm_thickness={:.2} leg_length={:.2} \
			 eye={:.2}x{:.2} tilt={:.1} ear={:.2}x{:.2} snout_length={:.2}",
			self.shoulder_width,
			self.hip_width,
			self.chest_thickness,
			self.hip_thickness,
			self.leg_thickness,
			self.buttocks_thickness,
			self.waist_thickness,
			self.lower_trunk_thickness,
			self.arm_length,
			self.arm_thickness,
			self.leg_length,
			self.eye_width,
			self.eye_height,
			self.eye_tilt,
			self.ear_width,
			self.ear_height,
			self.snout_length,
		)
	}
}
