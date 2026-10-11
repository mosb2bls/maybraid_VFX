//! Per-owner owned accounts, charge, and equipped selection.

use bevy::prelude::*;

use crate::activation::{LubikanActivation, LubikanActivationDenied, LubikanReadiness};
use crate::catalog::{LubikanCatalog, LubikanSelfConfig};
use crate::charge::{LubikanCharge, LubikanChargeMutation};
use crate::identity::{LubikanAccountId, LubikanDefinition, LubikanDefinitionId};
use crate::lifetime::{LubikanLifetimePolicy, LubikanOwnerLifetimeKind, LubikanPersistence};
use crate::selection::LubikanSelection;

/// Marker on a body that owns a [`LubikanLedger`].
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LubikanOwner;

/// Equipped account snapshotted at bind or event resolution.
///
/// Damage history and summons stamp this so attribution can name a charge
/// recipient after the owner despawns. It does not mutate charge.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanAccountBinding {
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub owner: Entity,
}

impl LubikanAccountBinding {
	pub fn new(account: LubikanAccountId, definition: LubikanDefinitionId, owner: Entity) -> Self {
		Self { account, definition, owner }
	}

	/// Self binding for a persistent account on the current owner body.
	pub fn self_account(account: LubikanAccountId, owner: Entity) -> Self {
		Self::new(account, LubikanDefinitionId::SELF, owner)
	}

	/// Body-scoped Self fallback when no other account is equipped.
	pub fn self_on(owner: Entity) -> Self {
		Self::self_account(LubikanAccountId::SELF, owner)
	}

	pub fn from_selection(selection: crate::LubikanSelection, owner: Entity) -> Self {
		let account = selection.resolved();
		Self::new(account, account.definition(), owner)
	}

	pub fn is_self(self) -> bool {
		self.definition.is_self()
	}

	/// Body generation token. A replacement body has a different entity id.
	pub fn generation(self) -> u64 {
		self.owner.to_bits()
	}

	pub fn matches_body(self, body: Entity) -> bool {
		self.owner == body
	}
}

/// One owned charge account.
#[derive(Clone, Debug, PartialEq)]
pub struct LubikanAccount {
	id: LubikanAccountId,
	charge: LubikanCharge,
	activation: LubikanActivation,
}

impl LubikanAccount {
	pub fn new(definition: LubikanDefinition) -> Self {
		let id = LubikanAccountId::unique(definition.id);
		Self {
			id,
			charge: LubikanCharge::empty(definition.capacity),
			activation: LubikanActivation::idle(id),
		}
	}

	pub fn with_id(id: LubikanAccountId, definition: LubikanDefinition) -> Self {
		Self {
			id,
			charge: LubikanCharge::empty(definition.capacity),
			activation: LubikanActivation::idle(id),
		}
	}

	pub fn id(&self) -> LubikanAccountId {
		self.id
	}

	pub fn charge(&self) -> LubikanCharge {
		self.charge
	}

	pub fn charge_mut(&mut self) -> &mut LubikanCharge {
		&mut self.charge
	}

	pub fn activation(&self) -> LubikanActivation {
		self.activation
	}

	pub fn activation_mut(&mut self) -> &mut LubikanActivation {
		&mut self.activation
	}
}

/// Owned Lubikan accounts for one owner. Self is always present and cannot
/// be removed or merged into another account.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct LubikanLedger {
	accounts: Vec<LubikanAccount>,
	selection: LubikanSelection,
}

impl LubikanLedger {
	pub fn with_self(config: &LubikanSelfConfig) -> Self {
		Self {
			accounts: vec![LubikanAccount::new(config.self_definition())],
			selection: LubikanSelection::default(),
		}
	}

	pub fn selection(&self) -> LubikanSelection {
		self.selection
	}

	pub fn self_account(&self) -> &LubikanAccount {
		self.accounts
			.iter()
			.find(|account| account.id.is_self())
			.unwrap_or(&self.accounts[0])
	}

	pub fn contains(&self, id: LubikanAccountId) -> bool {
		self.accounts.iter().any(|account| account.id == id)
	}

	pub fn account(&self, id: LubikanAccountId) -> Option<&LubikanAccount> {
		self.accounts.iter().find(|account| account.id == id)
	}

	pub fn account_mut(&mut self, id: LubikanAccountId) -> Option<&mut LubikanAccount> {
		self.accounts.iter_mut().find(|account| account.id == id)
	}

