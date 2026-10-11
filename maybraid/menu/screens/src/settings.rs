//! In-game settings: pause-menu overlay for debug and user toggles.

mod catalog;
mod persist;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use character_persist::SaveRoot;
use maybraid_menu_controller::MenuController;
use menu_components::info::description::{set_description_for_menu, TextMenuDescription};
use menu_components::single_select::republish_menu_activate;
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::{
	screen_back_scene, set_brand_mode_title, BrandModeLine, BrandModeTitle, MenuFocus,
	TextColumnAlign, TextColumnAnchor, TextCursorRow, TextMenuPlugin,
};
use serde::{Deserialize, Serialize};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::{GameMode, MenuScreen};

pub use catalog::{
	classify_host, seed_device_class, DeviceClass, DeviceType, HostProbe, PersistPolicy, Preference,
};
pub use persist::{
	apply_seed, apply_shadows_env, load_machine_settings, parse_render_scale_env,
	parse_shadows_env, save_machine_settings, QualitySeeded, ENV_RENDER_SCALE, ENV_SHADOWS,
};

/// Queue an in-game settings spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct RequestShowInGameSettings;

/// Marker on the spawned in-game settings root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InGameSettingsScreen;

/// Sun cascade quality. The game copies this onto the sky sun
/// (`maybraid_sky::ShadowQuality`) without a sky dependency here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InGameShadowQuality {
	#[default]
	Off,
	Low,
	High,
}

impl InGameShadowQuality {
	pub fn cycle(self) -> Self {
		match self {
			Self::High => Self::Low,
			Self::Low => Self::Off,
			Self::Off => Self::High,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::High => "High",
			Self::Low => "Low",
			Self::Off => "Off",
		}
	}
}

/// 3D shading scale. `Auto` is `1 / window.scale_factor()` (logical on Retina).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum InGameRenderScale {
	#[default]
	#[serde(rename = "auto")]
	Auto,
	#[serde(rename = "100")]
	Native,
	#[serde(rename = "75")]
	ThreeQuarter,
	#[serde(rename = "50")]
	Half,
}

impl InGameRenderScale {
	pub const ALL: [Self; 4] = [Self::Auto, Self::Native, Self::ThreeQuarter, Self::Half];

	pub fn cycle(self) -> Self {
		match self {
			Self::Auto => Self::Native,
			Self::Native => Self::ThreeQuarter,
			Self::ThreeQuarter => Self::Half,
			Self::Half => Self::Auto,
		}
	}

	pub fn label(self) -> &'static str {
		match self {
			Self::Auto => "Auto",
			Self::Native => "100%",
			Self::ThreeQuarter => "75%",
			Self::Half => "50%",
		}
	}

	/// Explicit factor, or `None` for the Retina / logical default.
	pub fn requested(self) -> Option<f32> {
		match self {
			Self::Auto => None,
			Self::Native => Some(1.0),
			Self::ThreeQuarter => Some(0.75),
			Self::Half => Some(0.5),
		}
	}

	/// Live 3D request. `MAYBRAID_RENDER_SCALE` wins when set.
	pub fn live_request(self, env: Option<f32>) -> Option<f32> {
		env.or(self.requested())
	}

	/// Snap an explicit factor onto the nearest Settings step.
	pub fn from_factor(value: f32) -> Self {
		if !value.is_finite() {
			return Self::Auto;
		}
		let value = value.clamp(0.5, 1.0);
		if value >= 0.875 {
			Self::Native
		} else if value >= 0.625 {
			Self::ThreeQuarter
		} else {
			Self::Half
		}
	}

	pub fn subtext(self, readout: RenderScaleReadout) -> String {
		format_render_scale_subtext(self, readout)
	}
}

/// Live 3D scale + window sizes for the Settings row. The game fills this.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct RenderScaleReadout {
	pub effective: f32,
	pub window_physical: UVec2,
	pub target_physical: UVec2,
	pub env_override: Option<f32>,
}

