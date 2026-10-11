//! Durable actor identity and the summon-owner attribution seam.

use bevy::prelude::*;

/// Generation-aware identity snapshotted from a live [`Entity`].
///
/// Bevy recycles entity indices; these bits keep the original generation so a
/// later occupant of the same index cannot match a retired actor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DamageActorId(u64);

impl DamageActorId {
	pub const fn from_bits(bits: u64) -> Self {
		Self(bits)
	}

	pub fn from_entity(entity: Entity) -> Self {
		Self(entity.to_bits())
	}

	pub const fn to_bits(self) -> u64 {
		self.0
	}

	/// Handle recorded at snapshot time. It may no longer be alive.
	pub fn recorded_entity(self) -> Entity {
		Entity::from_bits(self.0)
	}
}

/// Responsible party when it is not the live attacker.
///
/// Direct [`crate::Hit::source`] is the entity that connected. This component
/// names the owner that should be attributed after despawn — for example a
/// summon's summoner. Missing means the source is the responsible owner.
///
/// Store this on the attacker (or summon) at bind time. History snapshots the
/// handle's generation bits, so a later recycled entity does not inherit it.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResponsibleOwner {
	pub actor: Entity,
}

impl ResponsibleOwner {
	pub fn new(actor: Entity) -> Self {
		Self { actor }
	}

	pub fn id(self) -> DamageActorId {
		DamageActorId::from_entity(self.actor)
	}
}
