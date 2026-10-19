//! In-game settings: pause-menu overlay for debug and user toggles.

mod catalog;
mod persist;

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use character_persist::SaveRoot;
use maybraid_menu_controller::MenuController;
use maybraid_world::render_scale::{VisualStyle, VisualStyleAutoReason, VisualStyleIdentity};
use menu_components::single_select::republish_menu_activate;
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::{
	menu_list_panel_node, menu_region_wash, screen_back_scene, set_brand_mode_title,
	set_info_box_for_menu, BrandModeLine, BrandModeTitle, MenuFocus, MenuInfoBox, MenuInfoBoxLabel,
	MenuStep, TextColumnAlign, TextColumnAnchor, TextCursorRow, TextMenuPlugin, TextMenuSystems,
};
use serde::{Deserialize, Serialize};

use crate::input::add_menu_input;
use crate::show::{add_menu_show_systems, MenuScreens, MenuShowRequest};
use crate::{GameMode, MenuScreen};

pub use catalog::{
	classify_host, seed_device_class, DeviceClass, DeviceType, HostProbe, PersistPolicy, Preference,
};
pub use persist::{
	apply_seed, apply_shadows_env, load_machine_settings, load_machine_settings_ex,
	parse_render_scale_env, parse_shadows_env, save_machine_settings, LoadedMachineSettings,
	QualitySeeded, ENV_RENDER_SCALE, ENV_SHADOWS,
};

/// Queue an in-game settings spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Default, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowInGameSettings {
	pub selected: Option<InGameSettingsChoice>,
}

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

/// Visual style presentation mode. Machine-scoped.
///
/// Persist values are the shared [`VisualStyleIdentity::script_id`] names
/// (`native`, `sharp`, …). Settings files at version 1 stored today's
/// direct-window path as `sharp`; load remaps that string to [`Self::Native`]
/// so existing players keep the same look. Version 2 writes `native` / `sharp`
/// with the new meanings.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InGameVisualStyle(VisualStyle);

#[allow(non_upper_case_globals)]
impl InGameVisualStyle {
	pub const Native: Self = Self(VisualStyle::Native);
	pub const Sharp: Self = Self(VisualStyle::Sharp);
	pub const Impressionist: Self = Self(VisualStyle::Impressionist);
	pub const Watercolor: Self = Self(VisualStyle::Watercolor);
	pub const Retro: Self = Self(VisualStyle::Retro);
	pub const Antediluvian: Self = Self(VisualStyle::Antediluvian);
	pub const PrePostModern: Self = Self(VisualStyle::PrePostModern);

	pub const ALL: [Self; 7] = [
		Self(VisualStyle::ALL[0]),
		Self(VisualStyle::ALL[1]),
		Self(VisualStyle::ALL[2]),
		Self(VisualStyle::ALL[3]),
		Self(VisualStyle::ALL[4]),
		Self(VisualStyle::ALL[5]),
		Self(VisualStyle::ALL[6]),
	];

	pub fn step(self, delta: i32) -> Self {
		let index = Self::ALL.iter().position(|&value| value == self).unwrap_or(0) as i32;
		let n = Self::ALL.len() as i32;
		Self::ALL[(index + delta).rem_euclid(n) as usize]
	}

	pub fn cycle(self) -> Self {
		self.step(1)
	}

	pub fn identity(self) -> VisualStyleIdentity {
		self.0.identity()
	}

	pub fn label(self) -> &'static str {
		self.identity().label
	}

	pub fn to_visual_style(self) -> VisualStyle {
		self.0
	}

	pub fn from_visual_style(style: VisualStyle) -> Self {
		Self(style)
	}

	pub fn subtext(self) -> String {
		self.label().to_string()
	}

	/// Migrate a legacy InGameRenderScale value to InGameVisualStyle.
	pub fn from_legacy_render_scale(scale: InGameRenderScale) -> Self {
		match scale {
			InGameRenderScale::Auto | InGameRenderScale::Native => Self::Native,
			InGameRenderScale::Half => Self::Impressionist,
			InGameRenderScale::ThreeQuarter => Self::PrePostModern,
		}
	}
}

impl Serialize for InGameVisualStyle {
	fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
		serializer.serialize_str(self.identity().script_id)
	}
}

