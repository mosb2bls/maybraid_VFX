use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use bevy::window::WindowFocused;
use durham::{BaseTerrainNoise, DurhamSurface, TerrainCellLayout};
use game_commands::command::TextEntryFocus;
use lod::LodViewer;
use maybraid_input::{PadButton, VirtualPad};
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
	let center = layout.region_center_xz();
	let look_at = camera_look_at(
		&layout,
		crate::player::holding_elevation(&world_base.0, center.x, center.z),
	);
	let camera_pos = look_at + Vec3::new(0.0, 24.0, 48.0);
	let transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);

	commands.spawn((
		Camera3d::default(),
		transform,
		Projection::Perspective(PerspectiveProjection { near: 0.1, far: 8_000.0, ..default() }),
		DistanceFog {
			color: Color::srgba(0.55, 0.65, 0.72, 1.0),
			directional_light_color: Color::srgba(1.0, 0.92, 0.78, 0.35),
			directional_light_exponent: 24.0,
			falloff: FogFalloff::Linear { start: 700.0, end: 4500.0 },
		},
		CameraController { speed: 40.0, sensitivity: 0.005, yaw, pitch },
		LodViewer,
		Msaa::Off,
		DepthPrepass,
	));
}

pub fn refocus_camera_on_layout(
	layout: &TerrainCellLayout,
	base: &BaseTerrainNoise,
	transform: &mut Transform,
	controller: &mut CameraController,
) {
	let center = layout.region_center_xz();
	refocus_camera_on_elevation(
		layout,
		crate::player::holding_elevation(base, center.x, center.z),
		transform,
		controller,
	);
}

pub fn refocus_camera_on_elevation(
	layout: &TerrainCellLayout,
	elevation: f32,
	transform: &mut Transform,
	controller: &mut CameraController,
) {
	let look_at = camera_look_at(layout, elevation);
	let camera_pos = look_at + Vec3::new(0.0, 24.0, 48.0);
	*transform = Transform::from_translation(camera_pos).looking_at(look_at, Vec3::Y);
	let (yaw, pitch) = yaw_pitch_from_rotation(transform.rotation);
	controller.yaw = yaw;
	controller.pitch = pitch;
}

/// Composed height at the layout's center once its cell is published;
/// otherwise the holding altitude.
pub fn surface_or_hold(surface: &DurhamSurface) -> f32 {
	crate::player::surface_or_hold_at(surface, surface.layout().region_center_xz().xz())
}

fn camera_look_at(layout: &TerrainCellLayout, elevation: f32) -> Vec3 {
	let center = layout.region_center_xz();
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
	pad: Res<VirtualPad>,
	time: Res<Time>,
	text_focus: Res<TextEntryFocus>,
	mode: Res<PlaygroundMode>,
	mut query: Query<(&mut Transform, &mut CameraController), With<Camera3d>>,
) {
	let Ok((mut transform, mut controller)) = query.single_mut() else {
		return;
	};

	// Screenshot selection drags the mouse; Command chords are not look/fly.
	if command_held(&keyboard_input) {
		return;
	}

	controller.yaw -= pad.look_stick.x * controller.sensitivity;
	controller.pitch -= pad.look_stick.y * controller.sensitivity;
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
	let wish = pad.move_stick;
	movement += *forward * wish.y;
	movement += *right * wish.x;
	if pad.pressed(PadButton::A) {
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
