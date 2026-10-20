//! Actor-addressed delivery. Broadcast queries are not a substitute.

use bevy::prelude::*;

/// One actor entity that should consume a sample or intent.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ActorTarget(Entity);

/// The local pad's actor. Ordinary locomotion writes motor samples to this
/// entity only. Hosts may insert it; otherwise the locomotion controller
/// picks the lowest-id `Player`.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalActor(pub ActorTarget);

impl LocalActor {
	pub const fn new(entity: Entity) -> Self {
		Self(ActorTarget::new(entity))
	}

	pub const fn target(self) -> ActorTarget {
		self.0
	}

	pub const fn entity(self) -> Entity {
		self.0.entity()
	}

	pub fn matches(self, entity: Entity) -> bool {
		self.0.matches(entity)
	}

	/// The installed local pad actor, or the lowest-id candidate when hosts
	/// have not inserted [`LocalActor`].
	pub fn resolve(
		local: Option<&Self>,
		candidates: impl IntoIterator<Item = Entity>,
	) -> Option<Entity> {
		if let Some(local) = local {
			return Some(local.entity());
		}
		candidates.into_iter().min_by_key(|entity| entity.to_bits())
	}
}

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

	#[test]
	fn local_actor_wraps_one_target() {
		let entity = Entity::from_bits(3);
		let local = LocalActor::new(entity);
		assert!(local.matches(entity));
		assert_eq!(local.entity(), entity);
		assert_eq!(local.target(), ActorTarget::new(entity));
	}

	#[test]
	fn resolve_prefers_the_installed_local_actor() {
		let local = Entity::from_bits(9);
		let other = Entity::from_bits(1);
		assert_eq!(LocalActor::resolve(Some(&LocalActor::new(local)), [other, local]), Some(local));
		assert_eq!(LocalActor::resolve(None, [local, other]), Some(other));
	}
}
