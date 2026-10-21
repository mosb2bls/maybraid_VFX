//! Gameplay assembly: session, equipment, item runtimes, and cross-domain
//! adapters. Adapters translate or coordinate; they hold no domain state.

pub mod actor_lifecycle;
pub mod equipment;
pub mod powers;
pub mod schedule;

use bevy::prelude::*;
use firearm_user::FirearmUserPlugin;
use grenade_user::GrenadeUserPlugin;
use maybraid_character_controller::CharacterControllerPlugin;
use maybraid_skill_map::SkillMapPlugin;

pub use actor_lifecycle::ActorDomainTeardown;
pub use equipment::ItemRuntimes;
pub use maybraid_session::{
	ActorRetired, InputRouter, InputRouting, Participation, RoutingEvent, SessionBit,
	SessionCapabilities, SessionCapabilitySystems, SessionLifecycleSystems, SessionPlugin,
	WorldGameplayEnabled, WorldPlaySessionEnded, apply_in_game_menu_pause,
};

#[derive(Resource, Default)]
struct GameplayRuntime;

/// Headless-safe session + item-runtime registration shared by
/// [`GameplayPlugin`] and stack-tip tests. Rendering / audio stay on the
/// full item plugins added by [`GameplayPlugin`].
///
/// Idempotent: a second call, or a later item-plugin `register_*_runtime`,
/// does not add systems again.
pub fn register_gameplay_runtime(app: &mut App) {
	if app.world().contains_resource::<GameplayRuntime>() {
		return;
	}
	app.init_resource::<GameplayRuntime>();
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

/// Assembles session, equipment, and item-domain runtimes. World keeps terrain,
/// POI, and collision-support readiness. Game adds this beside World and the shell.
///
/// Item *runtimes* are registered here (and only here in the assembled game).
/// Item *presentation* plugins are added after the runtime so either install
/// order is safe.
pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
	fn build(&self, app: &mut App) {
		register_gameplay_runtime(app);
		if !app.is_plugin_added::<SkillMapPlugin>() {
			app.add_plugins(SkillMapPlugin);
		}
		if !app.is_plugin_added::<FirearmUserPlugin>() {
			app.add_plugins(FirearmUserPlugin);
		}
		if !app.is_plugin_added::<GrenadeUserPlugin>() {
			app.add_plugins(GrenadeUserPlugin);
		}
	}
}

#[cfg(test)]
mod controller_stack;

#[cfg(test)]
mod tests {
	use super::*;
	use firearm_user::register_firearm_user_runtime;
	use grenade_user::register_grenade_user_runtime;

	fn headless() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins);
		app
	}

	#[test]
	fn register_gameplay_runtime_is_idempotent() {
		let mut app = headless();
		register_gameplay_runtime(&mut app);
		register_gameplay_runtime(&mut app);
	}

	#[test]
	fn item_runtime_after_gameplay_is_idempotent() {
		let mut app = headless();
		register_gameplay_runtime(&mut app);
		register_firearm_user_runtime(&mut app);
		register_grenade_user_runtime(&mut app);
	}

	#[test]
	fn gameplay_after_item_runtime_is_idempotent() {
		let mut app = headless();
		register_firearm_user_runtime(&mut app);
		register_grenade_user_runtime(&mut app);
		register_gameplay_runtime(&mut app);
	}
}
