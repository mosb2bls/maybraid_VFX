//! Firearm execution contract: armed flag, [`WeaponFired`], and cadence types.
//!
//! Shot spawn / muzzle / impact stay in [`crate::presentation`].
//! [`crate::projectiles::fire_weapons`] still advances its own clock while
//! spawning. A follow-up will make one authoritative fire clock (the headless
//! `realize_held_weapon_fire` path or a shared cadence tick) and have
//! presentation consume [`WeaponFired`].

use bevy::prelude::*;
use damage::DamagePlugin;
use projectiles::ProjectilesPlugin;

use crate::cadence::WeaponFired;
use crate::projectiles::WeaponsArmed;

#[derive(Resource, Default)]
struct FirearmRuntime;

/// Messages and armed flag. Does not spawn meshes or register `fire_weapons`.
pub fn register_firearm_runtime(app: &mut App) {
	if app.world().contains_resource::<FirearmRuntime>() {
		return;
	}
	if !app.is_plugin_added::<ProjectilesPlugin>() {
		app.add_plugins(ProjectilesPlugin);
	}
	if !app.is_plugin_added::<DamagePlugin>() {
		app.add_plugins(DamagePlugin);
	}
	app.init_resource::<FirearmRuntime>()
		.init_resource::<WeaponsArmed>()
		.add_message::<WeaponFired>();
}
