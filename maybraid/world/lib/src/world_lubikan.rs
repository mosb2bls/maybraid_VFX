//! World Lubikan adapter: owner binding, session lifecycle, activation/healing routing.

use bevy::prelude::*;
use game_commands::command::TextEntryFocus;
use lubikans::{
	LubikanActivationRequest, LubikanHealingEnabled, LubikanOwner,
	LubikanOwnerLifetime, LubikanOwnerLifetimeKind, LubikansPlugin, LubikanSystems,
};
use lubikans_damage::LubikanDamagePlugin;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use player::{
	CharacterController, CharacterStance, LocomotionCapsule, MotorTraction, RestLocomotionCapsule, StanceKind,
};
use world_player::{Player, PlayerPhysicsEnabled};

use crate::control::{WorldGameplayEnabled, WorldPlaySessionEnded, apply_world_player_motor, strip_world_player_motor};
use crate::map_view::WorldMapView;

/// Adapter plugin connecting world player lifecycle, input, and session to Lubikan systems.
///
/// Owns:
/// - Owner binding: establishes player identity and LubikanOwner marker
/// - Session forwarding: emits LubikanOwnerLifetime on session teardown
/// - Activation routing: maps PowerUseItem intent to LubikanActivationRequest
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
		app.add_plugins((LubikansPlugin, LubikanDamagePlugin));
		
		app.add_systems(
			PreUpdate,
			(
				stamp_vegetation_motor_traction,
				stamp_world_player_motor,
				crate::control::grant_world_player_lubikan_owner.before(LubikanSystems::Owner),
				forward_world_session_end.before(LubikanSystems::Owner),
			),
		);
		
		app.add_systems(
			Update,
			(
				sync_lubikan_healing_enabled.before(LubikanSystems::Activate),
				request_lubikan_activation
					.after(CharacterControlSystems)
					.after(sync_lubikan_healing_enabled)
					.before(LubikanSystems::Activate),
			),
		);
	}
}

/// Vegetation capsule already has `ActiveCollisionHooks`; pair it with the shared motor marker.
fn stamp_vegetation_motor_traction(
	mut commands: Commands,
	players: Query<Entity, (With<Player>, Without<MotorTraction>)>,
) {
	for entity in &players {
		commands.entity(entity).insert(player::motor_traction_bundle());
	}
}

/// Put the world player on the player-crate capsule motor so column buoyancy runs.
fn stamp_world_player_motor(
	physics: Res<PlayerPhysicsEnabled>,
	mut commands: Commands,
	missing: Query<Entity, (With<Player>, Without<CharacterController>)>,
	present: Query<
		(
			Entity,
			Option<&CharacterStance>,
			Option<&RestLocomotionCapsule>,
			Option<&LocomotionCapsule>,
		),
		(With<Player>, With<CharacterController>),
	>,
) {
	if physics.0 {
		for entity in &missing {
			apply_world_player_motor(&mut commands, entity);
		}
		for (entity, stance, rest, live) in &present {
			if stance.is_none() {
				commands.entity(entity).insert(CharacterStance::settled(StanceKind::Stand));
			}
			if rest.is_none() {
				commands.entity(entity).insert(RestLocomotionCapsule(
					live.copied().unwrap_or(LocomotionCapsule::HUMANOID),
				));
			}
		}
	} else {
		for (entity, _, _, _) in &present {
			strip_world_player_motor(&mut commands, entity);
		}
	}
}

/// Removed - now in control.rs as public function for world player lifecycle.

/// Forward world session teardown to Lubikan lifetime events.
fn forward_world_session_end(
	mut ended: MessageReader<WorldPlaySessionEnded>,
	mut lives: MessageWriter<LubikanOwnerLifetime>,
) {
	if ended.read().next().is_none() {
		return;
	}
	lives.write(LubikanOwnerLifetime::new(
		None,
		None,
		LubikanOwnerLifetimeKind::RestartSession,
	));
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
	app.add_systems(
		Update,
		(
			sync_lubikan_healing_enabled.before(LubikanSystems::Activate),
			request_lubikan_activation
				.after(sync_lubikan_healing_enabled)
				.before(LubikanSystems::Activate),
		),
	);
}

/// Route PowerUseItem intent to equipped Lubikan activation.
fn request_lubikan_activation(
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	mut intents: MessageReader<CharacterIntent>,
	mut requests: MessageWriter<LubikanActivationRequest>,
	owners: Query<Entity, (With<Player>, With<LubikanOwner>)>,
) {
	if !gameplay.0 || text_focus.0 || map.is_some_and(|map| map.open) {
		for _ in intents.read() {}
		return;
	}
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::PowerUseItem)) {
		return;
	}
	for owner in &owners {
		requests.write(LubikanActivationRequest::equipped(owner));
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;

	#[test]
	fn power_use_item_requests_equipped_activation() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<LubikanActivationRequest>>();
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(true));
		let owner = world.spawn((Player, LubikanOwner)).id();
		world.write_message(CharacterIntent::PowerUseItem);
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
			.run_system_once(forward_world_session_end)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let count = world
			.run_system_once(|mut lives: MessageReader<LubikanOwnerLifetime>| {
				lives.read().count()
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(count, 1);
		Ok(())
	}
}