impl Default for RenderScaleReadout {
	fn default() -> Self {
		Self {
			effective: 1.0,
			window_physical: UVec2::ZERO,
			target_physical: UVec2::ZERO,
			env_override: None,
		}
	}
}

fn format_render_scale_subtext(
	preference: InGameRenderScale,
	readout: RenderScaleReadout,
) -> String {
	let mut label = preference.label().to_string();
	if preference == InGameRenderScale::Auto
		&& readout.window_physical.x > 0
		&& readout.env_override.is_none()
		&& (readout.effective - 1.0).abs() > 1e-3
	{
		label = format!("Auto ({:.0}%)", readout.effective * 100.0);
	}
	let body = resolution_subtext(&label, readout);
	match readout.env_override {
		Some(env) => format!("{body} · env {:.0}%", env * 100.0),
		None => body,
	}
}

fn resolution_subtext(label: &str, readout: RenderScaleReadout) -> String {
	if readout.window_physical.x == 0 || readout.window_physical.y == 0 {
		return label.to_string();
	}
	let target = if readout.target_physical.x == 0 {
		readout.window_physical
	} else {
		readout.target_physical
	};
	if target == readout.window_physical {
		format!("{label} · {}×{}", target.x, target.y)
	} else {
		format!(
			"{label} · {}×{} → window {}×{}",
			target.x, target.y, readout.window_physical.x, readout.window_physical.y
		)
	}
}

/// Live pause-menu bag. Quality the player owns ([`Self::shadows`]) sits next to
/// session chrome ([`Self::mob_hud`]). Persistence is a catalog projection, not
/// this resource as-is. The game copies shadows onto the sky sun and HUD onto
/// the world overlay.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct InGameSettings {
	pub mob_hud: bool,
	pub shadows: InGameShadowQuality,
	pub render_scale: InGameRenderScale,
}

impl Default for InGameSettings {
	fn default() -> Self {
		Self::from_device(seed_device_class())
	}
}

impl InGameSettings {
	pub fn mob_hud_state_label(self) -> &'static str {
		if self.mob_hud {
			"On"
		} else {
			"Off"
		}
	}

	pub fn state_label(self, choice: InGameSettingsChoice, readout: RenderScaleReadout) -> String {
		match choice {
			InGameSettingsChoice::Shadows => self.shadows.label().to_string(),
			InGameSettingsChoice::RenderScale => self.render_scale.subtext(readout),
			InGameSettingsChoice::MobHud => self.mob_hud_state_label().to_string(),
		}
	}
}

/// Settings plugin stages so the game can apply live 3D scale before the screen respawns.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InGameSettingsSystems {
	ApplyChoice,
	Show,
}

/// Settings rows. Activate toggles or cycles the matching [`InGameSettings`] field.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum InGameSettingsChoice {
	#[default]
	Shadows,
	RenderScale,
	MobHud,
}

impl InGameSettingsChoice {
	pub const ALL: [Self; 3] = [Self::Shadows, Self::RenderScale, Self::MobHud];

	pub fn label(self) -> &'static str {
		match self {
			Self::Shadows => "Shadows",
			Self::RenderScale => "Render Scale",
			Self::MobHud => "Mob HUD",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Shadows => {
				"Sun cascade shadows. High is four maps to 150 m. Low is four maps to 60 m. Off disables the sun's shadow maps."
			}
			Self::RenderScale => {
				"3D shading scale. Auto matches logical resolution on Retina (1 / scale factor). 100% shades every physical pixel. 75% and 50% upscale a smaller 3D image; UI stays native. MAYBRAID_RENDER_SCALE overrides the live scale when set."
			}
			Self::MobHud => {
				"Pins and colored poles on presented mob hosts. Use this to find where groups should stand."
			}
		}
	}
}

