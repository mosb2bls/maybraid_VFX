//! UI-addressed delivery. One focused menu, not every child.

use bevy::prelude::*;

/// One UI entity that should consume a nav sample or capture a source.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UiTarget(Entity);

impl UiTarget {
	pub const fn new(entity: Entity) -> Self {
		Self(entity)
	}

	pub const fn entity(self) -> Entity {
		self.0
	}

	pub fn matches(self, entity: Entity) -> bool {
		self.0 == entity
	}

	pub fn is_under(self, root: Entity, children: &Query<&Children>) -> bool {
		self.0 == root || children.iter_descendants(root).any(|entity| entity == self.0)
	}
}

impl From<Entity> for UiTarget {
	fn from(entity: Entity) -> Self {
		Self::new(entity)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn ui_target_is_entity_local() {
		let entity = Entity::from_bits(3);
		let target = UiTarget::new(entity);
		assert!(target.matches(entity));
		assert!(!target.matches(Entity::from_bits(4)));
		assert_eq!(target.entity(), entity);
	}
}
