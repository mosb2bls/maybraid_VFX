//! Follow camera with POV / look-cone / FOV. Item users write [`PlayerCameraAim`].

mod follow;
mod look;

use bevy::prelude::*;
use bevy::window::WindowFocused;
use maybraid_character_controller::CharacterControlSystems;
use player::{PlayerPlugin, PlayerPoseSystems, PlayerSystems};
use std::f32::consts::FRAC_PI_2;

pub use follow::{
	apply_camera_shake, map_pose, sync_camera_fov, sync_first_person_head_visibility, CameraShake,
};
pub use look::{CameraController, CameraLookSuppressed, CameraPov, CameraPovLocked};

/// Camera schedule. Item crates add aim writers to [`PlayerCameraSystems::Aim`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlayerCameraSystems {
	Look,
	Body,
	Aim,
	Follow,
	Apply,
}

/// Per-camera follow / FOV / look-cone knobs. Runtime look state lives on [`CameraController`].
#[derive(Component, Clone, Copy, Debug)]
pub struct FollowCamera {
	pub distance: f32,
	pub height: f32,
	pub look_height: f32,
	pub shoulder_offset: f32,
	pub eye_forward: f32,
	pub focus_blend_speed: f32,
	pub third_person_fov: f32,
	pub first_person_fov: f32,
	/// ADS FOV with iron sights. Held optics override this on LT / right-mouse
	/// focus. Left-bumper ADS keeps this base FOV.
	pub sight_fov: f32,
	pub max_look_yaw: f32,
	pub body_turn_rate: f32,
	/// Radians of look per input unit at the hip-fire FOV.
	pub sensitivity: f32,
	pub near: f32,
	pub far: f32,
}

impl FollowCamera {
	pub fn hip_fov(&self, pov: CameraPov) -> f32 {
		match pov {
			CameraPov::ThirdPerson | CameraPov::Map => self.third_person_fov,
			CameraPov::FirstPerson => self.first_person_fov,
		}
	}

	/// Sensitivity scaled by zoom (the focal-length ratio against the hip-fire
	/// FOV), so a sight or optic moves the view the same share of the screen per
	/// input as hip fire does.
	pub fn look_sensitivity(&self, pov: CameraPov, fov: f32) -> f32 {
		let hip = (0.5 * self.hip_fov(pov)).tan();
		let zoomed = (0.5 * fov).tan();
		if !(hip > 0.0 && zoomed > 0.0) {
			return self.sensitivity;
		}
		self.sensitivity * (zoomed / hip).min(1.0)
	}
}

impl Default for FollowCamera {
	fn default() -> Self {
		Self {
			distance: 3.6,
			height: 1.1,
			look_height: 0.65,
			shoulder_offset: 0.7,
			eye_forward: 0.04,
			focus_blend_speed: 12.0,
			third_person_fov: 45.0_f32.to_radians(),
			first_person_fov: 75.0_f32.to_radians(),
			sight_fov: 50.0_f32.to_radians(),
			max_look_yaw: 15.0_f32.to_radians(),
			body_turn_rate: 8.0,
			sensitivity: 0.005,
			near: 0.05,
			far: 4000.0,
		}
	}
}

pub struct PlayerCameraPlugin;

