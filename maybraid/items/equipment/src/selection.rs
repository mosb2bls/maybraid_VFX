//! Inventory category and ordered equipped-queue selection.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;
use lubikans::{LubikanAccountId, LubikanLedger, LubikanOwner};
use maybraid_character_controller::{CharacterIntent, InventoryCyclingCategory};
use maybraid_controllers_core::{InputRouter, InputSource};

use crate::lifecycle::EquipmentInventoryChanged;

/// HUD projection of the local actor's [`InventoryCycling`]. The actor
/// component is the owner; this resource is a reader-only mirror.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveInventoryCyclingCategory(pub InventoryCyclingCategory);

impl Default for ActiveInventoryCyclingCategory {
	fn default() -> Self {
		Self(InventoryCyclingCategory::Weapons)
	}
}

impl ActiveInventoryCyclingCategory {
	pub const fn from_category(category: InventoryCyclingCategory) -> Self {
		Self(category)
	}

	pub const fn category(self) -> InventoryCyclingCategory {
		self.0
	}

	pub fn project(&mut self, category: InventoryCyclingCategory) {
		self.0 = category;
	}
}

/// Per-actor category owner. One writer: inventory category selection.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct InventoryCycling {
	category: InventoryCyclingCategory,
}

impl InventoryCycling {
	pub const fn new(category: InventoryCyclingCategory) -> Self {
		Self { category }
	}

	pub const fn category(&self) -> InventoryCyclingCategory {
		self.category
	}

	pub fn select(&mut self, category: InventoryCyclingCategory) {
		self.category = category;
	}

	/// Rotate the equipped queue for this category. Weapons holster separately.
	pub fn cycle_bag(&self, bag: &mut Inventory, dir: i8) -> bool {
		match self.category {
			InventoryCyclingCategory::SkillMaps => bag.cycle_skills(dir),
			InventoryCyclingCategory::Weapons | InventoryCyclingCategory::Lubikans => false,
		}
	}
}

/// Advance equipped Lubikan selection across owned accounts. Selection only —
/// never mutates charge.
pub struct EquippedLubikanQueue;

impl EquippedLubikanQueue {
	pub fn cycle(ledger: &mut LubikanLedger, dir: i8) -> bool {
		let order = Self::owned_account_order(ledger);
		if order.len() < 2 {
			return false;
		}
		let current = ledger.selection().resolved();
		let position = order.iter().position(|&id| id == current).unwrap_or(0);
		let len = order.len() as i32;
		let step = if dir < 0 { -1 } else { 1 };
		let next = order[(position as i32 + step).rem_euclid(len) as usize];
		Self::select_owned(ledger, next)
	}

	fn owned_account_order(ledger: &LubikanLedger) -> Vec<LubikanAccountId> {
		let mut order: Vec<_> = ledger.iter().map(|account| account.id()).collect();
		order.sort();
		order
	}

	fn select_owned(ledger: &mut LubikanLedger, account: LubikanAccountId) -> bool {
		if account.is_self() {
			ledger.select(None)
		} else {
			ledger.select(Some(account))
		}
	}
}

/// D-pad category pick runs before holster begin and queue cycling on the same frame.
pub fn apply_inventory_category_selection(
	router: Res<InputRouter>,
	mut commands: Commands,
	mut cycling: Query<&mut InventoryCycling>,
	mut projected: ResMut<ActiveInventoryCyclingCategory>,
	mut intents: MessageReader<CharacterIntent>,
) {
	let Some(actor) = router.actor(InputSource::local_pad()) else {
		return;
	};
	let mut selected = None;
	for intent in intents.read() {
		if let CharacterIntent::SelectInventoryCyclingCategory(next) = *intent {
			selected = Some(next);
		}
	}
	if let Some(next) = selected {
		if let Ok(mut owner) = cycling.get_mut(actor.entity()) {
			owner.select(next);
		} else {
			commands.entity(actor.entity()).insert(InventoryCycling::new(next));
		}
		projected.project(next);
	} else if let Ok(owner) = cycling.get(actor.entity()) {
		projected.project(owner.category());
	}
}

