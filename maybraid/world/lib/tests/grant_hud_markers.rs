//! Grant generic player + Lubikan owner markers so combat HUD can target a world player.

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use lubikans::LubikanOwner;
use maybraid_world::control;
use player::Player as MaybraidPlayer;
use world_player::Player as WorldPlayer;

#[test]
fn world_player_gains_hud_markers() {
	let mut world = World::new();
	let player = world.spawn(WorldPlayer).id();
	world
		.run_system_once(control::grant_world_player_lubikan_owner)
		.expect("grant");
	assert!(
		world.get::<MaybraidPlayer>(player).is_some(),
		"grant adds player::Player for combat HUD queries"
	);
	assert!(
		world.get::<LubikanOwner>(player).is_some(),
		"grant adds LubikanOwner"
	);
}
