//! Host assignment boundary: lowest-ID [`Player`] once, never from controllers.

use bevy::prelude::*;
use maybraid_controllers_core::{InputRouter, InputSource};

use crate::identity::Player;

/// Resolve the local pad's actor at the host boundary.
///
/// Keeps a live assignment. If that player is gone, clears delivery (removal
/// effects). If unassigned, picks the lowest-ID [`Player`] once. Controllers
/// must read [`InputRouter::actor`] instead of rediscovering a player.
pub(crate) fn assign_local_player(
	mut router: ResMut<InputRouter>,
	players: Query<Entity, With<Player>>,
) {
	let source = InputSource::local_pad();
	router.retain_assigned_actor(source, players.iter());
	router.assign_lowest_id_actor(source, players.iter());
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use maybraid_controllers_core::ActorTarget;

	#[test]
	fn host_assigns_lowest_id_player_once() {
		let mut world = World::new();
		world.init_resource::<InputRouter>();
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
}
