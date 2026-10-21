//! World-dependent Lubikan setup: owner binding and session-end forwarding.
//!
//! Input-to-activation routing lives in `maybraid-gameplay` `powers`.

use bevy::prelude::*;
use lubikans::{LubikanSystems, LubikansPlugin};
use lubikans_damage::LubikanDamagePlugin;

use crate::control::{grant_world_player_lubikan_owner, sync_lubikan_session};

/// World player identity and session-end notices for Lubikan systems.
///
/// Owns:
/// - Owner binding: establishes player identity and LubikanOwner marker
/// - Session forwarding: emits LubikanOwnerLifetime on session teardown
///
/// Activation routing and healing gates are gameplay adapters.
pub struct WorldLubikanPlugin;

impl Plugin for WorldLubikanPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<LubikansPlugin>() {
			app.add_plugins(LubikansPlugin);
		}
		if !app.is_plugin_added::<LubikanDamagePlugin>() {
			app.add_plugins(LubikanDamagePlugin);
		}

		app.add_systems(
			PreUpdate,
			(
				grant_world_player_lubikan_owner.before(LubikanSystems::Owner),
				sync_lubikan_session.before(LubikanSystems::Owner),
			),
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::control::WorldPlaySessionEnded;
	use bevy::ecs::system::RunSystemOnce;
	use lubikans::LubikanOwnerLifetime;

	#[test]
	fn session_end_emits_lifetime() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<WorldPlaySessionEnded>>();
		world.init_resource::<Messages<LubikanOwnerLifetime>>();
		world.write_message(WorldPlaySessionEnded);
		world
			.run_system_once(sync_lubikan_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let count = world
			.run_system_once(|mut lives: MessageReader<LubikanOwnerLifetime>| lives.read().count())
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(count, 1);
		Ok(())
	}
}
