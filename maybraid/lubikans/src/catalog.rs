//! Self configuration and the registered definition catalog.

use bevy::prelude::*;

use crate::identity::{LubikanDefinition, LubikanDefinitionId};
use crate::lifetime::LubikanLifetimePolicy;

/// Default Self capacity. Balancing should change [`LubikanSelfConfig`], not
/// this constant's meaning.
pub const DEFAULT_SELF_CAPACITY: f32 = 512.0;

/// Default Self activation cost. Balancing should change [`LubikanSelfConfig`].
pub const DEFAULT_SELF_ACTIVATION_COST: f32 = 512.0;

/// Default Self activation stock. Balancing should change [`LubikanSelfConfig`].
pub const DEFAULT_SELF_MAX_STOCK: u32 = 1;

/// Configurable Self limits and lifetime policy.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct LubikanSelfConfig {
	pub capacity: f32,
	pub activation_cost: f32,
	/// How many Self activations can be banked. Defaults to `1`.
	pub max_stock: u32,
	pub lifetime: LubikanLifetimePolicy,
}

impl Default for LubikanSelfConfig {
	fn default() -> Self {
		Self {
			capacity: DEFAULT_SELF_CAPACITY,
			activation_cost: DEFAULT_SELF_ACTIVATION_COST,
			max_stock: DEFAULT_SELF_MAX_STOCK,
			lifetime: LubikanLifetimePolicy::default(),
		}
	}
}

impl LubikanSelfConfig {
	pub fn self_definition(self) -> LubikanDefinition {
		LubikanDefinition::self_with(self.capacity, self.activation_cost)
			.with_max_stock(self.max_stock)
	}
}

/// Registered Lubikan definitions. Self is always present.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct LubikanCatalog {
	entries: Vec<LubikanDefinition>,
}

impl LubikanCatalog {
	pub fn with_self(config: &LubikanSelfConfig) -> Self {
		Self { entries: vec![config.self_definition()] }
	}

	pub fn get(&self, id: LubikanDefinitionId) -> Option<LubikanDefinition> {
		self.entries.iter().copied().find(|entry| entry.id == id)
	}

	pub fn self_definition(&self) -> LubikanDefinition {
		self.get(LubikanDefinitionId::SELF)
			.unwrap_or_else(|| LubikanSelfConfig::default().self_definition())
	}

	pub fn capacity(&self, id: LubikanDefinitionId) -> f32 {
		self.get(id).map(|entry| entry.capacity).unwrap_or(0.0)
	}

	pub fn activation_cost(&self, id: LubikanDefinitionId) -> f32 {
		self.get(id).map(|entry| entry.activation_cost).unwrap_or(0.0)
	}

	pub fn max_stock(&self, id: LubikanDefinitionId) -> u32 {
		self.get(id).map(|entry| entry.max_stock).unwrap_or(1)
	}

	pub fn stock_capacity(&self, id: LubikanDefinitionId) -> f32 {
		self.get(id).map(|entry| entry.stock_capacity()).unwrap_or(0.0)
	}

	/// Insert or replace a definition. Self cannot be removed; registering
	/// Self overwrites the configured limits.
	pub fn register(&mut self, definition: LubikanDefinition) {
		if let Some(existing) = self.entries.iter_mut().find(|entry| entry.id == definition.id) {
			*existing = definition;
		} else {
			self.entries.push(definition);
		}
	}

	pub fn iter(&self) -> impl Iterator<Item = LubikanDefinition> + '_ {
		self.entries.iter().copied()
	}
}

impl Default for LubikanCatalog {
	fn default() -> Self {
		Self::with_self(&LubikanSelfConfig::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn catalog_always_has_self_and_can_replace_limits() {
		let mut catalog = LubikanCatalog::default();
		assert!(catalog.self_definition().is_self());
		assert_eq!(catalog.capacity(LubikanDefinitionId::SELF), DEFAULT_SELF_CAPACITY);
		catalog.register(LubikanDefinition::self_with(40.0, 8.0));
		assert_eq!(catalog.activation_cost(LubikanDefinitionId::SELF), 8.0);
		assert_eq!(catalog.max_stock(LubikanDefinitionId::SELF), 1);
		catalog.register(
			LubikanDefinition::new(LubikanDefinitionId::from_raw(4), 12.0, 3.0).with_max_stock(3),
		);
		assert_eq!(
			catalog.get(LubikanDefinitionId::from_raw(4)).map(|entry| entry.capacity),
			Some(12.0)
		);
		assert_eq!(catalog.max_stock(LubikanDefinitionId::from_raw(4)), 3);
		assert_eq!(catalog.stock_capacity(LubikanDefinitionId::from_raw(4)), 9.0);
	}
}
