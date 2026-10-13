use bevy::prelude::*;
use bevy::window::WindowFocused;
use game_commands::command::TextEntryFocus;
use std::f32::consts::PI;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn setup_camera(mut commands: Commands) {
	let camera_pos = Vec3::new(4.0, 2.5, 6.0);
	let look_at = Vec3::new(0.0, 1.0, 0.0);
	let transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	let pitch = sin_pitch.asin();

	commands.spawn((
		Camera3d::default(),
		lod::LodViewer,
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.1, far: 4000.0, ..default() }),
		CameraController { speed: 8.0, sensitivity: 0.005, yaw, pitch },
	));
}

/// macOS screenshot (⌘⇧3 / ⌘⇧4) steals modifier key-ups. Clear them on focus
/// change so Shift does not stay held and drop the fly camera through the floor.
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
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	text_focus: Res<TextEntryFocus>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	// Screenshot selection drags the mouse; Command chords are not look/fly.
	if command_held(&keyboard_input) {
		mouse_motion.clear();
		return;
	}

	let mut mouse_delta = Vec2::ZERO;
	for event in mouse_motion.read() {
		mouse_delta += event.delta;
	}

	controller.yaw -= mouse_delta.x * controller.sensitivity;
	controller.pitch -= mouse_delta.y * controller.sensitivity;
	controller.pitch = controller.pitch.clamp(-PI / 2.0 + 0.1, PI / 2.0 - 0.1);

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

	if movement != Vec3::ZERO {
		transform.translation += movement.normalize() * controller.speed * time.delta_secs();
	}
}
