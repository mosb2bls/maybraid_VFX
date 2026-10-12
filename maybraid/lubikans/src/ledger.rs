//! Per-owner owned accounts, charge, and equipped selection.

use bevy::prelude::*;

use crate::activation::{
	LubikanActivation, LubikanActivationDenied, LubikanActivationGrant, LubikanActivationOutcome,
	LubikanActivationSessionId, LubikanReadiness,
};
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
#[component(on_remove = crate::plugin::capture_retiring_ledger)]
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

	/// Equip `account` when it is owned. `None` returns to Self (`LubikanSelection`
	/// default / Self fallback). Selecting Self is the same as `None`.
	/// Unknown accounts are left unchanged and return `false`. Charge is never
	/// copied. [#1112](https://github.com/ramate-io/maybraid/issues/1112) cycles
	/// through this method only.
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
		self.readiness_for(catalog, self.selection.resolved()).unwrap_or_else(|| {
			let equipped = self.equipped();
			LubikanReadiness::from_charge(
				equipped.id,
				equipped.charge,
				catalog.activation_cost(equipped.id.definition()),
				equipped.activation.is_busy(),
			)
		})
	}

	pub fn readiness_for(
		&self,
		catalog: &LubikanCatalog,
		account: LubikanAccountId,
	) -> Option<LubikanReadiness> {
		self.account(account).map(|owned| {
			LubikanReadiness::from_charge(
				owned.id,
				owned.charge,
				catalog.activation_cost(owned.id.definition()),
				owned.activation.is_busy(),
			)
		})
	}

	/// Accept activation for `requested` (or the current selection).
	///
	/// Spends the catalog cost exactly once on that account and captures a
	/// session id. Completion and reset take that account (or session id),
	/// not the later equipped selection. A denial spends nothing.
	/// The Activate-set owner emits [`crate::LubikanActivationSession`] or
	/// [`crate::LubikanActivationRefused`]. [#1110](https://github.com/ramate-io/maybraid/issues/1110)
	/// and later summons consume the session and write
	/// [`crate::LubikanActivationReport`].
	pub fn try_activate(
		&mut self,
		catalog: &LubikanCatalog,
		requested: Option<LubikanAccountId>,
	) -> Result<LubikanActivationGrant, LubikanActivationDenied> {
		let id = requested.unwrap_or_else(|| self.selection.resolved());
		if !self.contains(id) {
			return Err(LubikanActivationDenied::UnknownAccount);
		}
		let ready =
			self.readiness_for(catalog, id).ok_or(LubikanActivationDenied::UnknownAccount)?;
		if !ready.can_activate {
			let busy = self.account(id).is_some_and(|account| account.activation.is_busy());
			if busy {
				return Err(LubikanActivationDenied::AlreadyActive);
			}
			return Err(LubikanActivationDenied::InsufficientCharge);
		}
		let cost = ready.activation_cost;
		let owned = self.account_mut(id).ok_or(LubikanActivationDenied::UnknownAccount)?;
		let spend = owned.charge.apply(-cost);
		let session = owned.activation.begin();
		Ok(LubikanActivationGrant {
			account: id,
			definition: id.definition(),
			id: session,
			spent: spend.applied,
		})
	}

	/// Complete the captured account. Does not consult current selection.
	pub fn complete_activation(&mut self, account: LubikanAccountId) -> bool {
		match self.account_mut(account) {
			Some(owned) => {
				owned.activation.complete();
				true
			}
			None => false,
		}
	}

	/// Reset the captured account. Does not consult current selection.
	pub fn reset_activation(&mut self, account: LubikanAccountId) -> bool {
		match self.account_mut(account) {
			Some(owned) => {
				owned.activation.reset();
				true
			}
			None => false,
		}
	}

	/// Close an accepted session by id. Unknown ids leave the ledger unchanged.
	pub fn finish_activation(
		&mut self,
		session: LubikanActivationSessionId,
		outcome: LubikanActivationOutcome,
	) -> bool {
		let Some(owned) = self
			.accounts
			.iter_mut()
			.find(|account| account.activation.session == Some(session))
		else {
			return false;
		};
		match outcome {
			LubikanActivationOutcome::Completed => owned.activation.complete(),
			LubikanActivationOutcome::Cancelled => owned.activation.reset(),
		}
		true
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
		if !ledger.select(Some(LubikanAccountId::SELF)) {
			return Err(anyhow!("select(Some(Self)) is the same as None"));
		}
		assert_eq!(ledger.selection(), crate::LubikanSelection::default());
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.charge().current()),
			Some(12.0)
		);
		assert_eq!(ledger.account(other).map(|account| account.charge().current()), Some(9.0));
		if !ledger.select(None) {
			return Err(anyhow!("select(None) is the Self fallback"));
		}
		assert_eq!(ledger.selection(), crate::LubikanSelection::default());
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
		let cost = catalog.activation_cost(LubikanDefinitionId::SELF);
		ledger.apply_to_equipped(cost);
		let grant = ledger.try_activate(&catalog, None)?;
		assert_eq!(grant.account, LubikanAccountId::SELF);
		assert_eq!(grant.spent, -cost);
		assert_eq!(ledger.equipped().activation().phase, LubikanActivationPhase::Active);
		assert_eq!(ledger.equipped().charge().current(), 0.0);
		match ledger.try_activate(&catalog, None) {
			Err(LubikanActivationDenied::AlreadyActive) => {}
			other => return Err(anyhow!("expected already-active, got {other:?}")),
		}
		Ok(())
	}

	#[test]
	fn complete_activation_uses_the_captured_account() -> Result<()> {
		let catalog = LubikanCatalog::default();
		let mut ledger = LubikanLedger::default();
		let extra = extra(40.0);
		let other = ledger.ensure_owned(extra);
		ledger.apply_to(LubikanAccountId::SELF, catalog.activation_cost(LubikanDefinitionId::SELF));
		ledger.apply_to(other, 9.0);
		let grant = ledger.try_activate(&catalog, None)?;
		assert_eq!(grant.account, LubikanAccountId::SELF);
		if !ledger.select(Some(other)) {
			return Err(anyhow!("switch after activate"));
		}
		if !ledger.complete_activation(grant.account) {
			return Err(anyhow!("complete captured Self"));
		}
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Completed)
		);
		assert_eq!(
			ledger.account(other).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Idle)
		);
		assert_eq!(ledger.account(other).map(|account| account.charge().current()), Some(9.0));
		if !ledger.finish_activation(grant.id, LubikanActivationOutcome::Cancelled) {
			return Err(anyhow!("cancel still finds the captured session"));
		}
		assert_eq!(
			ledger.account(LubikanAccountId::SELF).map(|account| account.activation().phase),
			Some(LubikanActivationPhase::Idle)
		);
		Ok(())
	}

	#[test]
	fn denial_does_not_spend() -> Result<()> {
		let catalog = LubikanCatalog::default();
		let mut ledger = LubikanLedger::default();
		ledger.apply_to_equipped(3.0);
		match ledger.try_activate(&catalog, None) {
			Err(LubikanActivationDenied::InsufficientCharge) => {}
			other => return Err(anyhow!("expected insufficient, got {other:?}")),
		}
		assert_eq!(ledger.equipped().charge().current(), 3.0);
		assert_eq!(ledger.equipped().activation().phase, LubikanActivationPhase::Idle);
		Ok(())
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
