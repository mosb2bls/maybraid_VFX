use std::collections::HashMap;

use bevy::prelude::*;

use crate::{AttackDefinition, AttackId};

/// Shared definition table. Encounter and summoned owners look up the same ids.
#[derive(Resource, Clone, Debug, Default)]
pub struct AttackCatalog {
	definitions: HashMap<AttackId, AttackDefinition>,
}

impl AttackCatalog {
	pub fn insert(&mut self, definition: AttackDefinition) -> &mut Self {
		self.definitions.insert(definition.id, definition);
		self
	}

	pub fn get(&self, id: AttackId) -> Option<&AttackDefinition> {
		self.definitions.get(&id)
	}

	pub fn contains(&self, id: AttackId) -> bool {
		self.definitions.contains_key(&id)
	}

	pub fn iter(&self) -> impl Iterator<Item = &AttackDefinition> {
		self.definitions.values()
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::{AimPolicy, AttackDefinition};

	#[test]
	fn catalog_shares_one_definition_key() {
		let mut catalog = AttackCatalog::default();
		catalog.insert(AttackDefinition::with_authored_defaults(
			AttackId::new("shared"),
			1.0,
			1.0,
			1.0,
			AimPolicy::OwnerPoseAtWindup,
		));
		assert!(catalog.contains(AttackId::new("shared")));
		assert_eq!(catalog.get(AttackId::new("shared")).map(|def| def.id.as_str()), Some("shared"));
	}
}
