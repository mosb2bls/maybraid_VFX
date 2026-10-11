//! Temporary [`LubikanOwner`] / [`LubikanLedger`] stamp for world players.
//!
//! Owner lifetime registration is owned by [#1107](https://github.com/ramate-io/maybraid/issues/1107).

use bevy::prelude::*;
use lubikans::{LubikanLedger, LubikanOwner, LubikanSelfConfig};
use world_player::Player as VegetationPlayer;

use crate::control::WorldGameplayEnabled;

pub(crate) fn configure(app: &mut App) {
	app.add_systems(
		Update,
		attach_lubikan_owner_to_world_players.run_if(resource_equals(WorldGameplayEnabled(true))),
	);
}

// TODO(#1107): replace with lubikans world registration when stacked on the #1107 branch.
pub(crate) fn attach_lubikan_owner_components(
	commands: &mut Commands,
	config: &LubikanSelfConfig,
	player: Entity,
) {
	commands
		.entity(player)
		.insert((LubikanOwner, LubikanLedger::with_self(config)));
}

fn attach_lubikan_owner_to_world_players(
	mut commands: Commands,
	config: Res<LubikanSelfConfig>,
	players: Query<Entity, (With<VegetationPlayer>, Without<LubikanOwner>)>,
) {
	for player in &players {
		attach_lubikan_owner_components(&mut commands, &*config, player);
	}
}