pub fn cycle_equipped_inventory_queues(
	router: Res<InputRouter>,
	cycling: Query<&InventoryCycling>,
	mut intents: MessageReader<CharacterIntent>,
	mut changes: MessageWriter<EquipmentInventoryChanged>,
	users: Query<&InventoryUser>,
	mut bags: Query<&mut Inventory>,
	mut ledgers: Query<&mut LubikanLedger, With<LubikanOwner>>,
) {
	let Some(actor) = router.actor(InputSource::local_pad()) else {
		return;
	};
	let Some(dir) = queue_step(&mut intents) else {
		return;
	};
	let category = cycling
		.get(actor.entity())
		.ok()
		.map(InventoryCycling::category)
		.unwrap_or_default();
	if category == InventoryCyclingCategory::Lubikans {
		if let Ok(mut ledger) = ledgers.get_mut(actor.entity()) {
			EquippedLubikanQueue::cycle(&mut ledger, dir);
		}
		return;
	}
	if category == InventoryCyclingCategory::Weapons {
		// Weapon queue steps use [`crate::transition::begin_equipment_holster`].
		return;
	}
	let Ok(user) = users.get(actor.entity()) else {
		return;
	};
	let Ok(mut bag) = bags.get_mut(user.bag) else {
		return;
	};
	let owner = InventoryCycling::new(category);
	if !owner.cycle_bag(&mut bag, dir) {
		return;
	}
	changes.write(EquipmentInventoryChanged::new(actor.entity(), bag.clone()));
}

fn queue_step(intents: &mut MessageReader<CharacterIntent>) -> Option<i8> {
	for intent in intents.read() {
		match *intent {
			CharacterIntent::SwapActive => return Some(1),
			CharacterIntent::CycleEquippedQueue(step) => return Some(step),
			_ => {}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use character_items::InventoryItem;
	use lubikans::{LubikanDefinition, LubikanDefinitionId, LubikanLedger, LubikanSelection};
	use maybraid_controllers_core::ActorTarget;

	use super::*;

	fn assign_actor(world: &mut World, extra: impl Bundle) -> Entity {
		world.init_resource::<InputRouter>();
		let actor = world.spawn((InventoryCycling::default(), extra)).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
		actor
	}

	#[test]
	fn category_selection_updates_the_owner() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<ActiveInventoryCyclingCategory>();
		world.init_resource::<InputRouter>();
		let actor = world.spawn(InventoryCycling::default()).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SelectInventoryCyclingCategory(
					InventoryCyclingCategory::SkillMaps,
				));
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<InventoryCycling>(actor).map(InventoryCycling::category),
			Some(InventoryCyclingCategory::SkillMaps)
		);
		assert_eq!(
			world.resource::<ActiveInventoryCyclingCategory>().category(),
			InventoryCyclingCategory::SkillMaps
		);
		Ok(())
	}

	#[test]
	fn selecting_a_category_does_not_cycle_items() -> anyhow::Result<()> {
		use character_items::{SkillMapKind, SkillMapSpec};

		let inventory = Inventory {
			items: vec![
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: vec![0, 1],
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::SkillMaps));
		let bag = world.spawn(inventory).id();
		let actor = assign_actor(&mut world, InventoryUser::carrying(bag));
		if let Some(mut cycling) = world.get_mut::<InventoryCycling>(actor) {
			cycling.select(InventoryCyclingCategory::SkillMaps);
		}
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SelectInventoryCyclingCategory(
					InventoryCyclingCategory::Weapons,
				));
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Fireball, 1))
		);
		Ok(())
	}

	#[test]
	fn same_frame_category_then_y_uses_the_new_category() -> anyhow::Result<()> {
		use character_items::{SkillMapKind, SkillMapSpec};

		let inventory = Inventory {
			items: vec![
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: vec![0, 1],
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::Weapons));
		let bag = world.spawn(inventory).id();
		assign_actor(&mut world, InventoryUser::carrying(bag));
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SelectInventoryCyclingCategory(
					InventoryCyclingCategory::SkillMaps,
				));
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		Ok(())
	}

	#[test]
	fn lubikan_cycle_changes_selection_not_charge() -> anyhow::Result<()> {
		use lubikans::LubikanSelfConfig;

		let extra = LubikanDefinition::new(LubikanDefinitionId::from_raw(2), 12.0, 4.0);
		let config = LubikanSelfConfig::default();
		let mut ledger = LubikanLedger::with_self(&config);
		let extra_id = ledger.ensure_owned(extra);
		let _ = ledger.apply_to_equipped(5.0);
		assert!(ledger.select(Some(extra_id)));
		let extra_charge = ledger
			.account(extra_id)
			.ok_or_else(|| anyhow::anyhow!("extra"))?
			.charge()
			.current();
		assert!(EquippedLubikanQueue::cycle(&mut ledger, 1));
		assert_eq!(ledger.selection(), LubikanSelection::default());
		let extra_after = ledger
			.account(extra_id)
			.ok_or_else(|| anyhow::anyhow!("extra"))?
			.charge()
			.current();
		assert_eq!(extra_charge, extra_after);
		Ok(())
	}

	#[test]
	fn single_owned_lubikan_cycle_is_safe() -> anyhow::Result<()> {
		use lubikans::LubikanSelfConfig;

		let mut ledger = LubikanLedger::with_self(&LubikanSelfConfig::default());
		assert!(!EquippedLubikanQueue::cycle(&mut ledger, 1));
		assert!(ledger.selection().is_self());
		Ok(())
	}
}
