//! Machine-scoped `settings.json` plus `MAYBRAID_SHADOWS` overlay.
//!
//! `MAYBRAID_RENDER_SCALE` is parsed here for Settings copy, but does not rewrite
//! the saved preference. The game applies it to live 3D and the readout.

use std::collections::BTreeMap;
use std::fs;

use bevy::prelude::*;
use character_persist::{PersistError, SaveRoot};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::catalog::{
	overlay_machine_fields, seed_device_class, visual_style_choice_is_saved, DeviceClass,
	Preference, VisualStylePref,
};
use super::{InGameSettings, InGameShadowQuality, InGameVisualStyle};

pub const ENV_SHADOWS: &str = "MAYBRAID_SHADOWS";
pub const ENV_RENDER_SCALE: &str = "MAYBRAID_RENDER_SCALE";
const SETTINGS_VERSION: u32 = 2;
/// Files written before Native/Sharp split. Missing `version` is treated as this.
const LEGACY_SETTINGS_VERSION: u32 = 1;
/// `visual_style: "sharp"` means Native below this version, 1080p Sharp at or above it.
const VISUAL_STYLE_NATIVE_SPLIT_VERSION: u32 = 2;

/// Apply-once marker so a later adapter-info frame cannot re-seed over a loaded
/// or user-set value.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct QualitySeeded;

/// Last Machine projection written or loaded. Env overlays do not live here.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MachineSettings {
	pub settings: InGameSettings,
	pub visual_style_saved: bool,
}

impl Default for MachineSettings {
	fn default() -> Self {
		Self {
			settings: InGameSettings::from_device(seed_device_class()),
			visual_style_saved: false,
		}
	}
}

#[derive(Serialize, Deserialize, Default)]
struct SettingsFile {
	#[serde(default = "legacy_settings_version")]
	version: u32,
	#[serde(default)]
	fields: BTreeMap<String, Value>,
}

fn legacy_settings_version() -> u32 {
	LEGACY_SETTINGS_VERSION
}

/// Version-1 `sharp` was today's direct-window path. Remap it to Native so
/// existing players keep that look; version 2+ `sharp` is the new 1080p mode.
fn migrate_v1_sharp_to_native(settings: &mut InGameSettings, version: u32) {
	if version < VISUAL_STYLE_NATIVE_SPLIT_VERSION
		&& settings.visual_style == InGameVisualStyle::Sharp
	{
		settings.visual_style = InGameVisualStyle::Native;
	}
}

pub fn parse_shadows_env(raw: &str) -> Option<InGameShadowQuality> {
	let raw = raw.trim();
	if raw.is_empty() {
		return None;
	}
	match raw.to_ascii_lowercase().as_str() {
		"high" => Some(InGameShadowQuality::High),
		"low" => Some(InGameShadowQuality::Low),
		"off" => Some(InGameShadowQuality::Off),
		other => {
			eprintln!("[{ENV_SHADOWS}] unknown {other:?} (use high|low|off)");
			None
		}
	}
}

pub fn apply_shadows_env(settings: &mut InGameSettings, raw: Option<&str>) {
	if let Some(quality) = parse_shadows_env(raw.unwrap_or("")) {
		settings.shadows = quality;
	}
}

/// Parse `MAYBRAID_RENDER_SCALE`. `auto` / empty → `None` (keep the saved preference).
pub fn parse_render_scale_env(raw: Option<&str>) -> Option<f32> {
	let Some(raw) = raw else {
		return None;
	};
	let raw = raw.trim();
	if raw.is_empty() || raw.eq_ignore_ascii_case("auto") {
		return None;
	}
	match raw.parse::<f32>() {
		Ok(value) if value.is_finite() => Some(value.clamp(0.5, 1.0)),
		Ok(_) => {
			eprintln!("[{ENV_RENDER_SCALE}] non-finite {raw:?} (use 0.5-1.0 or auto)");
			None
		}
		Err(_) => {
			eprintln!("[{ENV_RENDER_SCALE}] unknown {raw:?} (use 0.5-1.0 or auto)");
			None
		}
	}
}

