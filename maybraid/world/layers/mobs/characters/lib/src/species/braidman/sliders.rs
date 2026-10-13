//! Braidman rig and feature slider values.
//!
//! Values are multipliers on top of the species baseline. `1.0` means "leave
//! Braidman's baseline intact", not "write an identity transform over the bind
//! pose". Feature mesh scales follow the same convention on the spawned part
//! transform; [`eye_tilt`](BraidmanSliders::eye_tilt) is in degrees.

use bevy::prelude::*;
use character_rigs::{BoneScale, RigPoseLayer};
use serde::{Deserialize, Serialize};

use crate::assembly::CharacterPartSlot;

/// Body rig and feature mesh sliders for the concepts pass.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct BraidmanSliders {
	pub shoulder_width: f32,
	pub hip_width: f32,
	pub chest_thickness: f32,
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
	pub nose_width: f32,
	pub nose_height: f32,
	pub mouth_width: f32,
	pub mouth_height: f32,
	pub ear_width: f32,
	pub ear_height: f32,
}

impl Default for BraidmanSliders {
	fn default() -> Self {
		Self {
			shoulder_width: 1.0,
			hip_width: 1.0,
			chest_thickness: 1.0,
			hip_thickness: 1.0,
			leg_thickness: 1.0,
			buttocks_thickness: 1.0,
			waist_thickness: 1.0,
			lower_trunk_thickness: 1.0,
			arm_length: 1.0,
			arm_thickness: 1.0,
			leg_length: 1.0,
			eye_width: 1.0,
			eye_height: 1.0,
			eye_tilt: 0.0,
			nose_width: 1.0,
			nose_height: 1.0,
			mouth_width: 1.0,
			mouth_height: 1.0,
			ear_width: 1.0,
			ear_height: 1.0,
		}
	}
}

impl BraidmanSliders {
	pub fn clamped(mut self) -> Self {
		// Ranges mirror spec Braidman slider bounds for the lean pass.
		self.shoulder_width = self.shoulder_width.clamp(0.8, 1.2);
		self.hip_width = self.hip_width.clamp(0.8, 1.4);
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
		self.nose_width = self.nose_width.clamp(0.8, 1.2);
		self.nose_height = self.nose_height.clamp(0.8, 1.2);
		self.mouth_width = self.mouth_width.clamp(0.8, 1.2);
		self.mouth_height = self.mouth_height.clamp(0.8, 1.2);
		self.ear_width = self.ear_width.clamp(0.8, 1.2);
		self.ear_height = self.ear_height.clamp(0.8, 1.2);
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

	pub fn with_hip_thickness(mut self, value: f32) -> Self {
		self.hip_thickness = value;
		self.clamped()
	}

	pub fn with_thigh_thickness(self, value: f32) -> Self {
		self.with_leg_thickness(value)
	}

	pub fn with_leg_thickness(mut self, value: f32) -> Self {
		self.leg_thickness = value;
		self.clamped()
	}

	pub fn with_buttocks_thickness(mut self, value: f32) -> Self {
		self.buttocks_thickness = value;
		self.clamped()
	}

	pub fn with_waist_thickness(mut self, value: f32) -> Self {
		self.waist_thickness = value;
		self.clamped()
	}

	pub fn with_lower_trunk_thickness(mut self, value: f32) -> Self {
		self.lower_trunk_thickness = value;
		self.clamped()
	}

	pub fn with_arm_length(mut self, value: f32) -> Self {
		self.arm_length = value;
		self.clamped()
	}

	pub fn with_arm_thickness(mut self, value: f32) -> Self {
		self.arm_thickness = value;
		self.clamped()
	}

	pub fn with_leg_length(mut self, value: f32) -> Self {
		self.leg_length = value;
		self.clamped()
	}

	pub fn with_eye_width(mut self, value: f32) -> Self {
		self.eye_width = value;
		self.clamped()
	}

	pub fn with_eye_height(mut self, value: f32) -> Self {
		self.eye_height = value;
		self.clamped()
	}

	pub fn with_eye_tilt(mut self, value: f32) -> Self {
		self.eye_tilt = value;
		self.clamped()
	}

	pub fn with_nose_width(mut self, value: f32) -> Self {
		self.nose_width = value;
		self.clamped()
	}

	pub fn with_nose_height(mut self, value: f32) -> Self {
		self.nose_height = value;
		self.clamped()
	}

	pub fn with_mouth_width(mut self, value: f32) -> Self {
		self.mouth_width = value;
		self.clamped()
	}

	pub fn with_mouth_height(mut self, value: f32) -> Self {
		self.mouth_height = value;
		self.clamped()
	}

	pub fn with_ear_width(mut self, value: f32) -> Self {
		self.ear_width = value;
		self.clamped()
	}

	pub fn with_ear_height(mut self, value: f32) -> Self {
		self.ear_height = value;
		self.clamped()
	}

	/// User/command slider layer applied after species baseline and presets.
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
		// Shoulder bones carry width in uniform local scale on this rig.
		layer
			.with_scale(BoneScale::length("shoulder.L", value))
			.with_scale(BoneScale::length("shoulder.R", value))
	}

