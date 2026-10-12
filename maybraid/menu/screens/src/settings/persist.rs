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

use super::catalog::{overlay_machine_fields, seed_device_class, DeviceClass, Preference};
use super::{InGameSettings, InGameShadowQuality};

pub const ENV_SHADOWS: &str = "MAYBRAID_SHADOWS";
pub const ENV_RENDER_SCALE: &str = "MAYBRAID_RENDER_SCALE";
const SETTINGS_VERSION: u32 = 1;

/// Apply-once marker so a later adapter-info frame cannot re-seed over a loaded
/// or user-set value.
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct QualitySeeded;

/// Last Machine projection written or loaded. Env overlays do not live here.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MachineSettings(pub InGameSettings);

impl Default for MachineSettings {
	fn default() -> Self {
		Self(InGameSettings::from_device(seed_device_class()))
	}
}

#[derive(Serialize, Deserialize, Default)]
struct SettingsFile {
	#[serde(default = "settings_version")]
	version: u32,
	#[serde(default)]
	fields: BTreeMap<String, Value>,
}

fn settings_version() -> u32 {
	SETTINGS_VERSION
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
	let mut settings = InGameSettings::from_device(class);
	if let Some(file) = read_settings_file(root) {
		overlay_machine_fields(&mut settings, &file.fields);
	}
	settings
}

pub fn save_machine_settings(
	root: &SaveRoot,
	settings: &InGameSettings,
) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let file = SettingsFile { version: SETTINGS_VERSION, fields: settings.persisted_fields() };
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
	P::set(&mut machine.0, value);
	let Some(root) = root else {
		return;
	};
	if let Err(error) = save_machine_settings(root, &machine.0) {
		warn!("failed to persist machine settings: {error}");
	}
}

/// Load Machine defaults and the live bag. `None` when [`QualitySeeded`] is set.
pub fn apply_seed(
	already_seeded: bool,
	root: Option<&SaveRoot>,
	shadows_env: Option<&str>,
) -> Option<(InGameSettings, InGameSettings)> {
	if already_seeded {
		return None;
	}
	let class = seed_device_class();
	let machine = root
		.map(|root| load_machine_settings(root, class))
		.unwrap_or_else(|| InGameSettings::from_device(class));
	let mut live = machine;
	apply_shadows_env(&mut live, shadows_env);
	Some((machine, live))
}

pub(super) fn seed_in_game_settings(
	mut commands: Commands,
	mut settings: ResMut<InGameSettings>,
	mut machine: ResMut<MachineSettings>,
	seeded: Option<Res<QualitySeeded>>,
	save_root: Option<Res<SaveRoot>>,
) {
	let env = std::env::var(ENV_SHADOWS).ok();
	let Some((loaded, live)) = apply_seed(seeded.is_some(), save_root.as_deref(), env.as_deref())
	else {
		return;
	};
	*machine = MachineSettings(loaded);
	*settings = live;
	commands.insert_resource(QualitySeeded);
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
	if let Err(error) = save_machine_settings(root.as_ref(), &machine.0) {
		warn!("failed to persist machine settings on exit: {error}");
	}
}

#[cfg(test)]
mod tests {
	use character_persist::SaveRoot;

	use super::{
		apply_seed, apply_shadows_env, load_machine_settings, parse_render_scale_env,
		parse_shadows_env, save_machine_settings, write_machine_preference, MachineSettings,
		ENV_RENDER_SCALE, ENV_SHADOWS,
	};
	use crate::settings::catalog::{DeviceClass, RenderScalePref, Shadows};
	use crate::settings::{InGameRenderScale, InGameSettings, InGameShadowQuality};

	fn write_fields(root: &SaveRoot, json: &str) {
		root.ensure_dirs().expect("dirs");
		std::fs::write(root.settings_path(), json).expect("write settings");
	}

	fn read_raw(root: &SaveRoot) -> String {
		std::fs::read_to_string(root.settings_path()).expect("read settings")
	}

	#[test]
	fn missing_file_uses_modest_off() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let settings = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(settings.shadows, InGameShadowQuality::Off);
		assert_eq!(settings.render_scale, InGameRenderScale::Auto);
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
				render_scale: InGameRenderScale::Half,
			},
		)
		.expect("save");
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.shadows, InGameShadowQuality::Low);
		assert_eq!(loaded.render_scale, InGameRenderScale::Half);
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
				render_scale: InGameRenderScale::Auto,
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
		let (machine, live) = apply_seed(false, Some(&root), None).expect("seed");
		assert_eq!(live.shadows, InGameShadowQuality::Off);
		assert_eq!(machine.shadows, InGameShadowQuality::Off);

		let mut stored = MachineSettings(machine);
		write_machine_preference::<Shadows>(
			Some(&mut stored),
			InGameShadowQuality::Low,
			Some(&root),
		);
		assert_eq!(stored.0.shadows, InGameShadowQuality::Low);
		assert!(read_raw(&root).contains("\"low\""));

		assert!(apply_seed(true, Some(&root), None).is_none());
		let (_, live) = apply_seed(false, Some(&root), None).expect("reload");
		assert_eq!(live.shadows, InGameShadowQuality::Low);
	}

	#[test]
	fn render_scale_round_trips_and_env_does_not_rewrite() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		save_machine_settings(
			&root,
			&InGameSettings {
				shadows: InGameShadowQuality::Off,
				mob_hud: false,
				render_scale: InGameRenderScale::ThreeQuarter,
			},
		)?;
		let loaded = load_machine_settings(&root, DeviceClass::Modest);
		assert_eq!(loaded.render_scale, InGameRenderScale::ThreeQuarter);
		let before = read_raw(&root);
		assert_eq!(parse_render_scale_env(Some("0.75")), Some(0.75));
		assert_eq!(parse_render_scale_env(Some("auto")), None);
		assert_eq!(parse_render_scale_env(Some("0.1")), Some(0.5));
		assert_eq!(ENV_RENDER_SCALE, "MAYBRAID_RENDER_SCALE");
		assert_eq!(read_raw(&root), before);
		assert!(before.contains("\"75\""));

		let mut stored = MachineSettings(loaded);
		write_machine_preference::<RenderScalePref>(
			Some(&mut stored),
			InGameRenderScale::Half,
			Some(&root),
		);
		assert_eq!(stored.0.render_scale, InGameRenderScale::Half);
		assert!(read_raw(&root).contains("\"50\""));
		Ok(())
	}
}