/// Device defaults, then Machine keys from disk. Env is applied separately so a
/// capture overlay cannot rewrite the file.
pub fn load_machine_settings(root: &SaveRoot, class: DeviceClass) -> InGameSettings {
	load_machine_settings_ex(root, class).settings
}

/// Load Machine keys and whether [`VisualStylePref`] was an explicit saved choice.
pub fn load_machine_settings_ex(root: &SaveRoot, class: DeviceClass) -> LoadedMachineSettings {
	let mut settings = InGameSettings::from_device(class);
	let Some(file) = read_settings_file(root) else {
		return LoadedMachineSettings { settings, visual_style_saved: false };
	};
	overlay_machine_fields(&mut settings, &file.fields);
	migrate_v1_sharp_to_native(&mut settings, file.version);
	LoadedMachineSettings {
		settings,
		visual_style_saved: visual_style_choice_is_saved(&file.fields),
	}
}

/// Device defaults plus whether visual style was saved on disk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadedMachineSettings {
	pub settings: InGameSettings,
	pub visual_style_saved: bool,
}

pub fn save_machine_settings(
	root: &SaveRoot,
	settings: &InGameSettings,
) -> Result<(), PersistError> {
	save_machine_fields(root, settings, true)
}

fn save_machine_fields(
	root: &SaveRoot,
	settings: &InGameSettings,
	persist_visual_style: bool,
) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let mut fields = settings.persisted_fields();
	if !persist_visual_style {
		fields.remove(VisualStylePref::KEY);
	}
	let file = SettingsFile { version: SETTINGS_VERSION, fields };
	fs::write(root.settings_path(), serde_json::to_string_pretty(&file)?)?;
	Ok(())
}

fn read_settings_file(root: &SaveRoot) -> Option<SettingsFile> {
	let json = fs::read_to_string(root.settings_path()).ok()?;
	serde_json::from_str(&json).ok()
}

pub(super) fn write_machine_preference<P: Preference>(
	machine: Option<&mut MachineSettings>,
	value: P::Value,
	root: Option<&SaveRoot>,
) {
	let Some(machine) = machine else {
		return;
	};
	P::set(&mut machine.settings, value);
	if P::KEY == VisualStylePref::KEY {
		machine.visual_style_saved = true;
	}
	let Some(root) = root else {
		return;
	};
	if let Err(error) = save_machine_fields(root, &machine.settings, machine.visual_style_saved) {
		warn!("failed to persist machine settings: {error}");
	}
}

/// Apply the host heuristic when `visual_style` was never an explicit save.
///
/// Returns `None` when [`VisualStyleSaved`] is set so an explicit choice stays.
pub fn apply_unsaved_visual_style_default(
	visual_style_saved: bool,
	settings: &mut InGameSettings,
	signals: &maybraid_world::VisualStyleSignals,
) -> Option<maybraid_world::VisualStylePick> {
	if visual_style_saved {
		return None;
	}
	let pick = maybraid_world::pick_default_visual_style(signals);
	settings.visual_style = InGameVisualStyle::from_visual_style(pick.style);
	Some(pick)
}

/// Load Machine defaults and the live bag. `None` when [`QualitySeeded`] is set.
pub fn apply_seed(
	already_seeded: bool,
	root: Option<&SaveRoot>,
	shadows_env: Option<&str>,
) -> Option<(InGameSettings, InGameSettings, bool)> {
	if already_seeded {
		return None;
	}
	let class = seed_device_class();
	let loaded = root.map(|root| load_machine_settings_ex(root, class)).unwrap_or_else(|| {
		LoadedMachineSettings {
			settings: InGameSettings::from_device(class),
			visual_style_saved: false,
		}
	});
	let mut live = loaded.settings;
	apply_shadows_env(&mut live, shadows_env);
	Some((loaded.settings, live, loaded.visual_style_saved))
}

