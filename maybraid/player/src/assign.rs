//! Host assignment boundary: lowest-ID [`Player`] once, never from controllers.

use bevy::prelude::*;
use maybraid_controllers_core::{InputRouting, InputSource};

use crate::identity::Player;

/// Resolve the local pad's actor at the host boundary.
///
/// Keeps a live assignment while the source is connected. If that player is
/// gone, clears delivery (removal effects). If connected and unassigned, picks
/// the lowest-ID [`Player`] once. Disconnect stays disconnected until the
/// source lifecycle calls [`InputRouter::connect`]. Controllers must read
/// [`InputRouter::actor`] instead of rediscovering a player.
pub(crate) fn assign_local_player(mut router: InputRouting, players: Query<Entity, With<Player>>) {
	let source = InputSource::local_pad();
	router.retain_assigned_actor(source, players.iter());
	router.assign_lowest_id_actor(source, players.iter());
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_controllers_core::{ActorTarget, InputRouter, RoutingEvent, SourceConnection};

	fn world_with_router() -> World {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
		world.init_resource::<Messages<RoutingEvent>>();
		world
	}

	#[test]
	fn host_assigns_lowest_id_player_once() {
		let mut world = world_with_router();
		let first = world.spawn(Player).id();
		let second = world.spawn(Player).id();
		let lowest = [first, second]
			.into_iter()
			.min_by_key(|entity| entity.to_bits())
			.expect("players");
		world.run_system_once(assign_local_player).expect("assign");
		assert_eq!(world.resource::<InputRouter>().local_actor(), Some(ActorTarget::new(lowest)));

		world.spawn(Player);
		world.run_system_once(assign_local_player).expect("retain");
		assert_eq!(world.resource::<InputRouter>().local_actor(), Some(ActorTarget::new(lowest)));

		world.entity_mut(lowest).despawn();
		let next = world
			.query_filtered::<Entity, With<Player>>()
			.iter(&world)
			.min_by_key(|entity| entity.to_bits())
			.expect("remaining player");
		world.run_system_once(assign_local_player).expect("reassign after loss");
		assert_eq!(world.resource::<InputRouter>().local_actor(), Some(ActorTarget::new(next)));

		let remaining: Vec<Entity> =
			world.query_filtered::<Entity, With<Player>>().iter(&world).collect();
		for entity in remaining {
			world.entity_mut(entity).despawn();
		}
		world.run_system_once(assign_local_player).expect("clear");
		assert_eq!(world.resource::<InputRouter>().local_actor(), None);
	}

	#[test]
	fn player_plugin_does_not_undo_disconnect() {
		use crate::PlayerPlugin;
		use bevy::input::InputPlugin;
		use maybraid_audio::AudioClip;
		use maybraid_character_controller::CharacterControllerPlugin;

		let mut app = App::new();
		app.add_plugins((
			MinimalPlugins,
			AssetPlugin::default(),
			InputPlugin,
			CharacterControllerPlugin,
			PlayerPlugin,
		))
		.init_asset::<AudioClip>();
		let player = app.world_mut().spawn(Player).id();
		app.update();
		assert_eq!(
			app.world().resource::<InputRouter>().local_actor(),
			Some(ActorTarget::new(player))
		);

		app.world_mut()
			.resource_mut::<InputRouter>()
			.disconnect(InputSource::local_pad());
		app.update();
		let router = app.world().resource::<InputRouter>();
		assert_eq!(router.connection(InputSource::local_pad()), SourceConnection::Disconnected);
		assert_eq!(router.local_actor(), None);

		app.world_mut().resource_mut::<InputRouter>().connect(InputSource::local_pad());
		app.update();
		assert_eq!(
			app.world().resource::<InputRouter>().local_actor(),
			Some(ActorTarget::new(player))
		);
	}
}
