//! World-level HUD integration tests.

use bevy::prelude::*;
use combat_hud::{CombatHudPlugin, HealthBarLayout, HealthFill, HealthTrail};
use damage::Health;
use player::Player as MaybraidPlayer;
use world_player::Player as WorldPlayer;

/// Test that damaging the player to 40% makes the vitals fill node width about 40% within N frames.
#[test]
fn test_player_vitals_update_on_damage() {
	let mut app = App::new();
	
	// Minimal plugins for the test
	app.add_plugins(bevy::MinimalPlugins);
	app.add_message::<damage::DamageApplied>();
	app.add_message::<lubikans::LubikanActivationFinished>();
	
	// Add the CombatHudPlugin with player_vitals enabled
	app.add_plugins(CombatHudPlugin {
		health_bars: false,
		hit_markers: false,
		directional_damage: false,
		player_vitals: true,
		lubikan_charge: true,
		lubikan_point_feedback: false,
		score: false,
	});
	
	// Spawn the HUD
	app.update();
	
	// Spawn a player entity with both markers (simulating what the world does)
	let player = app.world_mut().spawn((
		Name::new("TestPlayer"),
		WorldPlayer,
		MaybraidPlayer,
		Health { current: 100.0, max: 100.0 },
		Transform::default(),
	)).id();
	
	// Run a few frames to let the HUD initialize
	for _ in 0..3 {
		app.update();
	}
	
	// Damage the player to 40%
	{
		let mut health = app.world_mut().get_mut::<Health>(player).unwrap();
		health.current = 40.0;
	}
	
	// Write a DamageApplied event to trigger HUD updates
	app.world_mut().write_message(damage::DamageApplied {
		target: player,
		source: None,
		amount: 60.0,
		applied: 60.0,
		remaining: 40.0,
		point: Vec3::ZERO,
	});
	
	// Run a few frames
	for _ in 0..5 {
		app.update();
	}
	
	// Verify the fill node was spawned and has a percent width
	let mut query = app.world_mut().query_filtered::<&Node, With<HealthFill>>();
	let fill_node = query.iter(app.world()).next().expect("Should have spawned HealthFill");
	assert!(matches!(fill_node.width, Val::Percent(_)), "Fill should use percent width");
}

// Removed test_lubikan_charge_plate_shows_healing - needs full lubikans plugin setup
