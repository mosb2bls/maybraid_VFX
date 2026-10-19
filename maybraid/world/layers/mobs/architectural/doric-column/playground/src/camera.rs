use std::f32::consts::PI;

use bevy::prelude::*;

#[derive(Component)]
pub struct CameraController {
	pub speed: f32,
	pub sensitivity: f32,
	pub yaw: f32,
	pub pitch: f32,
}

/// `wide` | `capital` | `caps` | `topdown` (square plinth + abacus).
pub fn preview_camera_transform() -> Transform {
	match std::env::var("DORIC_PREVIEW_SHOT").as_deref() {
		Ok("capital") => {
			Transform::from_xyz(-6.5, 9.2, 6.5).looking_at(Vec3::new(-4.0, 8.6, 0.0), Vec3::Y)
		}
		Ok("caps") => {
			Transform::from_xyz(-2.0, 2.2, 7.5).looking_at(Vec3::new(-4.0, 2.8, 0.0), Vec3::Y)
		}
		Ok("topdown") => Transform::from_translation(Vec3::new(0.0, 28.0, 0.0))
			.with_rotation(Quat::from_rotation_x(-PI / 2.0)),
		_ => Transform::from_xyz(8.0, 6.5, 14.0).looking_at(Vec3::new(0.0, 4.8, 0.0), Vec3::Y),
	}
}

pub fn setup_camera(mut commands: Commands) {
	let transform = preview_camera_transform();
	let rotation = transform.rotation;
	let (x, y, z, w) = (rotation.x, rotation.y, rotation.z, rotation.w);
	let sin_yaw = 2.0 * (w * y + x * z);
	let cos_yaw = 1.0 - 2.0 * (y * y + z * z);
	let yaw = sin_yaw.atan2(cos_yaw);
	let sin_pitch = 2.0 * (w * x - y * z);
	let pitch = sin_pitch.asin();

	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection {
			near: 0.1,
			far: 500.0,
			fov: 55.0_f32.to_radians(),
			..default()
		}),
		CameraController { speed: 10.0, sensitivity: 0.003, yaw, pitch },
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 28_000.0,
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.65, 0.55, 0.0)),
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 8_000.0,
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.35, -2.1, 0.0)),
	));
}

pub fn camera_controller(
	time: Res<Time>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mouse: Res<ButtonInput<MouseButton>>,
	mut scroll: MessageReader<bevy::input::mouse::MouseWheel>,
	mut motion: MessageReader<bevy::input::mouse::MouseMotion>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};
	if mouse.pressed(MouseButton::Right) {
		for ev in motion.read() {
			controller.yaw -= ev.delta.x * controller.sensitivity;
			controller.pitch = (controller.pitch - ev.delta.y * controller.sensitivity)
				.clamp(-PI / 2.0 + 0.05, PI / 2.0 - 0.05);
		}
	}
	let mut velocity = Vec3::ZERO;
	if keyboard.pressed(KeyCode::KeyW) {
		velocity.z -= 1.0;
	}
	if keyboard.pressed(KeyCode::KeyS) {
		velocity.z += 1.0;
	}
	if keyboard.pressed(KeyCode::KeyA) {
		velocity.x -= 1.0;
	}
	if keyboard.pressed(KeyCode::KeyD) {
		velocity.x += 1.0;
	}
	if keyboard.pressed(KeyCode::Space) {
		velocity.y += 1.0;
	}
	if keyboard.pressed(KeyCode::ShiftLeft) {
		velocity.y -= 1.0;
	}
	for ev in scroll.read() {
		controller.speed = (controller.speed + ev.y * 0.5).clamp(2.0, 40.0);
	}
	if velocity.length_squared() > 0.0 {
		velocity = velocity.normalize();
	}
	let rotation = Quat::from_euler(EulerRot::YXZ, controller.yaw, controller.pitch, 0.0);
	transform.translation += rotation * velocity * controller.speed * time.delta_secs();
	transform.rotation = rotation;
}
