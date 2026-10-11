use bevy::prelude::*;

use crate::AttackId;

/// Start an authored attack on an owner that already has [`crate::AttackUser`].
#[derive(Message, Clone, Debug)]
pub struct BeginAttack {
	pub owner: Entity,
	pub definition: AttackId,
	/// Optional explicit victim. Missing falls back to targeting membership.
	pub target: Option<Entity>,
	pub allies: Vec<Entity>,
}

impl BeginAttack {
	pub fn new(owner: Entity, definition: AttackId) -> Self {
		Self { owner, definition, target: None, allies: Vec::new() }
	}

	pub fn with_target(mut self, target: Entity) -> Self {
		self.target = Some(target);
		self
	}

	pub fn with_allies(mut self, allies: Vec<Entity>) -> Self {
		self.allies = allies;
		self
	}
}

/// Request teardown of one instance or every live instance on an owner.
#[derive(Message, Clone, Copy, Debug)]
pub struct CancelAttacks {
	pub owner: Entity,
	pub instance: Option<crate::AttackInstanceId>,
	pub reason: AttackEndReason,
}

impl CancelAttacks {
	pub fn all(owner: Entity) -> Self {
		Self { owner, instance: None, reason: AttackEndReason::Cancelled }
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AttackEndReason {
	Completed,
	Cancelled,
	OwnerDowned,
	OwnerSuspended,
	OwnerRetired,
	OwnerDespawned,
}

/// Gameplay-facing completion. Encounter scripts listen here; VFX uses presentation.
#[derive(Message, Clone, Copy, Debug)]
pub struct AttackEnded {
	pub instance: crate::AttackInstanceId,
	pub owner: Entity,
	pub definition: AttackId,
	pub reason: AttackEndReason,
}
