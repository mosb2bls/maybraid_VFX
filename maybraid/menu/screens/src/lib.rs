//! Maybraid menu screen plugins.
//!
//! Screens compose [`menu_components`] widgets. The playground is the first
//! consumer; later the game app will add the same plugins.

use bevy::prelude::*;

pub mod create_character;
pub mod gallery;
pub mod game_mode;
pub mod home;
pub mod in_game;
pub mod input;
pub mod loading;
pub mod settings;
pub mod show;
pub mod spin_reveal;

pub use create_character::{
	cancel_pending_create, request_show_create_character, request_show_create_character_id,
	CreateCharacterPlugin, CreateCharacterReady, RequestShowCreateCharacter,
};
pub use gallery::{
	request_show_gallery, GalleryChoice, GalleryScreen, GalleryScreenPlugin, RequestShowGallery,
};
pub use game_mode::GameMode;
pub use home::{request_show_home, HomeMenuChoice, HomeScreen, HomeScreenPlugin, RequestShowHome};
pub use in_game::{
	request_show_in_game, request_show_in_game_at, request_show_in_game_with_mode,
	InGameMenuChoice, InGameScreen, InGameScreenPlugin, RequestShowInGame,
};
pub use input::{add_menu_input, MenuInputPlugin};
pub use loading::{
	request_loading_explainer, request_loading_progress, request_show_loading,
	LoadingExplainerText, LoadingProgress, LoadingScreen, LoadingScreenPlugin,
	LoadingScreenSystems, RequestShowLoading,
};
pub use settings::{
	apply_seed, apply_shadows_env, apply_unsaved_visual_style_default, classify_host,
	load_machine_settings, load_machine_settings_ex, parse_render_scale_env, parse_shadows_env,
	request_show_in_game_settings, save_machine_settings, seed_device_class, DeviceClass,
	DeviceType, HostProbe, InGameRenderScale, InGameSettings, InGameSettingsChoice,
	InGameSettingsPlugin, InGameSettingsScreen, InGameSettingsSystems, InGameShadowQuality,
	InGameVisualStyle, PersistPolicy, Preference, QualitySeeded, RenderScaleReadout,
	RequestShowInGameSettings, VisualStyleAutoPick, VisualStyleSaved, ENV_RENDER_SCALE,
	ENV_SHADOWS,
};
pub use show::{despawn_menu_screens, take_menu_show_request};
pub use spin_reveal::{
	cancel_spin_reveal, request_show_spin_reveal, RequestShowSpinReveal, SpinRevealChoice,
	SpinRevealCurrent, SpinRevealFinished, SpinRevealItems, SpinRevealScreen,
	SpinRevealScreenPlugin, SpinRevealSystems,
};

/// Marker on every full-screen menu root so show-requests can replace each other.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuScreen;
