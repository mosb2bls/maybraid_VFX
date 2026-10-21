//! Gameplay assembly: session, equipment, and cross-domain adapters.
//! Adapters translate or coordinate; they hold no domain state.
//! Firearm / grenade runtimes stay on the item plugins (world intelligence
//! in production; stack tests register them explicitly).

pub mod actor_lifecycle;
pub mod equipment;
pub mod powers;
pub mod schedule;

use bevy::prelude::*;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_skill_map::SkillMapPlugin;

pub use actor_lifecycle::ActorDomainTeardown;
pub use maybraid_session::{
	apply_in_game_menu_pause, ActorRetired, InputRouter, InputRouting, Participation, RoutingEvent,
	SessionBit, SessionCapabilities, SessionCapabilitySystems, SessionLifecycleSystems,
	SessionPlugin, WorldGameplayEnabled, WorldPlaySessionEnded,
};

/// Headless-safe session + equipment + adapter registration shared by
/// [`GameplayPlugin`] and stack-tip tests. Item clocks stay on
/// `register_*_user_runtime`; rendering / audio stay on the full item plugins.
pub fn register_gameplay_runtime(app: &mut App) {
	if !app.is_plugin_added::<CharacterControllerPlugin>() {
		app.add_plugins(CharacterControllerPlugin);
	}
	if !app.is_plugin_added::<SessionPlugin>() {
		app.add_plugins(SessionPlugin);
	}
	if !app.is_plugin_added::<lubikans::LubikansPlugin>() {
		app.add_plugins(lubikans::LubikansPlugin);
	}
	if !app.is_plugin_added::<lubikans_damage::LubikanDamagePlugin>() {
		app.add_plugins(lubikans_damage::LubikanDamagePlugin);
	}
	schedule::configure(app);
	equipment::configure(app);
	powers::configure(app);
	actor_lifecycle::configure(app);
}

/// Assembles session, equipment, and adapters. World keeps terrain, POI, and
/// collision-support readiness. Game adds this beside World and the shell.
pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
	fn build(&self, app: &mut App) {
		register_gameplay_runtime(app);
		if !app.is_plugin_added::<SkillMapPlugin>() {
			app.add_plugins(SkillMapPlugin);
		}
	}
}

#[cfg(test)]
mod controller_stack;
