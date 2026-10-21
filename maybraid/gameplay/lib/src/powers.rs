//! Equipped power use → Lubikan activation. Connects equipment to Lubikan
//! without either depending on world.

use bevy::prelude::*;
use lubikans::{LubikanActivationRequest, LubikanHealingEnabled, LubikanOwner, LubikanSystems};
use maybraid_character_controller::CharacterControlSystems;
use maybraid_controllers_core::{InputRouter, InputSource};
use maybraid_equipment::EquippedPowerUse;
use maybraid_session::{SessionCapabilitySystems, WorldGameplayEnabled};

/// Route addressed [`EquippedPowerUse`] to equipped Lubikan activation and
/// gate healing with the session gameplay projection.
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

/// Pause gates healing. Capture (console / map overlay) is not needed here.
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

/// Route addressed PowerUseItem to equipped Lubikan activation.
fn request_lubikan_activation(
	gameplay: Res<WorldGameplayEnabled>,
	router: Option<Res<InputRouter>>,
	mut uses: MessageReader<EquippedPowerUse>,
	mut requests: MessageWriter<LubikanActivationRequest>,
	owners: Query<Entity, With<LubikanOwner>>,
) {
	let blocked = router.is_some_and(|router| router.blocks_actor(InputSource::local_pad()));
	if !gameplay.accepts_character_input(blocked, false) {
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
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_character_controller::ActorTarget;

	#[test]
	fn power_use_item_requests_equipped_activation() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<EquippedPowerUse>>();
		world.init_resource::<Messages<LubikanActivationRequest>>();
		world.insert_resource(WorldGameplayEnabled(true));
		let owner = world.spawn(LubikanOwner).id();
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
}