	pub fn apply_hip_width(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		// Pelvis width is authored along bone Y (length), not lateral X.
		layer
			.with_scale(BoneScale::length("pelvis.L", value))
			.with_scale(BoneScale::length("pelvis.R", value))
	}

	pub fn apply_hip_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::thickness("pelvis.L", value))
			.with_scale(BoneScale::thickness("pelvis.R", value))
	}

	pub fn apply_chest_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		// Ventrally oriented control bone: uniform scale, not thickness helper.
		layer.with_scale(BoneScale::uniform("chest_thickness", value))
	}

	pub fn apply_thigh_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		Self::apply_leg_thickness(layer, value)
	}

	pub fn apply_leg_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::uniform("thigh_thickness.L", value))
			.with_scale(BoneScale::uniform("thigh_thickness.R", value))
			.with_scale(BoneScale::uniform("calf_thickness.L", value))
			.with_scale(BoneScale::uniform("calf_thickness.R", value))
	}

	pub fn apply_buttocks_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::uniform("buttocks", value))
	}

	pub fn apply_waist_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("waist.L", value))
			.with_scale(BoneScale::length("waist.R", value))
	}

	pub fn apply_lower_trunk_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer.with_scale(BoneScale::thickness("lumbar", value))
	}

	pub fn apply_arm_length(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("humerus.L", value))
			.with_scale(BoneScale::length("humerus.R", value))
			.with_scale(BoneScale::length("forearm.L", value))
			.with_scale(BoneScale::length("forearm.R", value))
	}

	pub fn apply_arm_thickness(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::thickness("humerus.L", value))
			.with_scale(BoneScale::thickness("humerus.R", value))
			.with_scale(BoneScale::thickness("forearm.L", value))
			.with_scale(BoneScale::thickness("forearm.R", value))
	}

	pub fn apply_leg_length(layer: RigPoseLayer, value: f32) -> RigPoseLayer {
		layer
			.with_scale(BoneScale::length("femur.L", value))
			.with_scale(BoneScale::length("femur.R", value))
			.with_scale(BoneScale::length("shin.L", value))
			.with_scale(BoneScale::length("shin.R", value))
	}

	/// Per-feature mesh scale/rotation composed with asset normalization at spawn.
	pub fn feature_transform(self, slot: CharacterPartSlot) -> Transform {
		match slot {
			CharacterPartSlot::EyeLeft => Transform {
				scale: Vec3::new(self.eye_width, self.eye_height, 1.0),
				rotation: Quat::from_rotation_z(self.eye_tilt.to_radians()),
				..default()
			},
			// Right eye sockets mirror X; negate tilt so cant is symmetric in world space.
			CharacterPartSlot::EyeRight => Transform {
				scale: Vec3::new(self.eye_width, self.eye_height, 1.0),
				rotation: Quat::from_rotation_z(-self.eye_tilt.to_radians()),
				..default()
			},
			CharacterPartSlot::Nose => {
				Transform::from_scale(Vec3::new(self.nose_width, self.nose_height, 1.0))
			}
			CharacterPartSlot::Mouth => {
				Transform::from_scale(Vec3::new(self.mouth_width, self.mouth_height, 1.0))
			}
			CharacterPartSlot::EarLeft | CharacterPartSlot::EarRight => {
				Transform::from_scale(Vec3::new(self.ear_width, self.ear_height, 1.0))
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
			 eye={:.2}x{:.2} tilt={:.1} nose={:.2}x{:.2} mouth={:.2}x{:.2} ear={:.2}x{:.2}",
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
			self.nose_width,
			self.nose_height,
			self.mouth_width,
			self.mouth_height,
			self.ear_width,
			self.ear_height,
		)
	}
}