impl InGameSettingsScreen {
	pub fn scene(
		mode: &GameMode,
		settings: InGameSettings,
		readout: RenderScaleReadout,
	) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::rows(
					"Settings",
					InGameSettingsChoice::ALL.into_iter().map(|choice| {
						TextCursorRow::new(choice.label(), choice)
							.with_subtext(settings.state_label(choice, readout))
					}),
				)
				.anchored(TextColumnAnchor::Center)
				.aligned(TextColumnAlign::Center)
				.with_description(InGameSettingsChoice::Shadows.description())
				.scene(),
			),
			Box::new(BrandModeLine::new(mode.label.clone()).scene()),
			Box::new(screen_back_scene()),
		];
		bsn! {
			InGameSettingsScreen
			MenuScreen
			MenuController
			Node {
				width: percent(100),
				height: percent(100),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
			}
			Pickable::IGNORE
			on(sync_settings_description)
			on(republish_menu_activate::<InGameSettingsChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_in_game_settings(commands: &mut Commands) {
	commands.spawn(RequestShowInGameSettings);
}

pub struct InGameSettingsPlugin;

impl Plugin for InGameSettingsPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.init_resource::<GameMode>()
			.init_resource::<InGameSettings>()
			.init_resource::<RenderScaleReadout>()
			.init_resource::<persist::MachineSettings>()
			.add_plugins(TextMenuPlugin::<InGameSettingsChoice>::default())
			.add_systems(Startup, persist::seed_in_game_settings)
			.add_systems(Last, persist::persist_machine_settings_on_exit)
			.add_systems(
				Update,
				apply_in_game_settings_choice.in_set(InGameSettingsSystems::ApplyChoice),
			)
			.add_systems(
				Update,
				(apply_show_in_game_settings, sync_in_game_settings_brand)
					.chain()
					.in_set(InGameSettingsSystems::Show)
					.after(InGameSettingsSystems::ApplyChoice),
			);
	}
}

fn apply_in_game_settings_choice(
	mut choices: MessageReader<InGameSettingsChoice>,
	mut settings: ResMut<InGameSettings>,
	mut machine: ResMut<persist::MachineSettings>,
	save_root: Option<Res<SaveRoot>>,
	mut commands: Commands,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match choice {
		InGameSettingsChoice::Shadows => {
			settings.shadows = settings.shadows.cycle();
			persist::write_machine_preference::<catalog::Shadows>(
				Some(&mut machine),
				settings.shadows,
				save_root.as_deref(),
			);
		}
		InGameSettingsChoice::RenderScale => {
			settings.render_scale = settings.render_scale.cycle();
			persist::write_machine_preference::<catalog::RenderScalePref>(
				Some(&mut machine),
				settings.render_scale,
				save_root.as_deref(),
			);
		}
		InGameSettingsChoice::MobHud => settings.mob_hud = !settings.mob_hud,
	}
	request_show_in_game_settings(&mut commands);
}

fn apply_show_in_game_settings(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowInGameSettings>>,
	existing: Query<Entity, With<MenuScreen>>,
	mode: Res<GameMode>,
	settings: Res<InGameSettings>,
	readout: Res<RenderScaleReadout>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	commands.spawn_scene(InGameSettingsScreen::scene(&mode, *settings, *readout));
}

fn sync_in_game_settings_brand(
	mode: Res<GameMode>,
	screens: Query<Entity, With<InGameSettingsScreen>>,
	children: Query<&Children>,
	mut titles: Query<&mut Text, With<BrandModeTitle>>,
) {
	if !mode.is_changed() {
		return;
	}
	let title = mode.title();
	for screen in &screens {
		set_brand_mode_title(screen, title.clone(), &children, &mut titles);
	}
}

fn sync_settings_description(
	focus: On<MenuFocus<InGameSettingsChoice>>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<TextMenuDescription>>,
) {
	set_description_for_menu(
		focus.event().entity,
		focus.event().choice.description(),
		&children,
		&mut lines,
	);
}

#[cfg(test)]
mod tests {
	use bevy::prelude::UVec2;

	use super::{
		format_render_scale_subtext, InGameRenderScale, InGameSettings, InGameSettingsChoice,
		InGameShadowQuality, RenderScaleReadout,
	};

	#[test]
	fn labels_and_descriptions_are_nonempty() {
		for choice in InGameSettingsChoice::ALL {
			assert!(!choice.label().is_empty());
			assert!(!choice.description().is_empty());
		}
	}

