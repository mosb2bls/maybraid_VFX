//! Sphere catalog for world [`material_ref::MaterialRef`] recipes.

mod camera;
mod capture;
pub mod commands;
mod ground;
mod preview;
mod shape;
mod shoot;
mod ui;

pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::PreviewConfig;

use bevy::prelude::*;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use ground::setup_ground;
use preview::sync_preview;
use shape::setup_meshes;
use shoot::{fly_shots, tick_shoot, ShootConfig};
use world_materials::WorldMaterialsPlugin;

pub struct WorldMaterialsPlaygroundPlugin;

impl Plugin for WorldMaterialsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>()
			.init_resource::<ShootConfig>()
			.add_plugins(WorldMaterialsPlugin)
			.add_plugins(GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()));
		if let Some(session) = capture::CaptureSession::from_env() {
			app.insert_resource(session).add_systems(Startup, capture::apply_column_preview);
		}
		app.add_systems(
			Startup,
			(camera::setup_camera, setup_lighting, setup_ground, setup_meshes),
		)
		.add_systems(
			Update,
			(
				camera::release_modifiers_on_focus_change.before(camera::camera_controller),
				camera::camera_controller.run_if(not(resource_exists::<capture::CaptureSession>)),
				sync_preview.after(capture_command_line_input::<PlaygroundCommand>),
				tick_shoot.after(sync_preview),
				fly_shots,
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
		);
		if app.world().contains_resource::<capture::CaptureSession>() {
			app.add_systems(Update, capture::tick_capture.after(sync_preview));
		}
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		brightness: 220.0,
		color: Color::srgb(0.72, 0.78, 0.92),
		..default()
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 9800.0,
			color: Color::srgb(1.0, 0.92, 0.82),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
		capture::KeySun,
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 2400.0,
			color: Color::srgb(0.55, 0.68, 1.0),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
		capture::FillSky,
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use clap::Parser;
	use game_commands::command::GameCommand;

	#[test]
	fn parses_show_hex() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("show hex")?;
		let PlaygroundCommand::Show { recipe } = command else {
			return Err("expected show".into());
		};
		assert_eq!(recipe, "hex");
		Ok(())
	}

	#[test]
	fn parses_shoot() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("shoot")?;
		assert!(matches!(command, PlaygroundCommand::Shoot { action: None }));
		Ok(())
	}

	#[test]
	fn parses_shoot_stop() -> Result<(), String> {
		let command = PlaygroundCommand::try_parse_from(["materials", "shoot", "stop"])
			.map_err(|err| err.to_string())?;
		assert!(matches!(
			command,
			PlaygroundCommand::Shoot { action: Some(commands::ShootAction::Stop) }
		));
		Ok(())
	}

	#[test]
	fn parses_mesh_blast() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh blast")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Blast);
		Ok(())
	}

	#[test]
	fn parses_mesh_ring() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh ring")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Ring);
		Ok(())
	}

	#[test]
	fn parses_mesh_cylinder() -> Result<(), String> {
		let command = <PlaygroundCommand as GameCommand>::parse_line("mesh cylinder")?;
		let PlaygroundCommand::Mesh { shape } = command else {
			return Err("expected mesh".into());
		};
		assert_eq!(shape, crate::shape::PreviewShape::Cylinder);
		Ok(())
	}

	#[test]
	fn show_doric_uses_the_column_recipe() -> Result<(), String> {
		let material = commands::material_for_show("doric");
		assert_eq!(material.name, material_ref::MaterialId::named("doric_column"));
		Ok(())
	}
}