impl<'de> Deserialize<'de> for InGameVisualStyle {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let raw = String::deserialize(deserializer)?;
		VisualStyle::from_script_id(&raw).map(Self).ok_or_else(|| {
			serde::de::Error::unknown_variant(&raw, &VisualStyleIdentity::SCRIPT_IDS)
		})
	}
}

/// LEGACY: 3D shading scale. Preserved for backward compatibility during migration.
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

	pub fn step(self, delta: i32) -> Self {
		let index = Self::ALL.iter().position(|&value| value == self).unwrap_or(0) as i32;
		let n = Self::ALL.len() as i32;
		Self::ALL[(index + delta).rem_euclid(n) as usize]
	}

	pub fn cycle(self) -> Self {
		self.step(1)
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

	pub fn subtext(self) -> String {
		self.label().to_string()
	}
}

/// How the live visual-style preference was chosen.
///
/// One settings-owned value replaces the old saved-bool plus auto-pick
/// resource. Persistence writes `visual_style` only for [`Self::Explicit`].
/// The settings readout derives its auto-pick footnote from
/// [`Self::AutoPicked`]. Live environment overlays stay off this enum.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum VisualStyleProvenance {
	/// First run, or a loaded file with no explicit visual-style choice.
	#[default]
	PendingAuto,
	/// Host heuristic ran this session. Not written to `settings.json`.
	AutoPicked(VisualStyleAutoReason),
	/// Player (or an already-saved file) chose the style.
	Explicit,
}

impl VisualStyleProvenance {
	/// True when `settings.json` should keep a `visual_style` key.
	pub fn is_saved(self) -> bool {
		matches!(self, Self::Explicit)
	}

	pub fn is_pending(self) -> bool {
		matches!(self, Self::PendingAuto)
	}

	pub fn auto_reason(self) -> Option<VisualStyleAutoReason> {
		match self {
			Self::AutoPicked(reason) => Some(reason),
			Self::PendingAuto | Self::Explicit => None,
		}
	}

	pub fn mark_explicit(&mut self) {
		*self = Self::Explicit;
	}

	/// Load-time mapping: an explicit `visual_style` (or non-`auto` legacy
	/// `render_scale`) is [`Self::Explicit`]. Missing keys stay pending.
	pub fn from_machine_fields(
		fields: &std::collections::BTreeMap<String, serde_json::Value>,
	) -> Self {
		if catalog::visual_style_choice_is_saved(fields) {
			Self::Explicit
		} else {
			Self::PendingAuto
		}
	}

	/// Host heuristic for first-run. No-op unless [`Self::PendingAuto`].
	/// On success, becomes [`Self::AutoPicked`] and writes the live style.
	pub fn apply_unsaved_default(
		&mut self,
		settings: &mut InGameSettings,
		signals: &maybraid_world::VisualStyleSignals,
	) -> Option<maybraid_world::VisualStylePick> {
		if !self.is_pending() {
			return None;
		}
		let pick = maybraid_world::pick_default_visual_style(signals);
		settings.visual_style = InGameVisualStyle::from_visual_style(pick.style);
		*self = Self::AutoPicked(pick.reason);
		Some(pick)
	}
}

/// Live 3D scale + window sizes for the Settings row. The game fills this.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct RenderScaleReadout {
	pub style: VisualStyle,
	pub effective_scale: f32,
	pub window_physical: UVec2,
	pub target_physical: UVec2,
	pub env_override: Option<VisualStyle>,
	pub auto_reason: Option<VisualStyleAutoReason>,
}

impl Default for RenderScaleReadout {
	fn default() -> Self {
		Self {
			style: VisualStyle::Native,
			effective_scale: 1.0,
			window_physical: UVec2::ZERO,
			target_physical: UVec2::ZERO,
			env_override: None,
			auto_reason: None,
		}
	}
}

fn format_visual_style_info(readout: RenderScaleReadout) -> String {
	let target = if readout.target_physical.x == 0 {
		readout.window_physical
	} else {
		readout.target_physical
	};
	if target.x == 0 || target.y == 0 {
		return String::from("Rendered at display size.");
	}
	let pixels = if target == readout.window_physical {
		format!("Rendered at {} x {}.", target.x, target.y)
	} else {
		format!("Rendered at {} x {}. Upscaled to display.", target.x, target.y)
	};
	if readout.env_override.is_some() {
		return pixels;
	}
	if let Some(reason) = readout.auto_reason {
		format!("{pixels} Auto-picked for this machine ({}).", reason.as_str())
	} else {
		pixels
	}
}

