//! Command-driven playground for Maybraid HUD menu screens.

pub mod character;
pub mod commands;
mod drop_prompt;
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
pub use drop_prompt::CharacterDropPrompt;
pub use game_commands::command::PendingStartupCommand;
pub use leave_prompt::CharacterLeavePrompt;
pub use preview::{CharacterPreviewLight, CharacterPreviewPlugin, CharacterPreviewRoot};
pub use session::{
	end_character_editor, leave_character_editor, save_editing_character, ActiveCharacter,
	CharacterEditorReturn, CharacterSession, CharacterSessionPlugin, EditingCharacter,
	RequestEditCharacter,
};
pub use weapon_gallery::{request_show_weapons, WeaponGalleryPlugin, WeaponGalleryScreen};

use bevy::prelude::*;
use camera_controls::look::{CameraLookConfig, CameraLookPlugin};
use character_creation_menus::MenuEvent;
use characters::{CharacterHostsPlugin, StandaloneCharacterMaterialRefPlugin};
use characters_playground::camera;
use game_commands::command::{CommandConsoleOutput, GameCommandPlugin};
use game_commands::ui::GameCommandDrawerConfig;
use lod::LodViewer;
use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
use maybraid_input::{MenuNavPad, VirtualPadPlugin};
use maybraid_menu_controller::MenuControllerPlugin;
use menu_components::{consume_screen_back, MenuModalStack, ScreenBackPressed, TextMenuSystems};
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
		.add_plugins(StandaloneCharacterMaterialRefPlugin)
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

