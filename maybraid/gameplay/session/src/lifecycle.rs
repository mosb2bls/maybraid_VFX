//! Session start / end. Pause must not write teardown.

use bevy::prelude::*;

/// Explicit play-session teardown. The game shell writes this when leaving
/// to Home. Pause must not write it.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldPlaySessionEnded;

/// A host retired this actor. Gameplay coordinates domain cleanup after
/// hosts persist loadout and strip world-owned markers.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActorRetired {
	entity: Entity,
}

impl ActorRetired {
	pub const fn new(entity: Entity) -> Self {
		Self { entity }
	}

	pub const fn entity(self) -> Entity {
		self.entity
	}
}

/// Hosts write [`ActorRetired`] in [`Self::Notify`]. Gameplay coordinates
/// domain cleanup in [`Self::Coordinate`].
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SessionLifecycleSystems {
	Notify,
	Coordinate,
}
