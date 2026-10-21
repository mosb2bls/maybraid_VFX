//! Actor-addressed delivery. Broadcast queries are not a substitute.

use bevy::prelude::*;

/// One actor entity that should consume a sample or intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorTarget(Entity);

impl ActorTarget {
	pub const fn new(entity: Entity) -> Self {
		Self(entity)
	}

	pub const fn entity(self) -> Entity {
		self.0
	}

	pub fn matches(self, entity: Entity) -> bool {
		self.0 == entity
	}
}

impl From<Entity> for ActorTarget {
	fn from(entity: Entity) -> Self {
		Self::new(entity)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn actor_target_is_entity_local() {
		let entity = Entity::from_bits(7);
		let target = ActorTarget::new(entity);
		assert!(target.matches(entity));
		assert!(!target.matches(Entity::from_bits(8)));
		assert_eq!(target.entity(), entity);
	}
}
