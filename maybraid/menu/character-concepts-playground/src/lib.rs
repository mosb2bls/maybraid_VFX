//! Command-driven playground for the Character Concepts Screen.
//!
//! This crate is the first executable surface for the concept plan. Commands are
//! temporary stand-ins for future UI fields; they spawn through
//! `characters` LodScene recipes ([`characters::CharacterRecipe::clothed`]).

mod animation;
mod camera_focus;
mod character_lod;
pub mod commands;
mod diagnostics;
mod focus;
mod focus_reference;
mod material;
mod menu_listeners;
mod preview;
mod preview_color;
mod scale_reference;
mod skinning;
mod species_session;
mod thumbnail;
mod ui;

pub use commands::{ConceptsCommand, CONCEPTS_CLI_NAME};
pub use diagnostics::fps_debug_enabled;
pub use game_commands::command::PendingStartupCommand;

use bevy::prelude::*;
use bevy_character_ui_menu_renderer::CharacterMenuRendererPlugin;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use character_creation_menus::CharacterMenu;
use characters_playground::camera;
use game_commands::command::{capture_command_line_input, GameCommandPlugin};

use camera_focus::{apply_camera_suggestion, PendingCameraFocus};
use character_lod::CharacterLodPlugin;
use characters::CharacterHostSystems;
use focus::animate_focused_preview_asset;
use focus_reference::{sync_focus_reference, FocusReferenceSyncState};
use lod::LodViewer;
use material::apply_preview_colors;
use material::PreviewColorMaterials;
use menu_listeners::{
	dispatch_menu_interactions, init_character_menu_state, on_character_menu_event,
	sync_menu_state_from_config, CharacterMenuState,
};
use preview::{
	preview_pass_ready, reveal_ready_preview, stamp_lod_character_preview, sync_preview,
	tick_preview_respawn_cooldown, ConceptPreviewConfig, ConceptPreviewSyncState,
	PreviewRespawnCooldown, PreviewRevealDebugState,
};
use skinning::{dump_bones_to_console, DumpBonesRequest};
use species_session::{
	ensure_species_camera_focus, persist_species_session, CameraFocusBootState, SpeciesSessionState,
};

pub struct CharacterConceptsPlaygroundPlugin;

impl Plugin for CharacterConceptsPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		if diagnostics::fps_debug_enabled() {
			app.add_plugins(diagnostics::FpsDiagnosticsPlugin);
		}
		app.init_resource::<ConceptPreviewConfig>()
			.init_resource::<ConceptPreviewSyncState>()
			.init_resource::<FocusReferenceSyncState>()
			.init_resource::<PreviewRespawnCooldown>()
			.init_resource::<PreviewRevealDebugState>()
			.init_resource::<DumpBonesRequest>()
			.init_resource::<PendingCameraFocus>()
			.init_resource::<SpeciesSessionState>()
			.init_resource::<CameraFocusBootState>()
			.init_resource::<PreviewColorMaterials>()
			.init_resource::<thumbnail::ThumbnailCache>()
			.init_resource::<ui::CreatorUiState>()
			.init_resource::<ui::CreatorUiSyncState>()
			.init_resource::<CharacterMenuState>()
			.add_plugins(CharacterMenuRendererPlugin::<CharacterMenu>::default())
			.add_plugins(CameraLookPlugin::new(CameraLookConfig {
				enabled_at_start: false,
				..CameraLookConfig::default()
			}))
			.add_plugins(CharacterLodPlugin)
			.add_plugins(
				GameCommandPlugin::<ConceptsCommand>::with_config(ui::ui_config())
					.with_drawer_config(ui::drawer_config()),
			)
			.add_observer(ui::on_creator_ui_scroll)
			.add_systems(
				Startup,
				(
					camera::setup_camera,
					add_lod_viewer_to_camera.after(camera::setup_camera),
					setup_lighting,
					scale_reference::setup_scale_reference,
					init_character_menu_state,
					ui::setup_creator_ui,
				),
			)
			.add_systems(
				Update,
				(
					tick_preview_respawn_cooldown,
					persist_species_session,
					ensure_species_camera_focus.after(persist_species_session),
					sync_menu_state_from_config,
					dispatch_menu_interactions,
					on_character_menu_event.after(dispatch_menu_interactions),
					ui::sync_creator_ui.after(dispatch_menu_interactions),
					ui::refresh_creator_ui_display.after(ui::sync_creator_ui),
					camera::camera_controller,
					ui::send_creator_ui_scroll_events,
					sync_preview
						.after(capture_command_line_input::<ConceptsCommand>)
						.after(on_character_menu_event),
					sync_focus_reference.after(sync_preview).before(CharacterHostSystems::BoneMap),
					stamp_lod_character_preview
						.after(sync_preview)
						.after(CharacterHostSystems::Membership)
						.before(characters::CharacterMotionSystems::Anim),
					animate_focused_preview_asset
						.after(dispatch_menu_interactions)
						.before(sync_preview),
				),
			)
			.add_systems(
				Update,
				(
					reveal_ready_preview
						.after(CharacterHostSystems::Pose)
						.after(CharacterHostSystems::Fulfill)
						.run_if(preview_pass_ready),
					apply_preview_colors
						.after(CharacterHostSystems::Fulfill)
						.run_if(preview_pass_ready),
					dump_bones_to_console,
					thumbnail::sync_thumbnail_camera_activity.after(ui::sync_creator_ui),
					ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
				),
			)
			.add_systems(
				PostUpdate,
				(apply_camera_suggestion
					.after(TransformSystems::Propagate)
					.after(CharacterHostSystems::Pose),),
			);
	}
}

fn add_lod_viewer_to_camera(
	mut commands: Commands,
	cameras: Query<Entity, (With<Camera3d>, Without<LodViewer>)>,
) {
	for entity in &cameras {
		commands.entity(entity).insert(LodViewer);
	}
}

fn setup_lighting(mut commands: Commands) {
	use std::f32::consts::PI;
	commands.spawn((
		DirectionalLight { illuminance: 10000.0, shadow_maps_enabled: true, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -PI / 4.0, PI / 4.0, 0.0)),
	));
	commands.spawn((
		DirectionalLight { illuminance: 500.0, shadow_maps_enabled: false, ..default() },
		Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, PI / 4.0, -PI / 4.0, 0.0)),
	));
}
