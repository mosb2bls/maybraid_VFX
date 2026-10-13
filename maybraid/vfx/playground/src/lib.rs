//! Isolated host for real-time VFX concepts.

mod camera;
pub mod commands;
mod ground;
mod show;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use game_commands::command::GameCommandPlugin;
use ground::setup_ground;
use maybraid_vfx::VfxPlugin;
use show::{
	apply_pending_shows, pause_at_frozen_age, repeat_shown_effect, LastVfxShow, RepeatingVfxShow,
};

use crate::commands::{CameraOrbit, LoopEnabled, PendingVfxShows, PreviewFreeze, SpreadOnce};

pub struct VfxPlaygroundPlugin;

impl Plugin for VfxPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PendingVfxShows>()
			.init_resource::<LastVfxShow>()
			.init_resource::<RepeatingVfxShow>()
			.init_resource::<PreviewFreeze>()
			.init_resource::<LoopEnabled>()
			.init_resource::<CameraOrbit>()
			.init_resource::<SpreadOnce>()
			.add_plugins(VfxPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()));
		app.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::release_modifiers_on_focus_change.before(camera::camera_controller),
					camera::camera_controller,
					camera::orbit_camera.after(camera::camera_controller),
					apply_pending_shows,
					repeat_shown_effect.after(apply_pending_shows),
					pause_at_frozen_age.after(repeat_shown_effect),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		brightness: 120.0,
		color: Color::srgb(0.55, 0.62, 0.78),
		..default()
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 3200.0,
			color: Color::srgb(1.0, 0.92, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use game_commands::command::GameCommand;
	use maybraid_vfx::{FIERY_EXPLOSION, FIREBALL, FLASH, SMOKE, SPARKS};

	#[test]
	fn parses_help() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("help")?;
		assert!(matches!(command, PlaygroundCommand::Help));
		Ok(())
	}

	#[test]
	fn parses_show_default() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show")?;
		let PlaygroundCommand::Show { effect, scale, intensity, distance, seed, playback } =
			command
		else {
			return Err("expected show".into());
		};
		assert_eq!(effect, "fiery-explosion");
		assert!((scale - 1.0).abs() < 1e-4);
		assert!((intensity - 1.0).abs() < 1e-4);
		assert!(distance.abs() < 1e-4);
		assert!(seed.is_none());
		assert!((playback - 1.0).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn parses_fiery_explosion_command() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("fiery-explosion")?;
		assert!(matches!(
			command,
			PlaygroundCommand::FieryExplosion { scale, intensity, distance, seed, playback }
				if (scale - 1.0).abs() < 1e-4
					&& (intensity - 1.0).abs() < 1e-4
					&& distance.abs() < 1e-4
					&& seed.is_none()
					&& (playback - 1.0).abs() < 1e-4
		));
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"fiery-explosion --scale 1.5 --intensity 1.2 --distance 6 --seed 3 --playback 0.5",
		)?;
		let PlaygroundCommand::FieryExplosion { scale, intensity, distance, seed, playback } =
			command
		else {
			return Err("expected fiery-explosion".into());
		};
		assert!((scale - 1.5).abs() < 1e-4);
		assert!((intensity - 1.2).abs() < 1e-4);
		assert!((distance - 6.0).abs() < 1e-4);
		assert_eq!(seed, Some(3));
		assert!((playback - 0.5).abs() < 1e-4);
		Ok(())
	}

	#[test]
	fn parses_show_layers_and_overrides() -> Result<(), String> {
		for (line, name) in [
			("show flash", FLASH),
			("show fireball", FIREBALL),
			("show smoke", SMOKE),
			("show sparks", SPARKS),
			("show fiery-explosion", FIERY_EXPLOSION),
		] {
			let command = <PlaygroundCommand as GameCommand>::parse_line(line)?;
			let PlaygroundCommand::Show { effect, .. } = command else {
				return Err(format!("expected show for {line}"));
			};
			assert_eq!(maybraid_vfx::canonicalize_effect_name(&effect), Some(name));
		}
		let command = <PlaygroundCommand as GameCommand>::parse_line(
			"show sparks --scale 2 --intensity 1.5",
		)?;
		let PlaygroundCommand::Show { effect, scale, intensity, .. } = command else {
			return Err("expected show".into());
		};
		assert_eq!(effect, "sparks");
		assert!((scale - 2.0).abs() < 1e-4);
		assert!((intensity - 1.5).abs() < 1e-4);
		Ok(())
	}
}