fn settings_info_copy(choice: InGameSettingsChoice, readout: RenderScaleReadout) -> String {
	match choice {
		InGameSettingsChoice::VisualStyle => format_visual_style_info(readout),
		other => other.description().to_string(),
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
	pub visual_style: InGameVisualStyle,
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

	pub fn state_label(self, choice: InGameSettingsChoice) -> String {
		match choice {
			InGameSettingsChoice::Shadows => self.shadows.label().to_string(),
			InGameSettingsChoice::VisualStyle => self.visual_style.subtext(),
			InGameSettingsChoice::MobHud => self.mob_hud_state_label().to_string(),
		}
	}
}

/// Settings plugin stages so the game can apply live 3D scale before the screen respawns.
///
/// [`Self::ApplyChoice`] is after [`TextMenuSystems::Navigate`] so left/right
/// [`MenuStep`] messages land in the same frame as the live 3D copy.
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InGameSettingsSystems {
	ApplyChoice,
	Show,
}

#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
struct SettingsStep {
	choice: InGameSettingsChoice,
	delta: i32,
}

/// Settings rows. Activate toggles or cycles the matching [`InGameSettings`] field.
/// Visual Style also steps on left/right.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum InGameSettingsChoice {
	#[default]
	Shadows,
	VisualStyle,
	MobHud,
}

impl InGameSettingsChoice {
	pub const ALL: [Self; 3] = [Self::Shadows, Self::VisualStyle, Self::MobHud];

	pub fn label(self) -> &'static str {
		match self {
			Self::Shadows => "Shadows",
			Self::VisualStyle => "Visual Style",
			Self::MobHud => "Mob HUD",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Shadows => {
				"Sun cascade shadows. High is four maps to 150 m. Low is four maps to 60 m. Off disables the sun's shadow maps."
			}
			Self::VisualStyle => {
				"Presentation mode: native (window) or sharp (1080p offscreen, height-fit). First run auto-picks Sharp on weaker machines. Stylized modes are opt-in: impressionist / watercolor (painterly), retro / antediluvian (nearest-neighbor), or pre-post-modern (color-print)."
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
		selected: InGameSettingsChoice,
	) -> impl Scene + 'static {
		let selected_index = InGameSettingsChoice::ALL
			.iter()
			.position(|&choice| choice == selected)
			.unwrap_or(0);
		let column: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::rows(
					"Settings",
					InGameSettingsChoice::ALL.into_iter().map(|choice| {
						let row = TextCursorRow::new(choice.label(), choice)
							.with_subtext(settings.state_label(choice));
						if choice == InGameSettingsChoice::VisualStyle {
							row.stepper()
						} else {
							row
						}
					}),
				)
				.anchored(TextColumnAnchor::Center)
				.aligned(TextColumnAlign::Center)
				.with_selected(selected_index)
				.scene(),
			),
			Box::new(MenuInfoBox::scene(settings_info_copy(selected, readout))),
		];
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(bsn! {
				template_value(menu_list_panel_node(AlignItems::Center))
				BackgroundColor(menu_region_wash())
				Pickable::IGNORE
				Children [ {column} ]
			}),
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
			on(queue_settings_step)
			Children [ {children} ]
		}
	}
}

pub fn request_show_in_game_settings(commands: &mut Commands) {
	commands.spawn(RequestShowInGameSettings { selected: None });
}

fn request_show_in_game_settings_at(commands: &mut Commands, selected: InGameSettingsChoice) {
	commands.spawn(RequestShowInGameSettings { selected: Some(selected) });
}

pub struct InGameSettingsPlugin;

impl Plugin for InGameSettingsPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.init_resource::<GameMode>()
			.init_resource::<InGameSettings>()
			.init_resource::<RenderScaleReadout>()
			.init_resource::<VisualStyleProvenance>()
			.init_resource::<persist::MachineSettings>()
			.add_message::<SettingsStep>()
			.add_plugins(TextMenuPlugin::<InGameSettingsChoice>::default())
			.configure_sets(
				Update,
				InGameSettingsSystems::ApplyChoice.after(TextMenuSystems::Navigate),
			)
			.add_systems(Startup, persist::seed_in_game_settings)
			.add_systems(Last, persist::persist_machine_settings_on_exit)
			.add_systems(
				Update,
				apply_in_game_settings_choice.in_set(InGameSettingsSystems::ApplyChoice),
			)
			.add_systems(
				Update,
				sync_in_game_settings_brand
					.in_set(InGameSettingsSystems::Show)
					.after(InGameSettingsSystems::ApplyChoice),
			);
		add_menu_show_systems(app, apply_show_in_game_settings);
	}
}

