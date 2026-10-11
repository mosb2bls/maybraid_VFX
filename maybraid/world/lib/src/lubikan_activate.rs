//! Map the player's activate intent onto [`LubikanActivationRequest`].

use bevy::prelude::*;
use game_commands::command::TextEntryFocus;
use lubikans::{LubikanActivationRequest, LubikanHealingEnabled, LubikanOwner, LubikanSystems};
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use world_player::Player;

use crate::control::WorldGameplayEnabled;
use crate::map_view::WorldMapView;

/// Default Self activation binding: pad **X** + fire (`PowerUseItem`).
pub(crate) fn configure(app: &mut App) {
	app.add_systems(
		Update,
		(
			sync_lubikan_healing_enabled,
			request_lubikan_activation
				.after(CharacterControlSystems)
				.before(LubikanSystems::Activate),
		),
	);
}

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
	use lubikans::LubikanInterfacePlugin;

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
	fn pause_does_not_request_activation_and_freezes_healing() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<LubikanActivationRequest>>();
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(LubikanHealingEnabled(true));
		world.spawn((Player, LubikanOwner));
		world.write_message(CharacterIntent::PowerUseItem);
		world
			.run_system_once(request_lubikan_activation)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_lubikan_healing_enabled)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let requests = world
			.run_system_once(|mut requests: MessageReader<LubikanActivationRequest>| {
				requests.read().count()
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(requests, 0);
		assert!(!world.resource::<LubikanHealingEnabled>().0);
		Ok(())
	}

	#[test]
	fn interface_plugin_is_enough_to_register_the_request() {
		let mut app = App::new();
		app.add_plugins(LubikanInterfacePlugin);
		assert!(app.world().get_resource::<Messages<LubikanActivationRequest>>().is_some());
	}
}
