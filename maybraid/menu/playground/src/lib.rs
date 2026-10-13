//! Command-driven playground for Maybraid HUD menu screens.

pub mod character;
pub mod commands;
mod leave_prompt;
mod loading_demo;
mod preview;
mod session;
mod ui;
mod weapon_gallery;

pub use character::{
	request_show_character, CharacterEditBaseline, CharacterMenuState, CharacterScreen,
	CharacterScreenPlugin, RequestShowCharacter,
};
pub use commands::{PlaygroundCommand, PLAYGROUND_CLI_NAME};
pub use game_commands::command::PendingStartupCommand;
pub use leave_prompt::{CharacterLeavePrompt, CharacterModalMode};
pub use preview::{CharacterPreviewLight, CharacterPreviewPlugin, CharacterPreviewRoot};
pub use session::{
	leave_character_editor, save_editing_character, ActiveCharacter, CharacterEditorReturn,
	CharacterSession, CharacterSessionPlugin, EditingCharacter, RequestEditCharacter,
};
pub use weapon_gallery::{request_show_weapons, WeaponGalleryPlugin, WeaponGalleryScreen};

use bevy::prelude::*;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use character_creation_menus::MenuEvent;
use characters::CharacterHostsPlugin;
use characters_playground::camera;
use game_commands::command::{CommandConsoleOutput, GameCommandPlugin};
use game_commands::ui::GameCommandDrawerConfig;
use lod::LodViewer;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use maybraid_input::{MenuNavPad, VirtualPadPlugin};
use maybraid_menu_controller::MenuControllerPlugin;
use menu_components::{
	consume_screen_back, ActiveOverlayKey, MenuBackConsumed, ScreenBackPressed, ShortTextModal,
	TextMenuSystems,
};
use menu_screens::{
	cancel_pending_create, request_show_gallery, request_show_home, request_show_in_game,
	request_show_in_game_settings, CreateCharacterPlugin, GalleryChoice, GalleryScreen,
	HomeMenuChoice, HomeScreen, HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin,
	InGameSettingsChoice, InGameSettingsScreen, LoadingScreenPlugin, LoadingScreenSystems,
	SpinRevealScreen,
};

pub struct MenuPlaygroundPlugin;

impl Plugin for MenuPlaygroundPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(
			GameCommandPlugin::<PlaygroundCommand>::with_config(ui::ui_config())
				.with_drawer_config(GameCommandDrawerConfig {
					open_at_start: false,
					toggle_keys: vec![KeyCode::F1, KeyCode::KeyY],
					..default()
				}),
		)
		.add_plugins(CharacterHostsPlugin)
		.add_plugins(CameraLookPlugin::new(CameraLookConfig {
			enabled_at_start: false,
			toggle_keys: Vec::new(),
			..CameraLookConfig::default()
		}))
		.add_plugins(VirtualPadPlugin::default())
		.add_plugins((
			HomeScreenPlugin,
			InGameScreenPlugin,
			LoadingScreenPlugin,
			CreateCharacterPlugin,
			CharacterSessionPlugin,
			CharacterScreenPlugin,
			CharacterPreviewPlugin,
			WeaponGalleryPlugin,
			MenuControllerPlugin,
		))
		.add_systems(
			Startup,
			(camera::setup_camera, add_lod_viewer_to_camera.after(camera::setup_camera)),
		)
		.add_systems(
			Update,
			(
				camera::camera_controller.run_if(character_screen_closed),
				echo_home_choice,
				echo_in_game_choice,
				echo_in_game_settings_choice,
				echo_character_menu,
				echo_gallery_choice,
				editor_back.after(TextMenuSystems::Navigate),
				loading_demo::run_loading_demo.before(LoadingScreenSystems::Apply),
				ui::sync_command_status_text.before(game_commands::ui::update_debug_ui),
			),
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

fn character_screen_closed(
	screens: Query<
		(),
		Or<(With<CharacterScreen>, With<SpinRevealScreen>, With<HomeScreen>, With<GalleryScreen>)>,
	>,
) -> bool {
	screens.is_empty()
}

fn echo_home_choice(
	mut choices: MessageReader<HomeMenuChoice>,
	mut console: ResMut<CommandConsoleOutput>,
	mut commands: Commands,
) {
	for choice in choices.read() {
		console.0 = format!("home: {}", choice.label());
		if *choice == HomeMenuChoice::Characters {
			request_show_gallery(&mut commands);
		}
	}
}

fn echo_in_game_choice(
	mut choices: MessageReader<InGameMenuChoice>,
	active: Option<Res<ActiveCharacter>>,
	mut edits: MessageWriter<crate::RequestEditCharacter>,
	mut console: ResMut<CommandConsoleOutput>,
	mut commands: Commands,
) {
	for choice in choices.read() {
		console.0 = format!("in-game: {}", choice.label());
		match *choice {
			InGameMenuChoice::Settings => request_show_in_game_settings(&mut commands),
			InGameMenuChoice::Character => {
				if let Some(active) = active.as_ref() {
					edits.write(crate::RequestEditCharacter {
						id: active.id,
						return_to: crate::CharacterEditorReturn::InGame,
						inventory: None,
					});
				}
			}
			_ => {}
		}
	}
}

fn echo_in_game_settings_choice(
	mut choices: MessageReader<InGameSettingsChoice>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for choice in choices.read() {
		console.0 = format!("in-game settings: {}", choice.label());
	}
}

fn echo_character_menu(
	mut events: MessageReader<CharacterMenuEvent<MenuEvent>>,
	menu_state: Res<CharacterMenuState>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for event in events.read() {
		match event {
			CharacterMenuEvent::Menu(_) => {
				console.0 = format!("character: {}", menu_state.0.species.value.label());
			}
			CharacterMenuEvent::CameraFocus(focus) => {
				console.0 = format!("character focus: {focus:?}");
			}
		}
	}
}

fn echo_gallery_choice(
	mut choices: MessageReader<GalleryChoice>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	for choice in choices.read() {
		console.0 = match choice {
			GalleryChoice::New => String::from("gallery: create a character"),
			GalleryChoice::Select(id) => format!("gallery: {}", id.to_hex()),
		};
	}
}

fn editor_back(
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	prompt: Res<CharacterLeavePrompt>,
	mode: Res<CharacterModalMode>,
	consumed: Res<MenuBackConsumed>,
	mut backs: MessageReader<ScreenBackPressed>,
	return_to: Option<Res<crate::CharacterEditorReturn>>,
	character: Query<(), With<CharacterScreen>>,
	spin: Query<(), With<SpinRevealScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
	weapons: Query<(), With<WeaponGalleryScreen>>,
	settings: Query<(), With<InGameSettingsScreen>>,
) {
	if !consume_screen_back(
		&nav,
		&overlay,
		modal.is_open() || prompt.is_open() || (!character.is_empty() && mode.blocks_leave()),
		&consumed,
		&mut backs,
	) {
		return;
	}
	if !settings.is_empty() {
		request_show_in_game(&mut commands);
		return;
	}
	if !character.is_empty() {
		// Leave without writing; [`save_editing_character`] is Save-only.
		leave_character_editor(&mut commands, return_to.as_deref().copied());
		return;
	}
	if !spin.is_empty() {
		cancel_pending_create(&mut commands);
		request_show_gallery(&mut commands);
		return;
	}
	if !gallery.is_empty() {
		request_show_home(&mut commands);
		return;
	}
	if !weapons.is_empty() {
		request_show_home(&mut commands);
	}
}
