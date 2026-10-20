//! Synthesized targets other systems query. Item users write the aim/use slots.

use bevy::prelude::*;

#[derive(Component)]
pub struct Player;

/// Capsule agent that is not the followed player. Shares look / pose slots, not pad or camera.
#[derive(Component)]
pub struct Npc;

#[derive(Component)]
pub struct CameraFollow;

/// Nested character host on the player capsule.
#[derive(Component)]
pub struct PlayerVisual;

#[derive(Component)]
pub struct PlayerCapsule;

/// Look copied off the follow camera. Item users read this instead of `Camera3d`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerLook {
	pub yaw: f32,
	pub pitch: f32,
	pub first_person: bool,
	/// ADS pose blend (optic or iron). Optic zoom is camera `focus`, not this.
	pub focus: f32,
}

impl PlayerLook {
	/// Camera-relative XZ wish from a stick-space vector.
	pub fn wish_from_stick(self, stick: Vec2) -> Vec3 {
		Self::wish_from_yaw(self.yaw, stick)
	}

	/// Camera-relative XZ wish from an explicit yaw and stick-space vector.
	pub fn wish_from_yaw(yaw: f32, stick: Vec2) -> Vec3 {
		if stick == Vec2::ZERO {
			return Vec3::ZERO;
		}
		let yaw = Quat::from_axis_angle(Vec3::Y, yaw);
		let forward = yaw * -Vec3::Z;
		let right = yaw * Vec3::X;
		(right * stick.x + forward * stick.y).clamp_length_max(1.0)
	}
}

/// Camera override written by a use-driver (sights, lock-on). Follow lerps by `focus`.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PlayerCameraAim {
	pub pose: Option<PlayerCameraPose>,
	pub focus: f32,
	/// ADS vertical FOV from the held optic. `None` keeps the follow-camera iron-sight FOV.
	pub sight_fov: Option<f32>,
}

#[derive(Clone, Copy, Debug)]
pub struct PlayerCameraPose {
	pub translation: Vec3,
	pub rotation: Quat,
}

impl PlayerCameraPose {
	pub fn interpolate(self, other: Self, amount: f32) -> Self {
		Self {
			translation: self.translation.lerp(other.translation, amount),
			rotation: self.rotation.slerp(other.rotation, amount),
		}
	}

	pub fn transform(self) -> Transform {
		Transform::from_translation(self.translation).with_rotation(self.rotation)
	}
}

/// Who owns [`PlayerVisual`] yaw: locomotion wish, or first-person look cone.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PlayerYawOwner {
	#[default]
	Wish,
	Look,
}

/// Active extra driver (held gun, melee, …). Camera/pose overlays key off this.
#[derive(Component, Clone, Copy, Debug)]
pub struct PlayerUse {
	pub driver: Entity,
}
