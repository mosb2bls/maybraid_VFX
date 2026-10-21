//! Unequip / hold requests. Item crates realize construction and teardown.

use bevy::prelude::*;
use character_items::Inventory;

use crate::dispatch::EquippedItemHandler;

/// Bag mutation equipment already committed. Hosts (world loadout) persist it.
#[derive(Message, Clone, Debug)]
pub struct EquipmentInventoryChanged {
	actor: Entity,
	inventory: Inventory,
}

impl EquipmentInventoryChanged {
	pub const fn new(actor: Entity, inventory: Inventory) -> Self {
		Self { actor, inventory }
	}

	pub const fn actor(&self) -> Entity {
		self.actor
	}

	pub fn inventory(&self) -> &Inventory {
		&self.inventory
	}

	pub fn into_inventory(self) -> Inventory {
		self.inventory
	}
}

/// Request that item crates tear down the actor's held kit.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct PendingEquipmentTeardown;

impl PendingEquipmentTeardown {
	pub fn request(entity: &mut EntityCommands) {
		entity.insert(Self);
	}
}

/// Request that item crates hold the bag's current primary weapon.
#[derive(Component, Clone, Debug)]
pub struct PendingEquipmentHold(Inventory);

impl PendingEquipmentHold {
	pub fn new(inventory: Inventory) -> Self {
		Self(inventory)
	}

	pub fn inventory(&self) -> &Inventory {
		&self.0
	}

	pub fn request(entity: &mut EntityCommands, inventory: Inventory) {
		EquippedItemHandler::from_inventory(&inventory).install(entity);
		entity.insert(Self(inventory));
	}
}

/// Drop request components after item crates have realized them.
pub fn clear_equipment_requests(
	mut commands: Commands,
	teardown: Query<Entity, With<PendingEquipmentTeardown>>,
	hold: Query<Entity, With<PendingEquipmentHold>>,
) {
	for entity in &teardown {
		commands.entity(entity).remove::<PendingEquipmentTeardown>();
	}
	for entity in &hold {
		commands.entity(entity).remove::<PendingEquipmentHold>();
	}
}