	pub fn equipped(&self) -> &LubikanAccount {
		let resolved = self.selection.resolved();
		self.account(resolved).unwrap_or_else(|| self.self_account())
	}

	pub fn equipped_mut(&mut self) -> &mut LubikanAccount {
		let resolved = self.selection.resolved();
		let index = self
			.accounts
			.iter()
			.position(|account| account.id == resolved)
			.or_else(|| self.accounts.iter().position(|account| account.id.is_self()))
			.unwrap_or(0);
		&mut self.accounts[index]
	}

	pub fn iter(&self) -> impl Iterator<Item = &LubikanAccount> {
		self.accounts.iter()
	}

	/// Equip `account` when it is owned. `None` returns to Self. Unknown
	/// accounts are left unchanged and return `false`.
	pub fn select(&mut self, account: Option<LubikanAccountId>) -> bool {
		match account {
			None => {
				self.selection.clear_to_self();
				true
			}
			Some(id) if self.contains(id) => {
				self.selection.select(id);
				true
			}
			Some(_) => false,
		}
	}

	/// Unlock or refresh an owned account for `definition`. Existing charge
	/// is kept when the account already exists; capacity is retargeted.
	pub fn ensure_owned(&mut self, definition: LubikanDefinition) -> LubikanAccountId {
		let id = LubikanAccountId::unique(definition.id);
		if let Some(existing) = self.account_mut(id) {
			existing.charge.set_capacity(definition.capacity);
			id
		} else {
			self.accounts.push(LubikanAccount::with_id(id, definition));
			id
		}
	}

	pub fn apply_to(&mut self, id: LubikanAccountId, delta: f32) -> Option<LubikanChargeMutation> {
		self.account_mut(id).map(|account| account.charge.apply(delta))
	}

	pub fn apply_to_equipped(&mut self, delta: f32) -> LubikanChargeMutation {
		self.equipped_mut().charge.apply(delta)
	}

	pub fn readiness(&self, catalog: &LubikanCatalog) -> LubikanReadiness {
		let equipped = self.equipped();
		LubikanReadiness::from_charge(
			equipped.id,
			equipped.charge,
			catalog.activation_cost(equipped.id.definition()),
			equipped.activation.is_busy(),
		)
	}

	/// Begin activation when the resolved account can afford it.
	///
	/// Does not spend charge; [#1110](https://github.com/ramate-io/maybraid/issues/1110)
	/// consumes cost through [`Self::apply_to`] / [`crate::LubikanChargeChanged`].
	pub fn try_activate(
		&mut self,
		catalog: &LubikanCatalog,
		requested: Option<LubikanAccountId>,
	) -> Result<LubikanActivation, LubikanActivationDenied> {
		if let Some(id) = requested {
			if !self.contains(id) {
				return Err(LubikanActivationDenied::UnknownAccount);
			}
			if !self.select(Some(id)) {
				return Err(LubikanActivationDenied::UnknownAccount);
			}
		}
		let ready = self.readiness(catalog);
		if !ready.can_activate {
			let equipped = self.equipped();
			if equipped.activation.is_busy() {
				return Err(LubikanActivationDenied::AlreadyActive);
			}
			return Err(LubikanActivationDenied::InsufficientCharge);
		}
		let equipped = self.equipped_mut();
		equipped.activation.begin();
		Ok(equipped.activation)
	}

	pub fn complete_activation(&mut self) {
		self.equipped_mut().activation.complete();
	}

	pub fn reset_activation(&mut self) {
		self.equipped_mut().activation.reset();
	}

	/// Apply a lifetime policy. Self remains owned.
	pub fn apply_lifetime(&mut self, persistence: LubikanPersistence) {
		match persistence {
			LubikanPersistence::Retain => {}
			LubikanPersistence::ResetCharge => self.reset_charges(),
			LubikanPersistence::ResetAll => {
				self.accounts.retain(|account| account.id.is_self());
				self.reset_charges();
				self.selection.clear_to_self();
			}
		}
	}

	pub fn apply_lifetime_kind(
		&mut self,
		policy: LubikanLifetimePolicy,
		kind: LubikanOwnerLifetimeKind,
	) {
		self.apply_lifetime(policy.for_kind(kind));
	}

