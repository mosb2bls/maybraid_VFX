use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use durham::{BaseTerrainNoise, TerrainCellLayout};
use game_commands::command::TextEntryFocus;
use lod::LodViewer;
use std::f32::consts::PI;

use crate::player::PlaygroundMode;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn setup_camera(
	mut commands: Commands,
	layout: Res<TerrainCellLayout>,
	world_base: Res<crate::WorldBaseTerrain>,
) {
	let look_at = camera_look_at(&layout, &world_base.0);
	let camera_pos = look_at + Vec3::new(0.0, layout.cell_size * 8.0, layout.cell_size * 12.0);
	let transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);

	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.1, far: 40_000.0, ..default() }),
		CameraController { speed: 80.0, sensitivity: 0.005, yaw, pitch },
		LodViewer,
		Msaa::Off,
		DepthPrepass,
	));
}

/// Recenters the camera above the current cell-layout region using base elevation.
pub fn refocus_camera_on_layout(
	layout: &TerrainCellLayout,
	base: &BaseTerrainNoise,
	transform: &mut Transform,
	controller: &mut CameraController,
) {
	let look_at = camera_look_at(layout, base);
	let camera_pos = look_at + Vec3::new(0.0, layout.cell_size * 8.0, layout.cell_size * 12.0);
	*transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
	controller.yaw = yaw;
	controller.pitch = pitch;
}

fn camera_look_at(layout: &TerrainCellLayout, base: &BaseTerrainNoise) -> Vec3 {
	let center = layout.region_center_xz();
	let elevation = base.height_at(center.x, center.z);
	Vec3::new(center.x, elevation, center.z)
}

fn yaw_pitch_from_rotation(rotation: Quat) -> (f32, f32) {
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = (2.0 * (w * x - y * z)).clamp(-1.0, 1.0);
	let pitch = sin_pitch.asin();
	(yaw, pitch)
}

pub fn camera_controller(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	text_focus: Res<TextEntryFocus>,
	mode: Res<PlaygroundMode>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	let mut mouse_delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		mouse_delta += event.delta;
	}

	controller.yaw -= mouse_delta.x * controller.sensitivity;
	controller.pitch -= mouse_delta.y * controller.sensitivity;
	controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);

	if *mode == PlaygroundMode::Character {
		return;
	}

	let yaw_quat = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch_quat = Quat::from_axis_angle(Vec3::X, controller.pitch);
	transform.rotation = yaw_quat * pitch_quat;

	if text_focus.0 {
		return;
	}

	let mut movement = Vec3::ZERO;
	let forward = transform.forward();
	let right = transform.right();

	if keyboard_input.pressed(KeyCode::KeyW) {
		movement += *forward;
	}
	if keyboard_input.pressed(KeyCode::KeyS) {
		movement -= *forward;
	}
	if keyboard_input.pressed(KeyCode::KeyA) {
		movement -= *right;
	}
	if keyboard_input.pressed(KeyCode::KeyD) {
		movement += *right;
	}
	if keyboard_input.pressed(KeyCode::Space) {
		movement += Vec3::Y;
	}
	if keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight) {
		movement -= Vec3::Y;
	}

	if movement.length_squared() > 0.0 {
		movement = movement.normalize() * controller.speed * time.delta_secs();
		transform.translation += movement;
	}
}