impl Plugin for PlayerCameraPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<CameraPovLocked>()
			.init_resource::<look::CameraLookSuppressed>()
			.configure_sets(
				Update,
				(
					PlayerCameraSystems::Look
						.after(CharacterControlSystems)
						.before(PlayerSystems::Intent),
					PlayerCameraSystems::Body
						.after(PlayerCameraSystems::Look)
						.before(PlayerPoseSystems::Item),
					PlayerCameraSystems::Aim.after(PlayerPoseSystems::Overlay),
					PlayerCameraSystems::Follow.after(PlayerCameraSystems::Aim),
					PlayerCameraSystems::Apply.after(PlayerCameraSystems::Follow),
				),
			)
			.add_systems(
				Update,
				(look::apply_look_intents, look::sync_player_look)
					.chain()
					.in_set(PlayerCameraSystems::Look),
			)
			.add_systems(
				Update,
				(look::sync_yaw_owner, look::turn_body_with_look)
					.chain()
					.in_set(PlayerCameraSystems::Body),
			)
			.add_systems(
				Update,
				follow::follow_character_camera.in_set(PlayerCameraSystems::Follow),
			)
			.add_systems(
				Update,
				(
					follow::sync_camera_fov,
					follow::sync_first_person_head_visibility,
					follow::apply_camera_shake,
				)
					.in_set(PlayerCameraSystems::Apply),
			)
			.add_systems(Update, release_modifiers_on_focus_change);
		if app.is_plugin_added::<PlayerPlugin>() {
			app.configure_sets(Update, PlayerCameraSystems::Body.before(PlayerSystems::Locomotion));
		}
	}
}

/// Virtual ear spacing for Bevy spatial gunfire. Matches the firearms clip
/// (`0.4`): slightly wider than a human head so left/right still reads.
const SPATIAL_LISTENER_GAP: f32 = 0.4;

pub fn spawn_follow_camera(commands: &mut Commands) -> Entity {
	let follow = FollowCamera::default();
	let yaw = -FRAC_PI_2;
	let pitch = -0.12;
	commands
		.spawn((
			Camera3d::default(),
			lod::LodViewer,
			SpatialListener::new(SPATIAL_LISTENER_GAP),
			follow,
			Transform::from_translation(Vec3::new(
				-follow.distance,
				follow.height + follow.look_height,
				0.0,
			))
			.looking_at(Vec3::Y * follow.look_height, Vec3::Y),
			Projection::Perspective(PerspectiveProjection {
				fov: follow.third_person_fov,
				near: follow.near,
				far: follow.far,
				..default()
			}),
			CameraController { yaw, pitch, ..default() },
			CameraShake::default(),
		))
		.id()
}

fn release_modifiers_on_focus_change(
	mut keyboard: ResMut<ButtonInput<KeyCode>>,
	mut focus: MessageReader<WindowFocused>,
) {
	if focus.read().next().is_none() {
		return;
	}
	for key in [
		KeyCode::ShiftLeft,
		KeyCode::ShiftRight,
		KeyCode::SuperLeft,
		KeyCode::SuperRight,
		KeyCode::ControlLeft,
		KeyCode::ControlRight,
		KeyCode::AltLeft,
		KeyCode::AltRight,
	] {
		keyboard.release(key);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use characters::CharacterMotionSystems;
	use maybraid_character_controller::CharacterIntent;

	#[test]
	fn follow_camera_carries_a_spatial_listener() {
		let mut world = World::new();
		let camera = spawn_follow_camera(&mut world.commands());
		world.flush();
		let listener = world.get::<SpatialListener>(camera).cloned();
		assert!(listener.is_some());
		if let Some(listener) = listener {
			assert!(
				(listener.right_ear_offset.x - listener.left_ear_offset.x - SPATIAL_LISTENER_GAP)
					.abs() < 1e-5
			);
		}
		assert!(world.get::<CameraShake>(camera).is_some());
	}

	#[test]
	fn presentation_only_camera_allows_world_body_after_animation() {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<ButtonInput<MouseButton>>()
			.add_message::<CharacterIntent>()
			.add_message::<WindowFocused>()
			.add_plugins(player::PlayerPresentationPlugin)
			.add_plugins(PlayerCameraPlugin)
			.configure_sets(
				Update,
				(
					CharacterMotionSystems::Elevation.after(CharacterMotionSystems::Anim),
					PlayerCameraSystems::Body
						.after(CharacterMotionSystems::Anim)
						.before(CharacterMotionSystems::Elevation),
				),
			);
		app.update();
	}
}
