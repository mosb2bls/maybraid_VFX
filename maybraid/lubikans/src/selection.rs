//! Equipped selection. No equipped external Lubikan is Self.

use crate::identity::LubikanAccountId;

/// Which account is equipped on an owner.
///
/// `None` is the Self fallback. Consumers should read [`Self::resolved`]
/// rather than treating a missing selection as "no Lubikan".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LubikanSelection {
	equipped: Option<LubikanAccountId>,
}

impl LubikanSelection {
	pub fn unequipped() -> Self {
		Self { equipped: None }
	}

	pub fn of(account: LubikanAccountId) -> Self {
		if account.is_self() {
			Self { equipped: None }
		} else {
			Self { equipped: Some(account) }
		}
	}

	/// Explicit equipped account, if any. `None` means the Self fallback.
	pub fn explicit(self) -> Option<LubikanAccountId> {
		self.equipped.filter(|account| !account.is_self())
	}

	/// Account gameplay should credit. Always a real id; never empty.
	pub fn resolved(self) -> LubikanAccountId {
		self.explicit().unwrap_or(LubikanAccountId::SELF)
	}

	pub fn is_self(self) -> bool {
		self.resolved().is_self()
	}

	pub fn select(&mut self, account: LubikanAccountId) {
		*self = Self::of(account);
	}

	pub fn clear_to_self(&mut self) {
		self.equipped = None;
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::identity::LubikanDefinitionId;

	#[test]
	fn missing_or_self_selection_resolves_to_self() {
		assert_eq!(LubikanSelection::default().resolved(), LubikanAccountId::SELF);
		assert_eq!(LubikanSelection::of(LubikanAccountId::SELF).explicit(), None);
		assert!(LubikanSelection::of(LubikanAccountId::SELF).is_self());
		let other = LubikanAccountId::unique(LubikanDefinitionId::from_raw(2));
		let selected = LubikanSelection::of(other);
		assert_eq!(selected.resolved(), other);
		assert_eq!(selected.explicit(), Some(other));
	}
}
