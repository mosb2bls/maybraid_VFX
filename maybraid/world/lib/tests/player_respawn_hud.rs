//! Test that HUD markers persist through player death and respawn.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use damage::Health;
use lubikans::LubikanOwner;
use maybraid_world::control;
use player::Player as MaybraidPlayer;
use world_player::Player as WorldPlayer;

#[test]
fn respawned_player_gets_hud_markers() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);

	// Spawn initial player with both markers
	let player = app
		.world_mut()
		.spawn((
			WorldPlayer,
			MaybraidPlayer,
			LubikanOwner,
			Health { current: 50.0, max: 100.0 },
			Transform::default(),
		))
		.id();

	// Simulate death: remove markers like queue_downed_world_player does
	app.world_mut().entity_mut(player).remove::<(WorldPlayer, MaybraidPlayer)>();

	// Respawn: spawn new entity with only WorldPlayer (like finish_world_player_spawn)
	let respawned = app
		.world_mut()
		.spawn((WorldPlayer, Health { current: 100.0, max: 100.0 }, Transform::default()))
		.id();

	// Run grant system
	app.world_mut()
		.run_system_once(control::grant_world_player_lubikan_owner)
		.unwrap();

	// Verify respawned player got both markers
	assert!(
		app.world().get::<WorldPlayer>(respawned).is_some(),
		"respawned player has WorldPlayer"
	);
	assert!(
		app.world().get::<MaybraidPlayer>(respawned).is_some(),
		"respawned player has player::Player"
	);
	assert!(
		app.world().get::<LubikanOwner>(respawned).is_some(),
		"respawned player has LubikanOwner"
	);
}

#[test]
fn hud_markers_added_on_initial_spawn() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);

	// Spawn player with only WorldPlayer (like world does initially)
	let player = app
		.world_mut()
		.spawn((WorldPlayer, Health { current: 100.0, max: 100.0 }, Transform::default()))
		.id();

	// Run grant system
	app.world_mut()
		.run_system_once(control::grant_world_player_lubikan_owner)
		.unwrap();

	// Verify player got both markers
	assert!(app.world().get::<WorldPlayer>(player).is_some());
	assert!(
		app.world().get::<MaybraidPlayer>(player).is_some(),
		"initial player gets player::Player"
	);
	assert!(app.world().get::<LubikanOwner>(player).is_some(), "initial player gets LubikanOwner");
}
