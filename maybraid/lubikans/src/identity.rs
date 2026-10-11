//! Catalog and owned-account identity.
//!
//! A definition is a catalog entry (Self, a future summon, …). An account is
//! one owner's charge bucket for that definition. Switching selection never
//! copies charge between accounts.

/// Catalog identity. `0` is reserved for Self.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LubikanDefinitionId(u32);

impl LubikanDefinitionId {
	/// The default Lubikan. Always registered in a normal game.
	pub const SELF: Self = Self(0);

	/// Wrap a raw catalog id. `0` is Self; content crates should start at `1`.
	pub const fn from_raw(raw: u32) -> Self {
		Self(raw)
	}

	pub const fn raw(self) -> u32 {
		self.0
	}

	pub const fn is_self(self) -> bool {
		self.0 == 0
	}

	/// Built-in label. Unknown ids return `None` so content crates can name
	/// their own definitions without this crate knowing about them.
	pub const fn well_known_label(self) -> Option<&'static str> {
		if self.is_self() {
			Some("Self")
		} else {
			None
		}
	}
}

/// One owned charge account. Distinct from [`LubikanDefinitionId`] so two
/// owned entries stay separate even when they share a catalog id.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LubikanAccountId {
	definition: LubikanDefinitionId,
	instance: u32,
}

impl LubikanAccountId {
	/// The owner's Self account. Instance `0`.
	pub const SELF: Self = Self { definition: LubikanDefinitionId::SELF, instance: 0 };

	/// One account for `definition` at `instance`. Self should use [`Self::SELF`].
	pub const fn new(definition: LubikanDefinitionId, instance: u32) -> Self {
		Self { definition, instance }
	}

	/// The unique instance-0 account for `definition`.
	pub const fn unique(definition: LubikanDefinitionId) -> Self {
		Self { definition, instance: 0 }
	}

	pub const fn definition(self) -> LubikanDefinitionId {
		self.definition
	}

	pub const fn instance(self) -> u32 {
		self.instance
	}

	pub const fn is_self(self) -> bool {
		self.definition.is_self() && self.instance == 0
	}
}

/// Catalog row: identity plus configurable charge limits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LubikanDefinition {
	pub id: LubikanDefinitionId,
	pub capacity: f32,
	pub activation_cost: f32,
}

impl LubikanDefinition {
	pub fn new(id: LubikanDefinitionId, capacity: f32, activation_cost: f32) -> Self {
		Self {
			id,
			capacity: sanitize_capacity(capacity),
			activation_cost: sanitize_cost(activation_cost),
		}
	}

	pub fn self_with(capacity: f32, activation_cost: f32) -> Self {
		Self::new(LubikanDefinitionId::SELF, capacity, activation_cost)
	}

	pub fn is_self(self) -> bool {
		self.id.is_self()
	}
}

pub(crate) fn sanitize_capacity(capacity: f32) -> f32 {
	if capacity.is_finite() {
		capacity.max(0.0)
	} else {
		0.0
	}
}

pub(crate) fn sanitize_cost(cost: f32) -> f32 {
	if cost.is_finite() {
		cost.max(0.0)
	} else {
		0.0
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn self_ids_are_stable_and_distinct_from_content() {
		assert!(LubikanDefinitionId::SELF.is_self());
		assert_eq!(LubikanDefinitionId::SELF.well_known_label(), Some("Self"));
		assert_eq!(LubikanDefinitionId::from_raw(1).well_known_label(), None);
		assert!(LubikanAccountId::SELF.is_self());
		assert!(!LubikanAccountId::new(LubikanDefinitionId::SELF, 1).is_self());
		assert!(!LubikanAccountId::unique(LubikanDefinitionId::from_raw(1)).is_self());
		assert_ne!(
			LubikanAccountId::unique(LubikanDefinitionId::SELF),
			LubikanAccountId::new(LubikanDefinitionId::from_raw(1), 0)
		);
	}

	#[test]
	fn definition_sanitizes_non_finite_limits() {
		let definition = LubikanDefinition::new(LubikanDefinitionId::from_raw(3), f32::NAN, -4.0);
		assert_eq!(definition.capacity, 0.0);
		assert_eq!(definition.activation_cost, 0.0);
	}
}
