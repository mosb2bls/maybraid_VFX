//! Held grenade, overhand throw, and owned recharge clocks.
//!
//! Execution lives in [`runtime`]; HUD / audio / held-mesh live in
//! [`presentation`].

mod cook_hud;
mod feel;
mod hold;
mod presentation;
mod runtime;
mod throw;

use bevy::prelude::*;

pub use feel::{blast_presence, blast_rumble, rumble_presence, GrenadeBlastFeel};
pub use hold::{spawn_held_grenade, HeldGrenade, HoldingGrenade};
pub use presentation::register_grenade_user_presentation;
pub use runtime::{register_grenade_user_runtime, teardown_held_grenade};
pub use throw::{
	apply_throw_intents, apply_throw_routing, landing_range, landing_time, launch_velocity,
	throw_aim, GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings, GrenadeUserSystems,
	STANDING_RELEASE_HEIGHT, THROW_GRAVITY,
};

/// Held-side 1:1 target of [`GrenadeUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = GrenadeUser)]
pub struct HeldByGrenade(Entity);

pub struct GrenadeUserPlugin;

impl Plugin for GrenadeUserPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<grenades::GrenadesPlugin>() {
			app.add_plugins(grenades::GrenadesPlugin);
		}
		if !app.is_plugin_added::<maybraid_spatial_rumble::SpatialRumblePlugin>() {
			app.add_plugins(maybraid_spatial_rumble::SpatialRumblePlugin);
		}
		register_grenade_user_runtime(app);
		register_grenade_user_presentation(app);
	}
}
