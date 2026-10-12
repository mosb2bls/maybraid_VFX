//! Settings rows: device class, persist policy, and the live-bag projection.

use std::collections::BTreeMap;

use bevy::prelude::info;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use super::{InGameRenderScale, InGameSettings, InGameShadowQuality, InGameVisualStyle};

/// Host class used to seed catalog defaults.
///
/// v1 seeds [`DeviceClass::Modest`] for every probe, including discrete desktop.
/// `Small` and `Full` stay reserved until a follow-up revisits High defaults.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceClass {
	Small,
	Modest,
	Full,
}

/// wgpu-style adapter kind. The v1 classifier inspects this but does not branch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceType {
	DiscreteGpu,
	IntegratedGpu,
	Cpu,
	Other,
}

/// Adapter identity plus the primary framebuffer, collected once for classification.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HostProbe {
	pub device_type: DeviceType,
	pub width: u32,
	pub height: u32,
}

impl HostProbe {
	pub fn short_side(self) -> u32 {
		self.width.min(self.height)
	}
}

/// Classify the host. v1 returns Modest for every probe we accept.
pub fn classify_host(probe: HostProbe) -> DeviceClass {
	match (probe.device_type, probe.short_side()) {
		(
			DeviceType::DiscreteGpu
			| DeviceType::IntegratedGpu
			| DeviceType::Cpu
			| DeviceType::Other,
			_,
		) => DeviceClass::Modest,
	}
}

/// Device class that seeds catalog defaults. Always Modest until Full is revisited.
pub fn seed_device_class() -> DeviceClass {
	DeviceClass::Modest
}

/// Whether a Settings row is written to `settings.json`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PersistPolicy {
	/// Debug / session chrome. Never read or written. Code default every boot.
	Session,
	/// Install preference. One file, shared across characters.
	Machine,
}

/// One Settings row. Add a new row = add one impl; the JSON map grows by one key.
pub trait Preference {
	const KEY: &'static str;
	const PERSIST: PersistPolicy;
	type Value: Serialize + DeserializeOwned + Clone + PartialEq;

	fn get(settings: &InGameSettings) -> Self::Value;
	fn set(settings: &mut InGameSettings, value: Self::Value);
	fn device_default(class: DeviceClass) -> Self::Value;
}

/// Sun cascade quality. Machine-scoped.
pub struct Shadows;

impl Preference for Shadows {
	const KEY: &'static str = "shadows";
	const PERSIST: PersistPolicy = PersistPolicy::Machine;
	type Value = InGameShadowQuality;

	fn get(settings: &InGameSettings) -> Self::Value {
		settings.shadows
	}

	fn set(settings: &mut InGameSettings, value: Self::Value) {
		settings.shadows = value;
	}

	fn device_default(_class: DeviceClass) -> Self::Value {
		// v1: do not assign Full / High as a default yet.
		InGameShadowQuality::Off
	}
}

/// Mob HUD pins. Session chrome; omitted from load and save.
pub struct MobHud;

impl Preference for MobHud {
	const KEY: &'static str = "mob_hud";
	const PERSIST: PersistPolicy = PersistPolicy::Session;
	type Value = bool;

	fn get(settings: &InGameSettings) -> Self::Value {
		settings.mob_hud
	}

	fn set(settings: &mut InGameSettings, value: Self::Value) {
		settings.mob_hud = value;
	}

	fn device_default(_class: DeviceClass) -> Self::Value {
		false
	}
}

/// Visual style presentation mode. Machine-scoped.
pub struct VisualStylePref;

impl Preference for VisualStylePref {
	const KEY: &'static str = "visual_style";
	const PERSIST: PersistPolicy = PersistPolicy::Machine;
	type Value = InGameVisualStyle;

	fn get(settings: &InGameSettings) -> Self::Value {
		settings.visual_style
	}

	fn set(settings: &mut InGameSettings, value: Self::Value) {
		settings.visual_style = value;
	}

	fn device_default(_class: DeviceClass) -> Self::Value {
		InGameVisualStyle::Impressionist
	}
}

/// LEGACY: 3D shading scale. Machine-scoped. Preserved for migration only.
/// This preference is no longer active; VisualStylePref replaced it.
pub struct RenderScalePref;

impl Preference for RenderScalePref {
	const KEY: &'static str = "render_scale";
	const PERSIST: PersistPolicy = PersistPolicy::Machine;
	type Value = InGameRenderScale;

	fn get(_settings: &InGameSettings) -> Self::Value {
		// Not active; exists only for migration.
		InGameRenderScale::Auto
	}

