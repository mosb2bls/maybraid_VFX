//! Owner lifetime: equip, unequip, body replace, and session restart.

use bevy::prelude::*;

/// How an account store should behave across a lifetime event.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanPersistence {
	/// Keep owned accounts and their charge.
	Retain,
	/// Keep owned accounts; empty every charge bucket.
	ResetCharge,
	/// Drop non-Self accounts and empty Self.
	ResetAll,
}

/// Configurable persistence for each owner-lifetime seam.
///
/// Charge persistence after death is a balance knob: body replace is the
/// death/respawn path in the current world player lifecycle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanLifetimePolicy {
	pub on_unequip: LubikanPersistence,
	pub on_body_replace: LubikanPersistence,
	pub on_session_restart: LubikanPersistence,
}

impl Default for LubikanLifetimePolicy {
	fn default() -> Self {
		Self {
			on_unequip: LubikanPersistence::Retain,
			on_body_replace: LubikanPersistence::Retain,
			on_session_restart: LubikanPersistence::ResetCharge,
		}
	}
}

impl LubikanLifetimePolicy {
	pub fn for_kind(self, kind: LubikanOwnerLifetimeKind) -> LubikanPersistence {
		match kind {
			LubikanOwnerLifetimeKind::Equip => LubikanPersistence::Retain,
			LubikanOwnerLifetimeKind::Unequip => self.on_unequip,
			LubikanOwnerLifetimeKind::ReplaceBody => self.on_body_replace,
			LubikanOwnerLifetimeKind::RestartSession => self.on_session_restart,
		}
	}
}

/// Kind of owner-lifetime transition.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LubikanOwnerLifetimeKind {
	Equip,
	Unequip,
	ReplaceBody,
	RestartSession,
}

/// Notice that an owner's body or session changed.
///
/// `previous` is the body that lost ownership; `current` is the replacement
/// when one exists. Session restart may set both to `None` after teardown.
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LubikanOwnerLifetime {
	pub previous: Option<Entity>,
	pub current: Option<Entity>,
	pub kind: LubikanOwnerLifetimeKind,
}

impl LubikanOwnerLifetime {
	pub fn new(
		previous: Option<Entity>,
		current: Option<Entity>,
		kind: LubikanOwnerLifetimeKind,
	) -> Self {
		Self { previous, current, kind }
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_policy_retains_across_equip_and_replace() {
		let policy = LubikanLifetimePolicy::default();
		assert_eq!(policy.for_kind(LubikanOwnerLifetimeKind::Unequip), LubikanPersistence::Retain);
		assert_eq!(
			policy.for_kind(LubikanOwnerLifetimeKind::ReplaceBody),
			LubikanPersistence::Retain
		);
		assert_eq!(
			policy.for_kind(LubikanOwnerLifetimeKind::RestartSession),
			LubikanPersistence::ResetCharge
		);
	}
}
