//! Equipment ↔ concrete item crates. Equipment owns category / holster state;
//! firearm-user and grenade-user realize hold and teardown.

use bevy::prelude::*;
use firearm_user::register_firearm_user_runtime;
use grenade_user::register_grenade_user_runtime;
use maybraid_equipment::EquipmentPlugin;

/// Register equipment contracts and the item runtimes that realize them.
pub fn configure(app: &mut App) {
	if !app.is_plugin_added::<EquipmentPlugin>() {
		app.add_plugins(EquipmentPlugin);
	}
	register_firearm_user_runtime(app);
	register_grenade_user_runtime(app);
}
