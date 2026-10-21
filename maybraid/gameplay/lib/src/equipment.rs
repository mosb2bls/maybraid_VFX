//! Equipment ↔ concrete item crates. Equipment owns category / holster state;
//! firearm-user and grenade-user realize hold and teardown.

use bevy::prelude::*;
use firearm_user::register_firearm_user_runtime;
use grenade_user::register_grenade_user_runtime;
use maybraid_equipment::EquipmentPlugin;

/// Sole gameplay owner of firearm / grenade *runtime* registration.
///
/// Item plugins and world intelligence may call the same `register_*_runtime`
/// functions; those are idempotent. Presentation stays on the item plugins.
pub struct ItemRuntimes;

impl ItemRuntimes {
	/// Headless tests and production share this. A second call is a no-op.
	pub fn register(app: &mut App) {
		register_firearm_user_runtime(app);
		register_grenade_user_runtime(app);
	}
}

/// Register equipment contracts and the item runtimes that realize them.
pub fn configure(app: &mut App) {
	if !app.is_plugin_added::<EquipmentPlugin>() {
		app.add_plugins(EquipmentPlugin);
	}
	ItemRuntimes::register(app);
}
