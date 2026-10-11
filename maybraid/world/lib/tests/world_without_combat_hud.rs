//! Test that world gameplay works without CombatHudPlugin.

use bevy::prelude::*;
use maybraid_world::WorldPlugin;

#[test]
fn world_builds_without_combat_hud() {
	let mut app = App::new();
	
	// Add only minimal Bevy plugins and WorldPlugin, no CombatHudPlugin
	app.add_plugins(MinimalPlugins);
	
	// WorldPlugin should build successfully without CombatHudPlugin
	// This ensures gameplay is independent of HUD presentation
	app.add_plugins(WorldPlugin::default());
	
	// Run one update to verify systems are registered correctly
	app.update();
	
	// Test passes if we get here without panicking
}
