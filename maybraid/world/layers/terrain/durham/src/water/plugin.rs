//! Idempotent plugin for the Durham water model.

use bevy::prelude::*;

/// Registers resources for the water model.
pub struct WaterPlugin;

impl Default for WaterPlugin {
	fn default() -> Self {
		Self
	}
}

/// Idempotent registration of [`WaterPlugin`].
pub fn register_water_plugin(app: &mut App) {
	if app.is_plugin_added::<WaterPlugin>() {
		return;
	}
	app.add_plugins(WaterPlugin);
}

impl Plugin for WaterPlugin {
	fn build(&self, _app: &mut App) {}
}
