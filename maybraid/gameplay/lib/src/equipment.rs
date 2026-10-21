//! Equipment ↔ concrete item crates. Equipment owns category / holster state;
//! firearm-user and grenade-user realize hold and teardown on their plugins.

use bevy::prelude::*;
use maybraid_equipment::EquipmentPlugin;

/// Register equipment contracts. Item crates realize [`PendingEquipmentHold`]
/// / [`PendingEquipmentTeardown`] in `EquipmentSystems::Realize`.
///
/// Do not call `register_*_user_runtime` here. Production item plugins stay on
/// world intelligence. Those registers are idempotent, but gameplay still
/// must not own them: one assembly point, item crates realize. Headless
/// stack tests register the runtimes themselves.
pub fn configure(app: &mut App) {
	if !app.is_plugin_added::<EquipmentPlugin>() {
		app.add_plugins(EquipmentPlugin);
	}
}