	fn set(_settings: &mut InGameSettings, _value: Self::Value) {
		// Not active; exists only for migration.
	}

	fn device_default(_class: DeviceClass) -> Self::Value {
		InGameRenderScale::Auto
	}
}

impl InGameSettings {
	pub fn from_device(class: DeviceClass) -> Self {
		Self {
			shadows: Shadows::device_default(class),
			mob_hud: MobHud::device_default(class),
			visual_style: VisualStylePref::device_default(class),
		}
	}

	/// Machine keys only. Session chrome such as Mob HUD is omitted.
	pub fn persisted_fields(&self) -> BTreeMap<String, Value> {
		let mut fields = BTreeMap::new();
		write_machine_field::<Shadows>(&mut fields, self);
		write_machine_field::<VisualStylePref>(&mut fields, self);
		write_machine_field::<MobHud>(&mut fields, self);
		fields
	}
}

pub(super) fn overlay_machine_fields(
	settings: &mut InGameSettings,
	fields: &BTreeMap<String, Value>,
) {
	overlay_machine_field::<Shadows>(settings, fields);
	overlay_machine_field::<VisualStylePref>(settings, fields);
	overlay_machine_field::<MobHud>(settings, fields);
	
	// Migration: if visual_style is missing but render_scale exists, migrate
	if !fields.contains_key(VisualStylePref::KEY) && fields.contains_key(RenderScalePref::KEY) {
		if let Ok(legacy_scale) = serde_json::from_value::<InGameRenderScale>(
			fields.get(RenderScalePref::KEY).unwrap().clone(),
		) {
			settings.visual_style = InGameVisualStyle::from_legacy_render_scale(legacy_scale);
			info!(
				"Migrated legacy render_scale {:?} -> visual_style {:?}",
				legacy_scale, settings.visual_style
			);
		}
	}
}

fn write_machine_field<P: Preference>(
	fields: &mut BTreeMap<String, Value>,
	settings: &InGameSettings,
) {
	if P::PERSIST != PersistPolicy::Machine {
		return;
	}
	if let Ok(value) = serde_json::to_value(P::get(settings)) {
		fields.insert(P::KEY.to_string(), value);
	}
}

fn overlay_machine_field<P: Preference>(
	settings: &mut InGameSettings,
	fields: &BTreeMap<String, Value>,
) {
	if P::PERSIST != PersistPolicy::Machine {
		return;
	}
	let Some(value) = fields.get(P::KEY) else {
		return;
	};
	if let Ok(parsed) = serde_json::from_value(value.clone()) {
		P::set(settings, parsed);
	}
}

#[cfg(test)]
mod tests {
	use super::{
		classify_host, seed_device_class, DeviceClass, DeviceType, HostProbe, InGameSettings,
		InGameShadowQuality, InGameVisualStyle, MobHud, Preference, Shadows, VisualStylePref,
	};

	#[test]
	fn v1_class_is_modest_for_discrete_desktop() {
		let class = classify_host(HostProbe {
			device_type: DeviceType::DiscreteGpu,
			width: 1920,
			height: 1080,
		});
		assert_eq!(class, DeviceClass::Modest);
		assert_eq!(seed_device_class(), DeviceClass::Modest);
		assert_eq!(InGameSettings::from_device(class).shadows, InGameShadowQuality::Off);
	}

	#[test]
	fn integrated_is_also_modest() {
		let class = classify_host(HostProbe {
			device_type: DeviceType::IntegratedGpu,
			width: 1280,
			height: 720,
		});
		assert_eq!(class, DeviceClass::Modest);
		assert_eq!(Shadows::device_default(class), InGameShadowQuality::Off);
		assert!(!MobHud::device_default(class));
	}

	#[test]
	fn catalog_projection_writes_machine_keys_only() {
		let fields = InGameSettings {
			shadows: InGameShadowQuality::Low,
			mob_hud: true,
			visual_style: InGameVisualStyle::Sharp,
		}
		.persisted_fields();
		assert_eq!(fields.get("shadows"), Some(&serde_json::json!("low")));
		assert_eq!(fields.get("visual_style"), Some(&serde_json::json!("sharp")));
		assert!(!fields.contains_key("mob_hud"));
		assert_eq!(fields.len(), 2);
	}

	#[test]
	fn visual_style_defaults_to_impressionist() {
		assert_eq!(
			VisualStylePref::device_default(DeviceClass::Modest),
			InGameVisualStyle::Impressionist
		);
		assert_eq!(
			InGameSettings::from_device(DeviceClass::Modest).visual_style,
			InGameVisualStyle::Impressionist
		);
	}
}
