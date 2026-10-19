//! Isolated host for spatial rumble presets.

mod camera;
mod ground;
mod ui;

use bevy::prelude::*;
use maybraid_input::VirtualPadPlugin;
use maybraid_spatial_rumble::{SpatialRumble, SpatialRumblePlugin, SpatialRumbleSystems};

use crate::camera::{camera_controller, release_modifiers_on_focus_change, setup_camera};
use crate::ground::{setup_ground, sync_ground_presentation};
use crate::ui::{setup_status, sync_status_text};

/// Optional argv-driven capture path: fire stacked presets, then optionally exit.
#[derive(Resource, Clone, Debug)]
pub struct PlaygroundDemo {
	pub enabled: bool,
	pub quit_after: Option<f32>,
	next_fire: f32,
}

impl Default for PlaygroundDemo {
	fn default() -> Self {
		Self { enabled: false, quit_after: None, next_fire: 0.1 }
	}
}

impl PlaygroundDemo {
	pub fn from_args<I, S>(args: I) -> Self
	where
		I: IntoIterator<Item = S>,
		S: AsRef<str>,
	{
		let mut demo = Self::default();
		let mut args = args.into_iter().map(|s| s.as_ref().to_string());
		while let Some(arg) = args.next() {
			if arg == "demo" {
				demo.enabled = true;
			}
			if arg == "--quit-after" {
				if let Some(value) = args.next() {
					if let Ok(secs) = value.parse::<f32>() {
						demo.quit_after = Some(secs.max(0.1));
						demo.enabled = true;
					}
				}
			}
		}
		demo
	}
}

pub struct SpatialRumblePlaygroundPlugin;

impl Plugin for SpatialRumblePlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(VirtualPadPlugin::default())
			.add_plugins(SpatialRumblePlugin)
			.init_resource::<PlaygroundDemo>()
			.add_systems(Startup, (setup_camera, setup_lighting, setup_ground, setup_status))
			.add_systems(
				Update,
				(
					release_modifiers_on_focus_change.before(camera_controller),
					camera_controller.before(SpatialRumbleSystems::Present),
					dispatch_presets.before(SpatialRumbleSystems::Ingest),
					run_demo.before(SpatialRumbleSystems::Ingest),
					toggle_pause,
					sync_ground_presentation.after(SpatialRumbleSystems::Compose),
					sync_status_text.after(SpatialRumbleSystems::Compose),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		brightness: 140.0,
		color: Color::srgb(0.55, 0.62, 0.78),
		..default()
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 2800.0,
			color: Color::srgb(1.0, 0.93, 0.84),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 6.0, 0.0)),
	));
}

fn dispatch_presets(
	keyboard: Res<ButtonInput<KeyCode>>,
	mut rumbles: MessageWriter<SpatialRumble>,
) {
	if keyboard.just_pressed(KeyCode::Digit1) || keyboard.just_pressed(KeyCode::Numpad1) {
		rumbles.write(SpatialRumble::bray(Vec3::ZERO).with_seed(1));
	}
	if keyboard.just_pressed(KeyCode::Digit2) || keyboard.just_pressed(KeyCode::Numpad2) {
		rumbles.write(SpatialRumble::heavy_landing(Vec3::ZERO).with_seed(2));
	}
	if keyboard.just_pressed(KeyCode::Digit3) || keyboard.just_pressed(KeyCode::Numpad3) {
		rumbles.write(SpatialRumble::bray(Vec3::ZERO).with_seed(3));
		rumbles.write(SpatialRumble::heavy_landing(Vec3::ZERO).with_seed(4));
	}
	if keyboard.just_pressed(KeyCode::Digit4) || keyboard.just_pressed(KeyCode::Numpad4) {
		rumbles.write(SpatialRumble::bray(Vec3::new(28.0, 0.0, 0.0)).with_seed(5));
	}
}

fn run_demo(
	time: Res<Time>,
	mut demo: ResMut<PlaygroundDemo>,
	mut rumbles: MessageWriter<SpatialRumble>,
	mut exit: MessageWriter<AppExit>,
) {
	if !demo.enabled {
		return;
	}
	if time.elapsed_secs() >= demo.next_fire {
		rumbles.write(SpatialRumble::bray(Vec3::ZERO).with_seed(3));
		rumbles.write(SpatialRumble::heavy_landing(Vec3::ZERO).with_seed(4));
		demo.next_fire = time.elapsed_secs() + 0.85;
	}
	if let Some(limit) = demo.quit_after {
		if time.elapsed_secs() >= limit {
			exit.write(AppExit::Success);
		}
	}
}

fn toggle_pause(keyboard: Res<ButtonInput<KeyCode>>, mut time: ResMut<Time<Virtual>>) {
	if keyboard.just_pressed(KeyCode::KeyP) {
		if time.is_paused() {
			time.unpause();
		} else {
			time.pause();
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn presets_share_the_dispatch_message() {
		let bray = SpatialRumble::bray(Vec3::ZERO);
		let landing = SpatialRumble::heavy_landing(Vec3::ZERO);
		assert!(bray.channels.camera && landing.channels.ground);
		assert!(bray.duration > landing.duration);
	}

	#[test]
	fn demo_args_enable_stacked_capture() {
		let demo =
			PlaygroundDemo::from_args(["spatial-rumble-playground", "demo", "--quit-after", "1.5"]);
		assert!(demo.enabled);
		assert!((demo.quit_after.unwrap_or(0.0) - 1.5).abs() < 1e-5);
	}
}