	#[test]
	fn mob_hud_starts_off() {
		assert!(!InGameSettings::default().mob_hud);
		assert_eq!(
			InGameSettings {
				mob_hud: false,
				shadows: InGameShadowQuality::High,
				render_scale: InGameRenderScale::Auto,
			}
			.mob_hud_state_label(),
			"Off"
		);
		assert_eq!(
			InGameSettings {
				mob_hud: true,
				shadows: InGameShadowQuality::High,
				render_scale: InGameRenderScale::Auto,
			}
			.mob_hud_state_label(),
			"On"
		);
	}

	#[test]
	fn shadows_start_off_and_cycle() {
		assert_eq!(InGameSettings::default().shadows, InGameShadowQuality::Off);
		assert_eq!(InGameShadowQuality::High.cycle(), InGameShadowQuality::Low);
		assert_eq!(InGameShadowQuality::Low.cycle(), InGameShadowQuality::Off);
		assert_eq!(InGameShadowQuality::Off.cycle(), InGameShadowQuality::High);
		assert_eq!(
			InGameSettings::default()
				.state_label(InGameSettingsChoice::Shadows, RenderScaleReadout::default()),
			"Off"
		);
	}

	#[test]
	fn render_scale_starts_auto_and_cycles() -> anyhow::Result<()> {
		assert_eq!(InGameSettings::default().render_scale, InGameRenderScale::Auto);
		assert_eq!(InGameRenderScale::Auto.cycle(), InGameRenderScale::Native);
		assert_eq!(InGameRenderScale::Native.cycle(), InGameRenderScale::ThreeQuarter);
		assert_eq!(InGameRenderScale::ThreeQuarter.cycle(), InGameRenderScale::Half);
		assert_eq!(InGameRenderScale::Half.cycle(), InGameRenderScale::Auto);
		assert_eq!(InGameRenderScale::from_factor(1.0), InGameRenderScale::Native);
		assert_eq!(InGameRenderScale::from_factor(0.75), InGameRenderScale::ThreeQuarter);
		assert_eq!(InGameRenderScale::from_factor(0.5), InGameRenderScale::Half);
		assert_eq!(InGameRenderScale::from_factor(0.6), InGameRenderScale::Half);
		assert_eq!(InGameRenderScale::Auto.requested(), None);
		assert_eq!(InGameRenderScale::ThreeQuarter.requested(), Some(0.75));
		assert_eq!(InGameRenderScale::Auto.live_request(None), None);
		assert_eq!(InGameRenderScale::Auto.live_request(Some(0.75)), Some(0.75));
		assert_eq!(InGameRenderScale::Half.live_request(Some(1.0)), Some(1.0));
		Ok(())
	}

	#[test]
	fn render_scale_subtext_shows_resolution_and_env() -> anyhow::Result<()> {
		let retina = RenderScaleReadout {
			effective: 0.5,
			window_physical: UVec2::new(3024, 1964),
			target_physical: UVec2::new(1512, 982),
			env_override: None,
		};
		assert_eq!(
			format_render_scale_subtext(InGameRenderScale::Auto, retina),
			"Auto (50%) · 1512×982 → window 3024×1964"
		);
		assert_eq!(
			format_render_scale_subtext(InGameRenderScale::Half, retina),
			"50% · 1512×982 → window 3024×1964"
		);
		let native = RenderScaleReadout {
			effective: 1.0,
			window_physical: UVec2::new(1280, 720),
			target_physical: UVec2::new(1280, 720),
			env_override: None,
		};
		assert_eq!(
			format_render_scale_subtext(InGameRenderScale::Native, native),
			"100% · 1280×720"
		);
		let env = RenderScaleReadout { env_override: Some(0.75), ..retina };
		assert_eq!(
			format_render_scale_subtext(InGameRenderScale::Auto, env),
			"Auto · 1512×982 → window 3024×1964 · env 75%"
		);
		assert_eq!(
			format_render_scale_subtext(InGameRenderScale::Auto, RenderScaleReadout::default()),
			"Auto"
		);
		Ok(())
	}
}
