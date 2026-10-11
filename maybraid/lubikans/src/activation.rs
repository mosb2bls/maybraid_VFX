//! Activation request, derived readiness, and session handoff.

use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;

use crate::charge::LubikanCharge;
use crate::identity::{LubikanAccountId, LubikanDefinitionId};

/// Ask the equipped (or specified) Lubikan to activate.
///
/// Input writes this. The gameplay plugin that runs
/// [`crate::LubikanSystems::Activate`] is the sole owner that accepts or
/// refuses. HUD visibility cannot.
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

/// Stable id for one accepted activation. Completion and cancel report
/// through this id so a later selection change cannot retarget the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LubikanActivationSessionId(u64);

impl LubikanActivationSessionId {
	pub fn mint() -> Self {
		static NEXT: AtomicU64 = AtomicU64::new(1);
		Self(NEXT.fetch_add(1, Ordering::Relaxed))
	}

	pub const fn from_raw(raw: u64) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u64 {
		self.0
	}
}

/// Accepted activation. Exactly one consumer (#1110 healing for Self, later
/// summons) picks this up and writes a [`LubikanActivationReport`].
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanActivationSession {
	pub owner: Entity,
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub id: LubikanActivationSessionId,
}

impl LubikanActivationSession {
	pub fn new(
		owner: Entity,
		account: LubikanAccountId,
		definition: LubikanDefinitionId,
		id: LubikanActivationSessionId,
	) -> Self {
		Self { owner, account, definition, id }
	}
}

/// Ledger-side accept: the account that paid and the session id to emit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LubikanActivationGrant {
	pub account: LubikanAccountId,
	pub definition: LubikanDefinitionId,
	pub id: LubikanActivationSessionId,
	/// Applied spend (negative of the catalog cost after clamp). Zero only
	/// when the catalog cost is zero.
	pub spent: f32,
}

impl LubikanActivationGrant {
	pub fn session(self, owner: Entity) -> LubikanActivationSession {
		LubikanActivationSession::new(owner, self.account, self.definition, self.id)
	}
}

/// Request owner refused. No charge was spent.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanActivationRefused {
	pub owner: Entity,
	pub lubikan: Option<LubikanAccountId>,
	pub reason: LubikanActivationDenied,
}

/// Consumer report for an accepted session. The request owner completes or
/// resets the captured account — never the later equipped selection.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanActivationReport {
	pub id: LubikanActivationSessionId,
	pub outcome: LubikanActivationOutcome,
}

impl LubikanActivationReport {
	pub fn completed(id: LubikanActivationSessionId) -> Self {
		Self { id, outcome: LubikanActivationOutcome::Completed }
	}

	pub fn cancelled(id: LubikanActivationSessionId) -> Self {
		Self { id, outcome: LubikanActivationOutcome::Cancelled }
	}
}

/// How a consumer closed an accepted session.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanActivationOutcome {
	Completed,
	Cancelled,
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
	/// Set when the request owner accepts. Completion uses this, not selection.
	pub session: Option<LubikanActivationSessionId>,
}

impl LubikanActivation {
	pub fn idle(account: LubikanAccountId) -> Self {
		Self { account, phase: LubikanActivationPhase::Idle, session: None }
	}

	pub fn is_active(self) -> bool {
		self.phase == LubikanActivationPhase::Active
	}

	pub fn is_busy(self) -> bool {
		matches!(self.phase, LubikanActivationPhase::Active | LubikanActivationPhase::Completed)
	}

	pub fn begin(&mut self) -> LubikanActivationSessionId {
		let id = LubikanActivationSessionId::mint();
		self.phase = LubikanActivationPhase::Active;
		self.session = Some(id);
		id
	}

	pub fn complete(&mut self) {
		self.phase = LubikanActivationPhase::Completed;
	}

	pub fn reset(&mut self) {
		self.phase = LubikanActivationPhase::Idle;
		self.session = None;
	}
}

/// Lifecycle of one accepted activation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanActivationPhase {
	Idle,
	Active,
	Completed,
}

/// Why [`crate::LubikanLedger::try_activate`] refused. A denial spends nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanActivationDenied {
	UnknownAccount,
	InsufficientCharge,
	AlreadyActive,
}

impl std::fmt::Display for LubikanActivationDenied {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.write_str(match self {
			Self::UnknownAccount => "unknown Lubikan account",
			Self::InsufficientCharge => "insufficient Lubikan charge",
			Self::AlreadyActive => "Lubikan already active",
		})
	}
}

impl std::error::Error for LubikanActivationDenied {}

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

	#[test]
	fn begin_captures_a_session_id_that_reset_clears() {
		let mut activation = LubikanActivation::idle(LubikanAccountId::SELF);
		let id = activation.begin();
		assert!(activation.is_active());
		assert_eq!(activation.session, Some(id));
		activation.complete();
		assert_eq!(activation.session, Some(id));
		activation.reset();
		assert!(!activation.is_busy());
		assert_eq!(activation.session, None);
	}
}
