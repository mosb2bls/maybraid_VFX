//! World Lubikan adapter: owner binding, session lifecycle, activation/healing routing.

use bevy::prelude::*;
use game_commands::command::TextEntryFocus;
use lubikans::{
	LubikanActivationRequest, LubikanHealingEnabled, LubikanOwner, LubikanSystems, LubikansPlugin,
};
use lubikans_damage::LubikanDamagePlugin;
use maybraid_character_controller::{CharacterControlSystems, EquippedPowerUse};
use maybraid_world_controller::SessionCapabilitySystems;
use world_player::Player;

use crate::control::{
	grant_world_player_lubikan_owner, sync_lubikan_session, WorldGameplayEnabled,
};
use crate::map_view::WorldMapView;

/// Adapter plugin connecting world player lifecycle, input, and session to Lubikan systems.
///
/// Owns:
/// - Owner binding: establishes player identity and LubikanOwner marker
/// - Session forwarding: emits LubikanOwnerLifetime on session teardown
/// - Activation routing: maps addressed [`EquippedPowerUse`] to [`LubikanActivationRequest`]
/// - Healing sync: gates healing with WorldGameplayEnabled
///
/// Player identity (generic player::Player marker and LubikanOwner) is established
/// in the motor setup lifecycle. The motor is applied when the world player spawns
/// and persists through pause (gameplay off, physics on). On respawn after death,
/// the motor and markers are reapplied.
///
/// ## Schedule Ordering
///
/// Intra-schedule constraints only. A same-named set in another schedule does
/// not order these systems.
///
/// | Schedule   | Stage      | Systems |
/// |------------|------------|---------|
/// | PreUpdate  | Owner      | grant / session-end before `LubikanSystems::Owner` |
/// | Update     | Gate+Act   | `sync_lubikan_healing_enabled` then activation request before `LubikanSystems::Activate` |
/// | PostUpdate | Heal       | `LubikanSystems::Heal` after damage history / awards; HUD after Heal |
pub struct WorldLubikanPlugin;

impl Plugin for WorldLubikanPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins((LubikansPlugin, LubikanDamagePlugin))
			.add_message::<EquippedPowerUse>();

		app.add_systems(
			PreUpdate,
			(
				grant_world_player_lubikan_owner.before(LubikanSystems::Owner),
				sync_lubikan_session.before(LubikanSystems::Owner),
			),
		);

		configure(app);
	}
}

/// Sync Lubikan healing enabled state with world gameplay enabled (pause gates healing).
fn sync_lubikan_healing_enabled(
	gameplay: Res<WorldGameplayEnabled>,
	healing: Option<ResMut<LubikanHealingEnabled>>,
) {
	let Some(mut healing) = healing else {
		return;
	};
	if healing.0 != gameplay.0 {
		healing.0 = gameplay.0;
	}
}

/// Configure world lubikan routing systems for headless tests.
/// Registers activation routing and healing sync, matching the old lubikan_activate.rs behavior.
pub fn configure(app: &mut App) {
	app.add_message::<EquippedPowerUse>().add_systems(
		Update,
		(
			sync_lubikan_healing_enabled.before(LubikanSystems::Activate),
			request_lubikan_activation
				.after(CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.after(sync_lubikan_healing_enabled)
				.before(LubikanSystems::Activate),
		),
	);
}

/// Route addressed PowerUseItem to equipped Lubikan activation.
fn request_lubikan_activation(
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	mut uses: MessageReader<EquippedPowerUse>,
	mut requests: MessageWriter<LubikanActivationRequest>,
	owners: Query<Entity, (With<Player>, With<LubikanOwner>)>,
) {
	if !gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open)) {
		for _ in uses.read() {}
		return;
	}
	for use_item in uses.read() {
		if owners.contains(use_item.actor().entity()) {
			requests.write(LubikanActivationRequest::equipped(use_item.actor().entity()));
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::control::WorldPlaySessionEnded;
	use bevy::ecs::system::RunSystemOnce;
	use lubikans::LubikanOwnerLifetime;

	#[test]
	fn power_use_item_requests_equipped_activation() -> anyhow::Result<()> {
		use maybraid_character_controller::ActorTarget;

		let mut world = World::new();
		world.init_resource::<Messages<EquippedPowerUse>>();
		world.init_resource::<Messages<LubikanActivationRequest>>();
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(true));
		let owner = world.spawn((Player, LubikanOwner)).id();
		world.write_message(EquippedPowerUse::new(ActorTarget::new(owner)));
		world
			.run_system_once(request_lubikan_activation)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let request = world
			.run_system_once(|mut requests: MessageReader<LubikanActivationRequest>| {
				requests.read().next().copied()
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?
			.ok_or_else(|| anyhow::anyhow!("activation request"))?;
		assert_eq!(request, LubikanActivationRequest::equipped(owner));
		Ok(())
	}

	#[test]
	fn gameplay_off_freezes_healing() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(LubikanHealingEnabled(true));
		world
			.run_system_once(sync_lubikan_healing_enabled)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<LubikanHealingEnabled>().0);
		Ok(())
	}

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
