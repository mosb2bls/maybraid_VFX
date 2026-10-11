//! Thin fixture matching the shared Lubikan interface owned by
//! [#1107](https://github.com/ramate-io/maybraid/issues/1107).
//!
//! This module snapshots **account identity**, **equipped selection**, and
//! **owner lifetime** so damage attribution can name a charge-credit recipient
//! without owning charge mutation, activation, or selection. Replace these
//! types with the real `lubikan` exports when that lane lands. Do not grow a
//! private charge model here.
//!
//! Marked fixture: no Bikanwarda types, no HUD, no visuals.

use bevy::prelude::*;

use crate::DamageActorId;

/// Stable Lubikan account. Survives equip changes, body replacement, and
/// session-scoped selection; it does not survive an explicit session reset.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LubikanAccountId(u64);

impl LubikanAccountId {
	pub const fn new(bits: u64) -> Self {
		Self(bits)
	}

	pub const fn to_bits(self) -> u64 {
		self.0
	}
}

/// Equipped definition. [`LubikanDefinitionId::SELF`] is the null Lubikan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LubikanDefinitionId(u64);

impl LubikanDefinitionId {
	pub const SELF: Self = Self(0);

	pub const fn new(bits: u64) -> Self {
		Self(bits)
	}

	pub const fn to_bits(self) -> u64 {
		self.0
	}
}

/// Owner body generation for one account.
///
/// `generation` changes when the owner body is replaced. Charge-credit
/// attribution keeps the account; activation-scoped lookback uses the
/// generation so a new body does not inherit an old healing session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanOwnerLifetime {
	pub account: LubikanAccountId,
	pub generation: u64,
}

impl LubikanOwnerLifetime {
	pub fn from_body(account: LubikanAccountId, body: Entity) -> Self {
		Self { account, generation: body.to_bits() }
	}

	pub fn matches_body(self, body: Entity) -> bool {
		self.generation == body.to_bits()
	}
}

/// Equipped selection snapshotted at event resolution.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanAttribution {
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub owner: LubikanOwnerLifetime,
}

impl LubikanAttribution {
	pub fn new(
		account: LubikanAccountId,
		definition: LubikanDefinitionId,
		owner: LubikanOwnerLifetime,
	) -> Self {
		Self { account, definition, owner }
	}

	pub fn from_binding(binding: LubikanAccountBinding) -> Self {
		Self { account: binding.account, definition: binding.definition, owner: binding.lifetime }
	}

	pub fn is_self(self) -> bool {
		self.definition == LubikanDefinitionId::SELF
	}
}

/// Fixture stand-in for the #1107 equipped binding on an owner or summon.
///
/// Stamp this at bind time so attribution still names the account after the
/// owner despawns. History reads it; it does not mutate charge.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanAccountBinding {
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub lifetime: LubikanOwnerLifetime,
}

impl LubikanAccountBinding {
	pub fn new(
		account: LubikanAccountId,
		definition: LubikanDefinitionId,
		lifetime: LubikanOwnerLifetime,
	) -> Self {
		Self { account, definition, lifetime }
	}

	/// Self binding for a persistent account on the current owner body.
	pub fn self_account(account: LubikanAccountId, body: Entity) -> Self {
		Self::new(
			account,
			LubikanDefinitionId::SELF,
			LubikanOwnerLifetime::from_body(account, body),
		)
	}

	/// Body-scoped Self fallback when no persistent account exists yet.
	/// Prefer [`Self::self_account`] once #1107 assigns a durable id.
	pub fn self_on(body: Entity) -> Self {
		Self::self_account(LubikanAccountId::new(DamageActorId::from_entity(body).to_bits()), body)
	}

	pub fn attribution(self) -> LubikanAttribution {
		LubikanAttribution::from_binding(self)
	}
}