	/// Unequip the current external selection. Charge stays on that account
	/// unless the unequip policy resets it.
	pub fn unequip(&mut self, policy: LubikanLifetimePolicy) {
		if let Some(id) = self.selection.explicit() {
			if policy.on_unequip == LubikanPersistence::ResetCharge {
				if let Some(account) = self.account_mut(id) {
					let capacity = account.charge.capacity();
					account.charge = LubikanCharge::empty(capacity);
					account.activation.reset();
				}
			} else if policy.on_unequip == LubikanPersistence::ResetAll {
				self.accounts.retain(|account| account.id.is_self() || account.id != id);
			}
		}
		self.selection.clear_to_self();
	}

	fn reset_charges(&mut self) {
		for account in &mut self.accounts {
			let capacity = account.charge.capacity();
			account.charge = LubikanCharge::empty(capacity);
			account.activation = LubikanActivation::idle(account.id);
		}
	}

	pub fn self_definition_id() -> LubikanDefinitionId {
		LubikanDefinitionId::SELF
	}
}

impl Default for LubikanLedger {
	fn default() -> Self {
		Self::with_self(&LubikanSelfConfig::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::activation::LubikanActivationPhase;
	use anyhow::{anyhow, Result};

	fn extra(capacity: f32) -> LubikanDefinition {
		LubikanDefinition::new(LubikanDefinitionId::from_raw(7), capacity, 4.0)
	}

	#[test]
	fn selection_does_not_transfer_or_drop_self() -> Result<()> {
		let mut ledger = LubikanLedger::default();
		ledger
			.apply_to(LubikanAccountId::SELF, 12.0)
			.ok_or_else(|| anyhow!("self account"))?;
		let other = ledger.ensure_owned(extra(30.0));
		ledger.apply_to(other, 9.0).ok_or_else(|| anyhow!("other account"))?;
		if !ledger.select(Some(other)) {
			return Err(anyhow!("should own the unlocked account"));
		}
		assert_eq!(ledger.equipped().id(), other);
		assert_eq!(ledger.equipped().charge().current(), 9.0);
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(12.0)
		);
		if ledger.select(Some(LubikanAccountId::unique(LubikanDefinitionId::from_raw(99)))) {
			return Err(anyhow!("unknown accounts must not become equipped"));
		}
		assert_eq!(ledger.equipped().id(), other);
		if !ledger.select(None) {
			return Err(anyhow!("clear to self"));
		}
		assert!(ledger.selection().is_self());
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(12.0)
		);
		assert_eq!(ledger.account(other).map(|account| account.charge().current()), Some(9.0));
		Ok(())
	}

	#[test]
	fn insufficient_charge_cannot_activate() -> Result<()> {
		let catalog = LubikanCatalog::default();
		let mut ledger = LubikanLedger::default();
		match ledger.try_activate(&catalog, None) {
			Err(LubikanActivationDenied::InsufficientCharge) => {}
			other => return Err(anyhow!("expected insufficient, got {other:?}")),
		}
		ledger.apply_to_equipped(catalog.activation_cost(LubikanDefinitionId::SELF));
		let activation = ledger.try_activate(&catalog, None)?;
		assert_eq!(activation.phase, LubikanActivationPhase::Active);
		match ledger.try_activate(&catalog, None) {
			Err(LubikanActivationDenied::AlreadyActive) => Ok(()),
			other => Err(anyhow!("expected already-active, got {other:?}")),
		}
	}

	#[test]
	fn account_binding_snapshots_self_without_creating_charge() {
		let owner = Entity::from_bits(42);
		let binding = LubikanAccountBinding::self_on(owner);
		assert!(binding.is_self());
		assert!(binding.matches_body(owner));
		assert_eq!(binding.account, LubikanAccountId::SELF);
		assert_eq!(binding.definition, LubikanDefinitionId::SELF);
		assert_eq!(binding.generation(), owner.to_bits());
	}

	#[test]
	fn reset_all_keeps_self() {
		let mut ledger = LubikanLedger::default();
		let other = ledger.ensure_owned(extra(8.0));
		ledger.select(Some(other));
		ledger.apply_to_equipped(3.0);
		ledger.apply_lifetime(LubikanPersistence::ResetAll);
		assert!(ledger.contains(LubikanAccountId::SELF));
		assert!(!ledger.contains(other));
		assert!(ledger.selection().is_self());
		assert_eq!(ledger.equipped().charge().current(), 0.0);
	}
}