fn apply_in_game_settings_choice(
	mut choices: MessageReader<InGameSettingsChoice>,
	mut steps: MessageReader<SettingsStep>,
	mut settings: ResMut<InGameSettings>,
	mut machine: ResMut<persist::MachineSettings>,
	mut provenance: ResMut<VisualStyleProvenance>,
	save_root: Option<Res<SaveRoot>>,
	mut commands: Commands,
) {
	let step = steps.read().last().copied();
	let choice = choices.read().last().copied();
	if let Some(step) = step {
		if apply_settings_step(
			step,
			&mut settings,
			&mut machine,
			&mut provenance,
			save_root.as_deref(),
		) {
			request_show_in_game_settings_at(&mut commands, step.choice);
			return;
		}
	}
	let Some(choice) = choice else {
		return;
	};
	match choice {
		InGameSettingsChoice::Shadows => {
			settings.shadows = settings.shadows.cycle();
			persist::write_machine_preference::<catalog::Shadows>(
				Some(&mut machine),
				&mut provenance,
				settings.shadows,
				save_root.as_deref(),
			);
		}
		InGameSettingsChoice::VisualStyle => {
			settings.visual_style = settings.visual_style.cycle();
			persist::write_machine_preference::<catalog::VisualStylePref>(
				Some(&mut machine),
				&mut provenance,
				settings.visual_style,
				save_root.as_deref(),
			);
		}
		InGameSettingsChoice::MobHud => settings.mob_hud = !settings.mob_hud,
	}
	request_show_in_game_settings_at(&mut commands, choice);
}

fn queue_settings_step(
	step: On<MenuStep<InGameSettingsChoice>>,
	mut writer: MessageWriter<SettingsStep>,
) {
	writer.write(SettingsStep { choice: step.event().choice, delta: step.event().delta });
}

fn apply_settings_step(
	step: SettingsStep,
	settings: &mut InGameSettings,
	machine: &mut persist::MachineSettings,
	provenance: &mut VisualStyleProvenance,
	save_root: Option<&SaveRoot>,
) -> bool {
	if step.choice != InGameSettingsChoice::VisualStyle {
		return false;
	}
	settings.visual_style = settings.visual_style.step(step.delta);
	persist::write_machine_preference::<catalog::VisualStylePref>(
		Some(machine),
		provenance,
		settings.visual_style,
		save_root,
	);
	true
}

fn apply_show_in_game_settings(
	mut commands: Commands,
	requests: Query<&RequestShowInGameSettings>,
	screens: MenuScreens,
	mode: Res<GameMode>,
	settings: Res<InGameSettings>,
	readout: Res<RenderScaleReadout>,
) {
	let Some(request) = screens.take(&mut commands, &requests) else {
		return;
	};
	let selected = requests.get(request).ok().and_then(|request| request.selected);
	commands.spawn_scene(InGameSettingsScreen::scene(
		&mode,
		*settings,
		*readout,
		selected.unwrap_or(InGameSettingsChoice::Shadows),
	));
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
	readout: Res<RenderScaleReadout>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<MenuInfoBoxLabel>>,
) {
	set_info_box_for_menu(
		focus.event().entity,
		settings_info_copy(focus.event().choice, *readout),
		&children,
		&mut lines,
	);
}

#[cfg(test)]
mod tests {
	use bevy::prelude::UVec2;

	use menu_components::TextCursorRow;

	use super::{
		apply_settings_step, format_visual_style_info, persist, InGameSettings,
		InGameSettingsChoice, InGameShadowQuality, InGameVisualStyle, RenderScaleReadout,
		SettingsStep, VisualStyleProvenance,
	};

	#[test]
	fn labels_and_descriptions_are_nonempty() {
		for choice in InGameSettingsChoice::ALL {
			assert!(!choice.label().is_empty());
			assert!(!choice.description().is_empty());
		}
	}

