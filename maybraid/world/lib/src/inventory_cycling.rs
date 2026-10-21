//! D-pad category selection and **Y** / bracket equipped-queue cycling.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;
use game_commands::command::TextEntryFocus;
use lubikans::{LubikanAccountId, LubikanLedger, LubikanOwner, LubikanSystems};
use maybraid_character_controller::{
	CharacterIntent, InventoryCycling, InventoryCyclingCategory,
};
use maybraid_controllers_core::{InputRouter, InputSource};
use world_player::Player as VegetationPlayer;

use crate::control::WorldGameplayEnabled;
use crate::map_view::WorldMapView;
use crate::weapon::WorldPlayerLoadout;
use maybraid_world_controller::SessionCapabilitySystems;

/// HUD projection of the local actor's [`InventoryCycling`]. The actor
/// component is the owner; this resource is a reader-only mirror.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct ActiveInventoryCyclingCategory(pub InventoryCyclingCategory);

impl Default for ActiveInventoryCyclingCategory {
	fn default() -> Self {
		Self(InventoryCyclingCategory::Weapons)
	}
}

/// D-pad category pick runs before weapon swap and queue cycling on the same frame.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InventoryCyclingCategorySystems;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InventoryCyclingSystems;

pub(crate) fn configure(app: &mut App) {
	app.init_resource::<ActiveInventoryCyclingCategory>()
		.configure_sets(
			Update,
			(
				InventoryCyclingCategorySystems,
				InventoryCyclingSystems.after(InventoryCyclingCategorySystems),
			),
		)
		.add_systems(
			Update,
			apply_inventory_category_selection
				.in_set(InventoryCyclingCategorySystems)
				.after(maybraid_character_controller::CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.after(LubikanSystems::Owner)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
		)
		.add_systems(
			Update,
			cycle_equipped_inventory_queues
				.in_set(InventoryCyclingSystems)
				.after(maybraid_character_controller::CharacterControlSystems)
				.after(SessionCapabilitySystems)
				.after(LubikanSystems::Owner)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
		);
}

pub(crate) fn gameplay_cycling_allowed(
	gameplay: &WorldGameplayEnabled,
	text_focus: &TextEntryFocus,
	map: Option<&WorldMapView>,
) -> bool {
	gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open))
}

pub(crate) fn apply_inventory_category_selection(
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	router: Res<InputRouter>,
	mut commands: Commands,
	mut cycling: Query<&mut InventoryCycling>,
	mut projected: ResMut<ActiveInventoryCyclingCategory>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open)) {
		return;
	}
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
		projected.0 = next;
	} else if let Ok(owner) = cycling.get(actor.entity()) {
		projected.0 = owner.category();
	}
}

pub(crate) fn cycle_equipped_inventory_queues(
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	router: Res<InputRouter>,
	cycling: Query<&InventoryCycling>,
	mut intents: MessageReader<CharacterIntent>,
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	players: Query<&InventoryUser, With<VegetationPlayer>>,
	mut bags: Query<&mut Inventory>,
	mut ledgers: Query<&mut LubikanLedger, With<LubikanOwner>>,
) {
	if !gameplay.accepts_character_input(text_focus.0, map.is_some_and(|map| map.open)) {
		return;
	}
	let Some(actor) = router.actor(InputSource::local_pad()) else {
		return;
	};
	let mut dir = None;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::SwapActive => {
				dir = Some(1i8);
				break;
			}
			CharacterIntent::CycleEquippedQueue(step) => {
				dir = Some(step);
				break;
			}
			_ => {}
		}
	}
	let Some(dir) = dir else {
		return;
	};
	let category = cycling.get(actor.entity()).ok().map(InventoryCycling::category).unwrap_or_default();
	if category == InventoryCyclingCategory::Lubikans {
		if let Ok(mut ledger) = ledgers.get_mut(actor.entity()) {
			cycle_ledger_selection(&mut ledger, dir);
		}
		return;
	}
	if category == InventoryCyclingCategory::Weapons {
		// Weapon queue steps use [`crate::weapon::begin_weapon_swap`] / [`commit_weapon_swap`].
		return;
	}
	let Ok(user) = players.get(actor.entity()) else {
		return;
	};
	let Ok(mut bag) = bags.get_mut(user.bag) else {
		return;
	};
	let changed = match category {
		InventoryCyclingCategory::SkillMaps => bag.cycle_skills(dir),
		InventoryCyclingCategory::Weapons | InventoryCyclingCategory::Lubikans => false,
	};
	if !changed {
		return;
	}
	let snapshot = bag.clone();
	if let Some(loadout) = loadout.as_deref_mut() {
		loadout.retarget_inventory(snapshot.clone());
		commands
			.entity(actor.entity())
			.insert(crate::weapon::AppliedWorldPlayerLoadout(loadout.clone()));
	}
}

/// Advance equipped selection across owned accounts. Selection only — never mutates charge.
pub(crate) fn cycle_ledger_selection(ledger: &mut LubikanLedger, dir: i8) -> bool {
	let order = owned_account_cycle_order(ledger);
	if order.len() < 2 {
		return false;
	}
	let current = ledger.selection().resolved();
	let position = order.iter().position(|&id| id == current).unwrap_or(0);
	let len = order.len() as i32;
	let step = if dir < 0 { -1 } else { 1 };
	let next = order[(position as i32 + step).rem_euclid(len) as usize];
	select_owned_account(ledger, next)
}

fn owned_account_cycle_order(ledger: &LubikanLedger) -> Vec<LubikanAccountId> {
	let mut order: Vec<_> = ledger.iter().map(|account| account.id()).collect();
	order.sort();
	order
}

fn select_owned_account(ledger: &mut LubikanLedger, account: LubikanAccountId) -> bool {
	if account.is_self() {
		ledger.select(None)
	} else {
		ledger.select(Some(account))
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use character_items::InventoryItem;
	use lubikans::{
		LubikanDefinition, LubikanDefinitionId, LubikanLedger, LubikanSelection, LubikanSelfConfig,
	};

	use super::*;

	fn assign_player(world: &mut World, extra: impl Bundle) -> Entity {
		use maybraid_controllers_core::ActorTarget;

		world.init_resource::<InputRouter>();
		let player = world.spawn((VegetationPlayer, InventoryCycling::default(), extra)).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(player));
		player
	}

	#[test]
	fn category_selection_updates_the_owner() -> anyhow::Result<()> {
		use maybraid_controllers_core::ActorTarget;

		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.init_resource::<ActiveInventoryCyclingCategory>();
		world.init_resource::<InputRouter>();
		let player = world.spawn((VegetationPlayer, InventoryCycling::default())).id();
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(player));
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
			world.get::<InventoryCycling>(player).map(InventoryCycling::category),
			Some(InventoryCyclingCategory::SkillMaps)
		);
		assert_eq!(
			world.resource::<ActiveInventoryCyclingCategory>().0,
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
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::SkillMaps));
		let bag = world.spawn(inventory).id();
		let player = assign_player(&mut world, InventoryUser::carrying(bag));
		if let Some(mut cycling) = world.get_mut::<InventoryCycling>(player) {
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
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::Weapons));
		let bag = world.spawn(inventory).id();
		assign_player(&mut world, InventoryUser::carrying(bag));
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
		assert!(cycle_ledger_selection(&mut ledger, 1));
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
		let mut ledger = LubikanLedger::with_self(&LubikanSelfConfig::default());
		assert!(!cycle_ledger_selection(&mut ledger, 1));
		assert!(ledger.selection().is_self());
		Ok(())
	}
}
