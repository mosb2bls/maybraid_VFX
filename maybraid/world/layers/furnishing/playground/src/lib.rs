//! Isolated `/show` catalog for painted furniture assemblies in Richmond slots.

pub mod camera;
pub mod commands;
mod gallery;
mod ground;
mod material_lib;
mod preview;
mod ui;

pub use camera::CameraController;
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use preview::{PreviewConfig, PreviewSubject};

use bevy::prelude::*;
use building_components::FurnitureWireframePlugin;
use furniture_assemblies::FurnitureAssembliesPlugin;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};
use ground::setup_ground;
use lod_lazy_refs::LodLazyRefsPlugin;
use material_lib::PlaygroundMaterialRefPlugin;
use preview::present_preview;
use scene_ref::SceneRefPlugin;

pub struct FurniturePlaygroundPlugin;

impl Plugin for FurniturePlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PreviewConfig>().add_plugins((
			SceneRefPlugin,
			LodLazyRefsPlugin,
			PlaygroundMaterialRefPlugin,
			FurnitureWireframePlugin,
			FurnitureAssembliesPlugin,
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config()),
		));
		app.add_systems(Startup, (camera::setup_camera, setup_lighting, setup_ground))
			.add_systems(
				Update,
				(
					camera::release_modifiers_on_focus_change.before(camera::camera_controller),
					camera::camera_controller,
					present_preview.after(capture_command_line_input::<PlaygroundCommand>),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.insert_resource(GlobalAmbientLight {
		brightness: 560.0,
		color: Color::srgb(1.0, 0.90, 0.76),
		..default()
	});
	commands.spawn((
		DirectionalLight {
			illuminance: 12800.0,
			color: Color::srgb(1.0, 0.88, 0.70),
			shadow_maps_enabled: true,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 3.0, PI / 5.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight {
			illuminance: 4200.0,
			color: Color::srgb(1.0, 0.72, 0.48),
			shadow_maps_enabled: false,
			..default()
		},
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 5.0, -PI / 4.0, 0.0)),
	));
}

#[cfg(test)]
mod tests {
	use super::*;
	use clap::Parser;

	#[test]
	fn parses_show_gallery() -> anyhow::Result<()> {
		let cmd = PlaygroundCommand::try_parse_from(["furniture", "show", "gallery"])
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		assert!(matches!(cmd, PlaygroundCommand::Show(commands::Show::Gallery(_))));
		Ok(())
	}

	#[test]
	fn parses_show_bed_seed() -> anyhow::Result<()> {
		let cmd = PlaygroundCommand::try_parse_from(["furniture", "show", "bed", "--seed", "9"])
			.map_err(|err| anyhow::anyhow!("{err}"))?;
		match cmd {
			PlaygroundCommand::Show(commands::Show::Bed(bed)) => {
				if bed.seed != 9 {
					return Err(anyhow::anyhow!("expected seed 9"));
				}
			}
			_ => return Err(anyhow::anyhow!("expected show bed")),
		}
		Ok(())
	}
}
