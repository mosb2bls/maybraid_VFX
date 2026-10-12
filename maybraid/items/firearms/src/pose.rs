//! Playground / recipe proportion pose for kit bones.

use bevy::prelude::*;
use firearms_components::{BoneScale, ResolvedRigPose, RigPoseLayer};
use std::f32::consts::FRAC_PI_2;

use crate::parts::KitBone;

/// Blender rest is −Y forward, +Z up. The exported armature already applies the
/// glTF +90° X, so runtime rest has bore along +Z and the grip hanging −Y.
/// Yaw that onto world +X; do not pitch, or the bore goes into the ground.
pub fn aim_plus_x() -> Quat {
	Quat::from_rotation_y(FRAC_PI_2)
}

/// Length (bone local Y) and thickness (bone local XZ) multipliers. `1.0` is bind.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoneFit {
	pub length: f32,
	pub thickness: f32,
}

impl BoneFit {
	pub const BIND: Self = Self { length: 1.0, thickness: 1.0 };
	pub const COLLAPSED: Self = Self { length: 0.0, thickness: 0.0 };

	pub const fn collapsed_if(missing: bool) -> Self {
		if missing {
			Self::COLLAPSED
		} else {
			Self::BIND
		}
	}
}

impl Default for BoneFit {
	fn default() -> Self {
		Self::BIND
	}
}

/// Per-kit-bone length / thickness. Applied through [`ActiveRigPose`](firearms_components::ActiveRigPose).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct FirearmPose {
	pub body: BoneFit,
	pub barrel: BoneFit,
	pub trigger_box: BoneFit,
	pub grip: BoneFit,
	pub stock: BoneFit,
}

impl FirearmPose {
	pub fn fit(self, bone: KitBone) -> BoneFit {
		match bone {
			KitBone::Body => self.body,
			KitBone::Barrel => self.barrel,
			KitBone::TriggerBox => self.trigger_box,
			KitBone::Grip => self.grip,
			KitBone::Stock => self.stock,
		}
	}

	pub fn fit_mut(&mut self, bone: KitBone) -> &mut BoneFit {
		match bone {
			KitBone::Body => &mut self.body,
			KitBone::Barrel => &mut self.barrel,
			KitBone::TriggerBox => &mut self.trigger_box,
			KitBone::Grip => &mut self.grip,
			KitBone::Stock => &mut self.stock,
		}
	}

	pub fn to_resolved(self) -> ResolvedRigPose {
		let mut layer = RigPoseLayer::new("kit");
		for bone in KitBone::VALUES {
			let fit = self.fit(*bone);
			let name = bone.bone_name();
			layer = layer
				.with_scale(BoneScale::length(name, fit.length))
				.with_scale(BoneScale::thickness(name, fit.thickness));
		}
		ResolvedRigPose::new().with_layer(layer)
	}

	pub fn label(self) -> String {
		let mut parts = Vec::new();
		for bone in KitBone::VALUES {
			let fit = self.fit(*bone);
			if (fit.length - 1.0).abs() > f32::EPSILON || (fit.thickness - 1.0).abs() > f32::EPSILON
			{
				parts.push(format!("{} L={} T={}", bone.label(), fit.length, fit.thickness));
			}
		}
		if parts.is_empty() {
			"bind".into()
		} else {
			parts.join(" ")
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Vec3;

	#[test]
	fn aim_plus_x_sends_bore_forward_and_grip_down() {
		let q = aim_plus_x();
		assert!((q * Vec3::Z - Vec3::X).length() < 1.0e-4, "bore {}", q * Vec3::Z);
		assert!((q * Vec3::NEG_Y - Vec3::NEG_Y).length() < 1.0e-4, "grip {}", q * Vec3::NEG_Y);
	}

	#[test]
	fn length_and_thickness_compose_on_y_and_xz() {
		let mut pose = FirearmPose::default();
		pose.barrel.length = 2.0;
		pose.barrel.thickness = 0.5;
		assert_eq!(pose.to_resolved().scale_for_bone("barrel"), Vec3::new(0.5, 2.0, 0.5));
		assert_eq!(pose.to_resolved().scale_for_bone("body"), Vec3::ONE);
	}
}