pub(super) fn seed_in_game_settings(
	mut commands: Commands,
	mut settings: ResMut<InGameSettings>,
	mut machine: ResMut<MachineSettings>,
	seeded: Option<Res<QualitySeeded>>,
	save_root: Option<Res<SaveRoot>>,
) {
	let env = std::env::var(ENV_SHADOWS).ok();
	let Some((loaded, live, visual_style_saved)) =
		apply_seed(seeded.is_some(), save_root.as_deref(), env.as_deref())
	else {
		return;
	};
	*machine = MachineSettings { settings: loaded, visual_style_saved };
	*settings = live;
	commands.insert_resource(QualitySeeded);
	commands.insert_resource(super::VisualStyleSaved(visual_style_saved));
	commands.insert_resource(super::VisualStyleAutoPick::default());
}

pub(super) fn persist_machine_settings_on_exit(
	exits: MessageReader<AppExit>,
	machine: Res<MachineSettings>,
	save_root: Option<Res<SaveRoot>>,
) {
	if exits.is_empty() {
		return;
	}
	let Some(root) = save_root else {
		return;
	};
	if let Err(error) =
		save_machine_fields(root.as_ref(), &machine.settings, machine.visual_style_saved)
	{
		warn!("failed to persist machine settings on exit: {error}");
	}
}

#[cfg(test)]
mod tests {
	use character_persist::SaveRoot;

	use super::{
		apply_seed, apply_shadows_env, apply_unsaved_visual_style_default, load_machine_settings,
		load_machine_settings_ex, parse_render_scale_env, parse_shadows_env, save_machine_fields,
		save_machine_settings, write_machine_preference, MachineSettings, ENV_RENDER_SCALE,
		ENV_SHADOWS,
	};
	use crate::settings::catalog::{DeviceClass, Shadows, VisualStylePref};
	use crate::settings::{InGameSettings, InGameShadowQuality, InGameVisualStyle};

	fn write_fields(root: &SaveRoot, json: &str) {
		root.ensure_dirs().expect("dirs");
		std::fs::write(root.settings_path(), json).expect("write settings");
	}

	fn read_raw(root: &SaveRoot) -> String {
		std::fs::read_to_string(root.settings_path()).expect("read settings")
	}

