use std::f32::consts::PI;

use bevy::prelude::*;
use bevy::window::WindowFocused;
use maybraid_spatial_rumble::SpatialRumbleListener;

#[derive(Component)]
pub struct FlyCamera {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn setup_camera(mut commands: Commands) {
	let translation = Vec3::new(4.5, 2.2, 6.5);
	let transform = Transform::from_translation(translation).looking_at(Vec3::Y * 0.4, Vec3::Y);
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let yaw = (2.0 * (w * y + x * z)).atan2(1.0 - 2.0 * (y * y + z * z));
	let pitch = (2.0 * (w * x - y * z)).asin();
	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.05, far: 400.0, ..default() }),
		FlyCamera { speed: 8.0, sensitivity: 0.005, yaw, pitch },
		SpatialRumbleListener::default(),
	));
}

pub fn release_modifiers_on_focus_change(
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

fn command_held(keyboard: &ButtonInput<KeyCode>) -> bool {
	keyboard.pressed(KeyCode::SuperLeft) || keyboard.pressed(KeyCode::SuperRight)
}

pub fn camera_controller(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	mut cameras: Query<(&mut Transform, &mut FlyCamera), With<Camera3d>>,
) {
	let Ok((mut transform, mut fly)) = cameras.single_mut() else {
		return;
	};
	if command_held(&keyboard) {
		mouse_motion.clear();
		return;
	}
	let mut mouse_delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		mouse_delta += event.delta;
	}
	fly.yaw -= mouse_delta.x * fly.sensitivity;
	fly.pitch -= mouse_delta.y * fly.sensitivity;
	fly.pitch = fly.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);
	let yaw = Quat::from_axis_angle(Vec3::Y, fly.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, fly.pitch);
	transform.rotation = yaw * pitch;

	let mut movement = Vec3::ZERO;
	let forward = transform.forward();
	let right = transform.right();
	if keyboard.pressed(KeyCode::KeyW) {
		movement += *forward;
	}
	if keyboard.pressed(KeyCode::KeyS) {
		movement -= *forward;
	}
	if keyboard.pressed(KeyCode::KeyA) {
		movement -= *right;
	}
	if keyboard.pressed(KeyCode::KeyD) {
		movement += *right;
	}
	if keyboard.pressed(KeyCode::Space) {
		movement += Vec3::Y;
	}
	if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
		movement -= Vec3::Y;
	}
	if movement != Vec3::ZERO {
		transform.translation += movement.normalize() * fly.speed * time.delta_secs();
	}
}
