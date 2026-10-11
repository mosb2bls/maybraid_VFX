//! Activation request, derived readiness, and session phase.

use bevy::prelude::*;

use crate::charge::LubikanCharge;
use crate::identity::LubikanAccountId;

/// Ask the equipped (or specified) Lubikan to activate.
///
/// Input writes this. Gameplay decides eligibility; HUD visibility cannot.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanActivationRequest {
	pub owner: Entity,
	/// `None` means the owner's current selection (Self when unequipped).
	pub lubikan: Option<LubikanAccountId>,
}

impl LubikanActivationRequest {
	pub fn equipped(owner: Entity) -> Self {
		Self { owner, lubikan: None }
	}

	pub fn for_account(owner: Entity, lubikan: LubikanAccountId) -> Self {
		Self { owner, lubikan: Some(lubikan) }
	}
}

/// Derived eligibility snapshot. Presentation reads this; it does not mutate
/// charge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LubikanReadiness {
	pub account: LubikanAccountId,
	pub charge: f32,
	pub capacity: f32,
	pub activation_cost: f32,
	pub can_activate: bool,
}

impl LubikanReadiness {
	pub fn from_charge(
		account: LubikanAccountId,
		charge: LubikanCharge,
		activation_cost: f32,
		busy: bool,
	) -> Self {
		let cost = if activation_cost.is_finite() { activation_cost.max(0.0) } else { 0.0 };
		Self {
			account,
			charge: charge.current(),
			capacity: charge.capacity(),
			activation_cost: cost,
			can_activate: !busy && charge.can_afford(cost),
		}
	}
}

/// Activation session visible to presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanActivation {
	pub account: LubikanAccountId,
	pub phase: LubikanActivationPhase,
}

impl LubikanActivation {
	pub fn idle(account: LubikanAccountId) -> Self {
		Self { account, phase: LubikanActivationPhase::Idle }
	}

	pub fn is_active(self) -> bool {
		self.phase == LubikanActivationPhase::Active
	}

	pub fn is_busy(self) -> bool {
		matches!(self.phase, LubikanActivationPhase::Active | LubikanActivationPhase::Completed)
	}

	pub fn begin(&mut self) {
		self.phase = LubikanActivationPhase::Active;
	}

	pub fn complete(&mut self) {
		self.phase = LubikanActivationPhase::Completed;
	}

	pub fn reset(&mut self) {
		self.phase = LubikanActivationPhase::Idle;
	}
}

/// Lifecycle of one accepted activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanActivationPhase {
	Idle,
	Active,
	Completed,
}

/// Why [`crate::LubikanLedger::try_activate`] refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanActivationDenied {
	UnknownAccount,
	InsufficientCharge,
	AlreadyActive,
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn readiness_blocks_busy_or_poor_accounts() {
		let charge = LubikanCharge::new(10.0, 20.0);
		let ready = LubikanReadiness::from_charge(LubikanAccountId::SELF, charge, 10.0, false);
		assert!(ready.can_activate);
		let busy = LubikanReadiness::from_charge(LubikanAccountId::SELF, charge, 10.0, true);
		assert!(!busy.can_activate);
		let poor = LubikanReadiness::from_charge(LubikanAccountId::SELF, charge, 10.1, false);
		assert!(!poor.can_activate);
	}
}