	#[test]
	fn missing_file_uses_modest_off_native() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let settings = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(settings.shadows, InGameShadowQuality::Off);
		assert_eq!(settings.visual_style, InGameVisualStyle::Native);
		assert!(!settings.mob_hud);
	}

	#[test]
	fn round_trip_machine_drops_session() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		save_machine_settings(
			&root,
			&InGameSettings {
				shadows: InGameShadowQuality::Low,
				mob_hud: true,
				visual_style: InGameVisualStyle::Retro,
			},
		)
		.expect("save");
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.shadows, InGameShadowQuality::Low);
		assert_eq!(loaded.visual_style, InGameVisualStyle::Retro);
		assert!(!loaded.mob_hud);

		write_fields(&root, r#"{ "version": 1, "fields": { "shadows": "low", "mob_hud": true } }"#);
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.shadows, InGameShadowQuality::Low);
		assert!(!loaded.mob_hud);
	}

	#[test]
	fn unknown_key_is_ignored() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		write_fields(&root, r#"{ "version": 1, "fields": { "shadows": "high", "bloom": true } }"#);
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.shadows, InGameShadowQuality::High);
		assert!(!loaded.mob_hud);
	}

	#[test]
	fn env_wins_and_does_not_rewrite_the_file() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		save_machine_settings(
			&root,
			&InGameSettings {
				shadows: InGameShadowQuality::Off,
				mob_hud: false,
				visual_style: InGameVisualStyle::Impressionist,
			},
		)
		.expect("save");
		let before = read_raw(&root);

		let mut settings = load_machine_settings(&root, DeviceClass::Modest);
		apply_shadows_env(&mut settings, Some("high"));
		assert_eq!(settings.shadows, InGameShadowQuality::High);
		assert_eq!(parse_shadows_env("HIGH"), Some(InGameShadowQuality::High));
		assert_eq!(parse_shadows_env(""), None);
		assert_eq!(ENV_SHADOWS, "MAYBRAID_SHADOWS");
		assert_eq!(read_raw(&root), before);
		assert!(before.contains("\"off\""));
	}

	#[test]
	fn user_cycle_persists_and_later_seed_does_not_reset() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let (machine, live, visual_style_saved) =
			apply_seed(false, Some(&root), None).expect("seed");
		assert_eq!(live.shadows, InGameShadowQuality::Off);
		assert_eq!(machine.shadows, InGameShadowQuality::Off);
		assert!(!visual_style_saved);

		let mut stored = MachineSettings { settings: machine, visual_style_saved };
		write_machine_preference::<Shadows>(
			Some(&mut stored),
			InGameShadowQuality::Low,
			Some(&root),
		);
		assert_eq!(stored.settings.shadows, InGameShadowQuality::Low);
		assert!(!stored.visual_style_saved);
		assert!(read_raw(&root).contains("\"low\""));
		assert!(!read_raw(&root).contains("\"visual_style\""));

		assert!(apply_seed(true, Some(&root), None).is_none());
		let (_, live, still_unsaved) = apply_seed(false, Some(&root), None).expect("reload");
		assert_eq!(live.shadows, InGameShadowQuality::Low);
		assert!(!still_unsaved);
	}

	#[test]
	fn visual_style_round_trips_and_migrates_from_legacy() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		save_machine_settings(
			&root,
			&InGameSettings {
				shadows: InGameShadowQuality::Off,
				mob_hud: false,
				visual_style: InGameVisualStyle::PrePostModern,
			},
		)?;
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.visual_style, InGameVisualStyle::PrePostModern);
		let before = read_raw(&root);
		assert!(before.contains("\"pre_post_modern\""));

		let mut stored = MachineSettings { settings: loaded, visual_style_saved: true };
		write_machine_preference::<VisualStylePref>(
			Some(&mut stored),
			InGameVisualStyle::Native,
			Some(&root),
		);
		assert_eq!(stored.settings.visual_style, InGameVisualStyle::Native);
		assert!(stored.visual_style_saved);
		assert!(read_raw(&root).contains("\"native\""));
		assert!(read_raw(&root).contains("\"version\": 2"));

		write_machine_preference::<VisualStylePref>(
			Some(&mut stored),
			InGameVisualStyle::Sharp,
			Some(&root),
		);
		assert_eq!(stored.settings.visual_style, InGameVisualStyle::Sharp);
		assert!(read_raw(&root).contains("\"sharp\""));

		// Test migration from legacy render_scale
		write_fields(
			&root,
			r#"{ "version": 1, "fields": { "shadows": "off", "render_scale": "75" } }"#,
		);
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.visual_style, InGameVisualStyle::PrePostModern);
		Ok(())
	}

	#[test]
	fn version_1_sharp_migrates_to_native() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		write_fields(
			&root,
			r#"{ "version": 1, "fields": { "shadows": "off", "visual_style": "sharp" } }"#,
		);
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.visual_style, InGameVisualStyle::Native);
	}

	#[test]
	fn unversioned_sharp_migrates_to_native() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		write_fields(&root, r#"{ "fields": { "shadows": "off", "visual_style": "sharp" } }"#);
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.visual_style, InGameVisualStyle::Native);
	}

	#[test]
	fn version_2_sharp_stays_sharp() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		write_fields(
			&root,
			r#"{ "version": 2, "fields": { "shadows": "off", "visual_style": "sharp" } }"#,
		);
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.visual_style, InGameVisualStyle::Sharp);
	}

	#[test]
	fn render_scale_env_parsing_preserved() -> anyhow::Result<()> {
		assert_eq!(parse_render_scale_env(Some("0.75")), Some(0.75));
		assert_eq!(parse_render_scale_env(Some("auto")), None);
		assert_eq!(parse_render_scale_env(Some("0.1")), Some(0.5));
		assert_eq!(ENV_RENDER_SCALE, "MAYBRAID_RENDER_SCALE");
		Ok(())
	}

	#[test]
	fn missing_file_and_legacy_auto_leave_visual_style_unsaved() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let loaded = load_machine_settings_ex(&root, DeviceClass::Modest);
		assert!(!loaded.visual_style_saved);
		assert_eq!(loaded.settings.visual_style, InGameVisualStyle::Native);

		write_fields(
			&root,
			r#"{ "version": 1, "fields": { "shadows": "off", "render_scale": "auto" } }"#,
		);
		let loaded = load_machine_settings_ex(&root, DeviceClass::Modest);
		assert!(!loaded.visual_style_saved);
		assert_eq!(loaded.settings.visual_style, InGameVisualStyle::Native);
	}

	#[test]
	fn explicit_visual_style_or_legacy_scale_is_saved() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		write_fields(
			&root,
			r#"{ "version": 2, "fields": { "shadows": "off", "visual_style": "impressionist" } }"#,
		);
		let loaded = load_machine_settings_ex(&root, DeviceClass::Modest);
		assert!(loaded.visual_style_saved);
		assert_eq!(loaded.settings.visual_style, InGameVisualStyle::Impressionist);

		write_fields(
			&root,
			r#"{ "version": 1, "fields": { "shadows": "off", "render_scale": "75" } }"#,
		);
		let loaded = load_machine_settings_ex(&root, DeviceClass::Modest);
		assert!(loaded.visual_style_saved);
		assert_eq!(loaded.settings.visual_style, InGameVisualStyle::PrePostModern);
	}

	#[test]
	fn unsaved_visual_style_is_omitted_until_the_player_picks() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		save_machine_fields(
			&root,
			&InGameSettings {
				shadows: InGameShadowQuality::Off,
				mob_hud: false,
				visual_style: InGameVisualStyle::Sharp,
			},
			false,
		)
		.expect("save");
		assert!(!read_raw(&root).contains("\"visual_style\""));
		assert!(!load_machine_settings_ex(&root, DeviceClass::Modest).visual_style_saved);
	}

	#[test]
	fn auto_pick_applies_only_when_unsaved() {
		use bevy::prelude::UVec2;
		use maybraid_world::{AdapterKind, VisualStyle, VisualStyleAutoReason, VisualStyleSignals};

		let mut settings = InGameSettings {
			shadows: InGameShadowQuality::Off,
			mob_hud: false,
			visual_style: InGameVisualStyle::Native,
		};
		let mut signals = VisualStyleSignals {
			adapter_kind: AdapterKind::IntegratedGpu,
			name: "Intel Iris Xe Graphics".into(),
			window: UVec2::new(2560, 1440),
			cpu_cores: Some(8),
			memory_bytes: Some(16 * 1024 * 1024 * 1024),
			..VisualStyleSignals::default()
		};
		let pick =
			apply_unsaved_visual_style_default(false, &mut settings, &signals).expect("pick");
		assert_eq!(pick.style, VisualStyle::Sharp);
		assert_eq!(pick.reason, VisualStyleAutoReason::UnifiedMemoryBelow24GiB);
		assert_eq!(settings.visual_style, InGameVisualStyle::Sharp);

		settings.visual_style = InGameVisualStyle::Impressionist;
		signals.device_tier = Some(maybraid_world::DeviceTier::Low);
		assert!(apply_unsaved_visual_style_default(true, &mut settings, &signals).is_none());
		assert_eq!(settings.visual_style, InGameVisualStyle::Impressionist);
	}
}