/// Back on every screen except the character editor, whose leave prompt owns it.
fn editor_back(
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	stack: Res<MenuModalStack>,
	mut backs: MessageReader<ScreenBackPressed>,
	return_to: Option<Res<CharacterEditorReturn>>,
	character: Query<(), With<CharacterScreen>>,
	spin: Query<(), With<SpinRevealScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
	weapons: Query<(), With<WeaponGalleryScreen>>,
	settings: Query<(), With<InGameSettingsScreen>>,
) {
	if !consume_screen_back(&nav, &stack, &mut backs) {
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

#[cfg(test)]
mod tests {
	use super::editor_back;
	use crate::character::CharacterScreen;
	use crate::leave_prompt::{CharacterLeavePrompt, CharacterLeavePromptPlugin};
	use bevy::input::keyboard::{Key, KeyboardInput, NativeKey};
	use bevy::input::{ButtonState, InputPlugin, InputSystems};
	use bevy::prelude::*;
	use maybraid_input::{
		produce::keyboard::produce_keyboard, MenuNavPad, PadButton, PadInputPolicy, VirtualPad,
		VirtualPadConfig,
	};
	use maybraid_menu_controller::{
		dispatch, CaptureKind, InputRouter, InputSource, MenuController, MenuNavCapture, ScopeOwner,
	};
	use menu_components::{
		HudMenu, HudMenuItem, MenuComponentsPlugin, ShortTextField, ShortTextKey, ShortTextModal,
		TextMenuInputLock, TextMenuSystems,
	};
	use menu_screens::RequestShowGallery;

	/// Physical pad buttons held this frame.
	#[derive(Resource, Default)]
	struct HeldPad(Vec<PadButton>);

	/// Mirrors the menu controller's gate: the GameController backend cannot run
	/// off the main thread, so tests skip `MenuControllerPlugin`.
	fn gate_pad(
		lock: Res<TextMenuInputLock>,
		modal: Res<ShortTextModal>,
		mut router: ResMut<InputRouter>,
		mut menu_capture: ResMut<MenuNavCapture>,
		mut policy: ResMut<PadInputPolicy>,
	) {
		router.replace_owned_capture(
			&mut menu_capture.text,
			InputSource::local_pad(),
			(lock.0 && !modal.is_open()).then_some(ScopeOwner::text_lock()),
			CaptureKind::TextEntry,
		);
		*policy = router.pad_policy();
	}

	fn begin_held_pad(held: Res<HeldPad>, mut pad: ResMut<VirtualPad>) {
		pad.begin_frame();
		for button in &held.0 {
			pad.hold_digital(*button);
		}
	}

	fn finish_held_pad(
		config: Res<VirtualPadConfig>,
		policy: Res<PadInputPolicy>,
		mut pad: ResMut<VirtualPad>,
		mut nav: ResMut<MenuNavPad>,
	) {
		if policy.suppress_gameplay() {
			pad.suppress_gameplay();
		}
		pad.finish_digital();
		nav.derive(&pad, &config, 1.0 / 60.0);
	}

	fn editor_app() -> App {
		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			InputPlugin,
			bevy::window::WindowPlugin { primary_window: None, ..default() },
		))
		.init_asset::<Font>()
		.init_asset::<Image>()
		.init_resource::<HeldPad>()
		.init_resource::<VirtualPad>()
		.init_resource::<VirtualPadConfig>()
		.init_resource::<MenuNavPad>()
		.init_resource::<InputRouter>()
		.init_resource::<MenuNavCapture>()
		.init_resource::<PadInputPolicy>()
		.init_resource::<bevy::picking::hover::HoverMap>()
		.init_resource::<crate::CharacterMenuState>()
		.insert_resource(character_persist::SaveRoot::at(
			std::env::temp_dir().join(format!("maybraid-editor-back-{}", std::process::id())),
		))
		.add_plugins((MenuComponentsPlugin, CharacterLeavePromptPlugin))
		.add_plugins(menu_screens::MenuInputPlugin)
		.add_systems(
			PreUpdate,
			(gate_pad, begin_held_pad, produce_keyboard, finish_held_pad)
				.chain()
				.after(InputSystems),
		)
		.add_systems(
			Update,
			(
				(dispatch::refresh_menu_focus, dispatch::dispatch_menu_nav)
					.chain()
					.in_set(TextMenuSystems::Navigate),
				editor_back.after(TextMenuSystems::Navigate),
			),
		);
		let screen = app.world_mut().spawn((CharacterScreen, MenuController::default())).id();
		let menu = app.world_mut().spawn((HudMenu::new(1), ChildOf(screen))).id();
		app.world_mut().spawn((
			HudMenuItem { index: 0, menu },
			ShortTextKey("name"),
			ShortTextField { value: String::new(), max_len: 16, editing: false },
			ChildOf(menu),
		));
		app.update();
		app
	}

	fn key(app: &mut App, key_code: KeyCode, state: ButtonState) {
		app.world_mut().write_message(KeyboardInput {
			key_code,
			logical_key: Key::Unidentified(NativeKey::Unidentified),
			state,
			text: None,
			repeat: false,
			window: Entity::PLACEHOLDER,
		});
	}

	/// Press for one frame, release, then let a few frames settle.
	fn tap(app: &mut App, button: PadButton, key_code: Option<KeyCode>) {
		app.world_mut().resource_mut::<HeldPad>().0 = vec![button];
		if let Some(key_code) = key_code {
			key(app, key_code, ButtonState::Pressed);
		}
		app.update();
		app.world_mut().resource_mut::<HeldPad>().0.clear();
		if let Some(key_code) = key_code {
			key(app, key_code, ButtonState::Released);
		}
		for _ in 0..4 {
			app.update();
		}
	}

	fn left_editor(app: &mut App) -> bool {
		let gallery = app
			.world_mut()
			.query_filtered::<(), With<RequestShowGallery>>()
			.iter(app.world())
			.next()
			.is_some();
		gallery || app.world().resource::<CharacterLeavePrompt>().is_open()
	}

	fn open_keyboard(app: &mut App) {
		tap(app, PadButton::A, None);
		assert!(app.world().resource::<ShortTextModal>().is_open(), "A opens the name keyboard");
		assert!(!left_editor(app));
	}

	#[test]
	fn pad_b_on_the_name_keyboard_only_closes_the_keyboard() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		tap(&mut app, PadButton::B, None);
		assert!(!app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}

	#[test]
	fn start_on_the_name_keyboard_only_submits() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		tap(&mut app, PadButton::Start, None);
		assert!(!app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}

	#[test]
	fn typing_then_b_on_the_name_keyboard_only_closes_the_keyboard() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		tap(&mut app, PadButton::A, None);
		tap(&mut app, PadButton::A, None);
		assert!(app.world().resource::<ShortTextModal>().is_open(), "A types on the keypad");
		tap(&mut app, PadButton::B, None);
		assert!(!app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}

	/// Real presses span several frames; the pad gate drops while the keyboard
	/// closes and must not turn the still-held button into a second press.
	fn hold(app: &mut App, button: PadButton) {
		app.world_mut().resource_mut::<HeldPad>().0 = vec![button];
		for _ in 0..30 {
			app.update();
		}
		app.world_mut().resource_mut::<HeldPad>().0.clear();
		for _ in 0..4 {
			app.update();
		}
	}

	#[test]
	fn held_b_on_the_name_keyboard_only_closes_the_keyboard() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		hold(&mut app, PadButton::B);
		assert!(!app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}

	#[test]
	fn held_start_on_the_name_keyboard_only_submits() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		hold(&mut app, PadButton::Start);
		assert!(!app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}

	#[test]
	fn key_b_on_the_name_keyboard_does_not_back() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		key(&mut app, KeyCode::KeyB, ButtonState::Pressed);
		app.update();
		key(&mut app, KeyCode::KeyB, ButtonState::Released);
		for _ in 0..4 {
			app.update();
		}
		assert!(app.world().resource::<ShortTextModal>().is_open(), "B types; Escape backs");
		assert!(!left_editor(&mut app));
	}

	#[test]
	fn wasd_on_the_name_keyboard_does_not_steer_the_pad() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		for key_code in [KeyCode::KeyW, KeyCode::KeyA, KeyCode::KeyS, KeyCode::KeyD] {
			key(&mut app, key_code, ButtonState::Pressed);
		}
		app.update();
		let pad = app.world().resource::<VirtualPad>();
		assert_eq!(pad.move_stick, Vec2::ZERO);
		assert_eq!(pad.dpad, Vec2::ZERO);
		assert!(app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}

	#[test]
	fn escape_on_the_name_keyboard_only_closes_the_keyboard() {
		let mut app = editor_app();
		open_keyboard(&mut app);
		tap(&mut app, PadButton::B, Some(KeyCode::Escape));
		assert!(!app.world().resource::<ShortTextModal>().is_open());
		assert!(!left_editor(&mut app));
	}
}