	#[test]
	fn visual_style_row_is_a_stepper() {
		let rows: Vec<_> = InGameSettingsChoice::ALL
			.into_iter()
			.map(|choice| {
				let row = TextCursorRow::new(choice.label(), choice);
				if choice == InGameSettingsChoice::VisualStyle {
					row.stepper()
				} else {
					row
				}
			})
			.collect();
		assert!(!rows[0].stepper);
		assert!(rows[1].stepper);
		assert!(!rows[2].stepper);
		assert_eq!(rows[1].action, InGameSettingsChoice::VisualStyle);
	}

	#[test]
	fn mob_hud_starts_off() {
		assert!(!InGameSettings::default().mob_hud);
		assert_eq!(
			InGameSettings {
				mob_hud: false,
				shadows: InGameShadowQuality::High,
				visual_style: InGameVisualStyle::Impressionist,
			}
			.mob_hud_state_label(),
			"Off"
		);
		assert_eq!(
			InGameSettings {
				mob_hud: true,
				shadows: InGameShadowQuality::High,
				visual_style: InGameVisualStyle::Impressionist,
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
		assert_eq!(InGameSettings::default().state_label(InGameSettingsChoice::Shadows), "Off");
	}

	#[test]
	fn left_right_step_updates_the_preference() {
		let mut settings = InGameSettings {
			mob_hud: false,
			shadows: InGameShadowQuality::Off,
			visual_style: InGameVisualStyle::Native,
		};
		let mut machine = persist::MachineSettings::default();
		let mut provenance = VisualStyleProvenance::PendingAuto;
		assert!(apply_settings_step(
			SettingsStep { choice: InGameSettingsChoice::VisualStyle, delta: 1 },
			&mut settings,
			&mut machine,
			&mut provenance,
			None,
		));
		assert_eq!(settings.visual_style, InGameVisualStyle::Sharp);
		assert_eq!(provenance, VisualStyleProvenance::Explicit);
		assert!(provenance.is_saved());
		assert!(apply_settings_step(
			SettingsStep { choice: InGameSettingsChoice::VisualStyle, delta: -1 },
			&mut settings,
			&mut machine,
			&mut provenance,
			None,
		));
		assert_eq!(settings.visual_style, InGameVisualStyle::Native);
		assert!(!apply_settings_step(
			SettingsStep { choice: InGameSettingsChoice::Shadows, delta: 1 },
			&mut settings,
			&mut machine,
			&mut provenance,
			None,
		));
	}

	#[test]
	fn visual_style_provenance_derives_saved_and_auto_reason() {
		use maybraid_world::VisualStyleAutoReason;

		assert!(!VisualStyleProvenance::PendingAuto.is_saved());
		assert!(VisualStyleProvenance::PendingAuto.is_pending());
		assert_eq!(VisualStyleProvenance::PendingAuto.auto_reason(), None);

		let auto =
			VisualStyleProvenance::AutoPicked(VisualStyleAutoReason::UnifiedMemoryBelow24GiB);
		assert!(!auto.is_saved());
		assert!(!auto.is_pending());
		assert_eq!(auto.auto_reason(), Some(VisualStyleAutoReason::UnifiedMemoryBelow24GiB));

		assert!(VisualStyleProvenance::Explicit.is_saved());
		assert_eq!(VisualStyleProvenance::Explicit.auto_reason(), None);
		let mut provenance = auto;
		provenance.mark_explicit();
		assert_eq!(provenance, VisualStyleProvenance::Explicit);

		let mut fields = std::collections::BTreeMap::new();
		assert_eq!(
			VisualStyleProvenance::from_machine_fields(&fields),
			VisualStyleProvenance::PendingAuto
		);
		fields.insert("visual_style".into(), serde_json::json!("native"));
		assert_eq!(
			VisualStyleProvenance::from_machine_fields(&fields),
			VisualStyleProvenance::Explicit
		);
	}

	#[test]
	fn visual_style_starts_native_and_cycles() -> anyhow::Result<()> {
		assert_eq!(InGameSettings::default().visual_style, InGameVisualStyle::Native);
		assert_eq!(InGameVisualStyle::Native.cycle(), InGameVisualStyle::Sharp);
		assert_eq!(InGameVisualStyle::Sharp.cycle(), InGameVisualStyle::Impressionist);
		assert_eq!(InGameVisualStyle::Impressionist.cycle(), InGameVisualStyle::Watercolor);
		assert_eq!(InGameVisualStyle::Watercolor.cycle(), InGameVisualStyle::Retro);
		assert_eq!(InGameVisualStyle::Retro.cycle(), InGameVisualStyle::Antediluvian);
		assert_eq!(InGameVisualStyle::Antediluvian.cycle(), InGameVisualStyle::PrePostModern);
		assert_eq!(InGameVisualStyle::PrePostModern.cycle(), InGameVisualStyle::Native);
		assert_eq!(InGameVisualStyle::Native.step(-1), InGameVisualStyle::PrePostModern);
		assert_eq!(InGameVisualStyle::PrePostModern.step(1), InGameVisualStyle::Native);
		Ok(())
	}

	#[test]
	fn visual_style_select_is_just_the_label() {
		assert_eq!(InGameVisualStyle::Retro.subtext(), "Retro");
		assert_eq!(InGameVisualStyle::Impressionist.subtext(), "Impressionist");
		assert_eq!(
			InGameSettings::default().state_label(InGameSettingsChoice::VisualStyle),
			"Native"
		);
	}

	#[test]
	fn persisted_visual_style_names_match_shared_identity() -> anyhow::Result<()> {
		use maybraid_world::VisualStyleIdentity;

		assert_eq!(InGameVisualStyle::ALL.len(), VisualStyleIdentity::ALL.len());
		for (style, identity) in InGameVisualStyle::ALL.into_iter().zip(VisualStyleIdentity::ALL) {
			assert_eq!(style.to_visual_style(), identity.style);
			assert_eq!(InGameVisualStyle::from_visual_style(identity.style), style);
			assert_eq!(style.label(), identity.label);
			let json = serde_json::to_string(&style)?;
			assert_eq!(json, format!("\"{}\"", identity.script_id));
			assert_eq!(serde_json::from_str::<InGameVisualStyle>(&json)?, style);
		}
		assert_eq!(
			serde_json::from_str::<InGameVisualStyle>("\"pre_post_modern\"")?,
			InGameVisualStyle::PrePostModern
		);
		assert!(serde_json::from_str::<InGameVisualStyle>("\"PrePostModern\"").is_err());
		Ok(())
	}

	#[test]
	fn visual_style_info_is_a_short_pixel_line() -> anyhow::Result<()> {
		let retina = RenderScaleReadout {
			style: InGameVisualStyle::Impressionist.to_visual_style(),
			effective_scale: 0.5,
			window_physical: UVec2::new(3024, 1964),
			target_physical: UVec2::new(1512, 982),
			env_override: None,
			auto_reason: None,
		};
		assert_eq!(
			format_visual_style_info(retina),
			"Rendered at 1512 x 982. Upscaled to display."
		);
		let native = RenderScaleReadout {
			style: InGameVisualStyle::Native.to_visual_style(),
			effective_scale: 1.0,
			window_physical: UVec2::new(1280, 720),
			target_physical: UVec2::new(1280, 720),
			env_override: None,
			auto_reason: None,
		};
		assert_eq!(format_visual_style_info(native), "Rendered at 1280 x 720.");
		assert_eq!(
			format_visual_style_info(RenderScaleReadout::default()),
			"Rendered at display size."
		);
		Ok(())
	}

	#[test]
	fn visual_style_info_mentions_auto_pick_unless_env_wins() {
		use maybraid_world::VisualStyleAutoReason;

		let auto = RenderScaleReadout {
			style: InGameVisualStyle::Sharp.to_visual_style(),
			effective_scale: 1080.0 / 1440.0,
			window_physical: UVec2::new(2560, 1440),
			target_physical: UVec2::new(1920, 1080),
			env_override: None,
			auto_reason: Some(VisualStyleAutoReason::UnifiedMemoryBelow24GiB),
		};
		assert_eq!(
			format_visual_style_info(auto),
			"Rendered at 1920 x 1080. Upscaled to display. Auto-picked for this machine (unified memory below 24 GB)."
		);

		let env_wins = RenderScaleReadout {
			env_override: Some(InGameVisualStyle::Sharp.to_visual_style()),
			..auto
		};
		assert_eq!(
			format_visual_style_info(env_wins),
			"Rendered at 1920 x 1080. Upscaled to display."
		);
	}
}
