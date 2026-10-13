use bevy::prelude::*;
use camera_controls::look::{look_input_active, CameraLookEnabled};
use game_commands::command::TextEntryFocus;
use std::f32::consts::PI;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

pub fn orient_camera(
	transform: &mut Transform,
	controller: &mut CameraController,
	eye: Vec3,
	target: Vec3,
) {
	*transform = Transform::from_translation(eye).looking_at(target, Vec3::Y);
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	controller.yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	controller.pitch = sin_pitch.asin();
}

pub fn setup_camera(mut commands: Commands) {
	let mut transform = Transform::IDENTITY;
	let mut controller = CameraController { speed: 6.0, sensitivity: 0.005, yaw: 0.0, pitch: 0.0 };
	orient_camera(
		&mut transform,
		&mut controller,
		Vec3::new(0.0, 1.6, 3.5),
		Vec3::new(0.0, 1.0, 0.0),
	);
	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.1, far: 4000.0, ..default() }),
		controller,
	));
}

pub fn camera_controller(
	keyboard_input: Res<ButtonInput<KeyCode>>,
	mut mouse_motion: MessageReader<bevy::input::mouse::MouseMotion>,
	time: Res<Time>,
	text_focus: Res<TextEntryFocus>,
	look_enabled: Option<Res<CameraLookEnabled>>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	if look_input_active(look_enabled) {
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
	} else {
		mouse_motion.clear();
	}

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

	if movement.length_squared() > 0.0 {
		movement = movement.normalize() * controller.speed * time.delta_secs();
		transform.translation += movement;
	}
}
