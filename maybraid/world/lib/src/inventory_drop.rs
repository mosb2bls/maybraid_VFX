//! Apply [`InventoryDropRequest`] to a host bag and refresh the live loadout.
//!
//! Combat HUD is not consulted. Cycle-queue fixup lives on
//! [`character_items::Inventory::try_drop`]; this adapter persists the leftover
//! bag through [`WorldPlayerLoadout`] and optionally spawns a world stash when
//! the request's placeholder policy asks for a pickup.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{
	ActiveInventoryDropPolicy, Inventory, InventoryDropRefused, InventoryDropRequest,
	InventoryDropped,
};
use world_player::Player as VegetationPlayer;

use crate::stash::{spawn_world_stash, WorldStashSettings};
use crate::weapon::{AppliedWorldPlayerLoadout, WorldPlayerLoadout};

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InventoryDropSystems;

pub(crate) fn configure(app: &mut App) {
	app.init_resource::<ActiveInventoryDropPolicy>()
		.add_message::<InventoryDropRequest>()
		.add_message::<InventoryDropped>()
		.add_message::<InventoryDropRefused>()
		.configure_sets(Update, InventoryDropSystems)
		.add_systems(Update, apply_inventory_drop_requests.in_set(InventoryDropSystems));
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_inventory_drop_requests(
	mut requests: MessageReader<InventoryDropRequest>,
	mut dropped: MessageWriter<InventoryDropped>,
	mut refused: MessageWriter<InventoryDropRefused>,
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	settings: Option<Res<WorldStashSettings>>,
	assets: Option<Res<AssetServer>>,
	users: Query<(Entity, &InventoryUser, Option<&Transform>, Has<VegetationPlayer>)>,
	mut bags: Query<&mut Inventory>,
) {
	let assets = assets.as_deref();
	let ephemeral = settings.as_ref().map(|settings| settings.ephemeral_policy());
	for request in requests.read() {
		let Some((owner, user, transform, is_player)) =
			users.iter().find(|(entity, _, _, _)| *entity == request.owner)
		else {
			refused.write(InventoryDropRefused {
				owner: request.owner,
				index: request.index,
				reason: character_items::InventoryDropError::MissingIndex,
			});
			continue;
		};
		let Ok(mut bag) = bags.get_mut(user.bag) else {
			refused.write(InventoryDropRefused {
				owner,
				index: request.index,
				reason: character_items::InventoryDropError::MissingIndex,
			});
			continue;
		};
		match bag.try_drop(request.index, request.policy) {
			Ok(outcome) => {
				let snapshot = bag.clone();
				if is_player {
					if let Some(loadout) = loadout.as_deref_mut() {
						loadout.retarget_inventory(snapshot.clone());
						commands.entity(owner).insert(AppliedWorldPlayerLoadout(loadout.clone()));
					}
				}
				if outcome.spawn_world_pickup {
					if let Some(origin) = transform.map(|transform| transform.translation) {
						let policy = ephemeral.unwrap_or_default();
						spawn_world_stash(
							&mut commands,
							Transform::from_translation(origin + Vec3::Y * 0.05),
							visual_drop_bag(outcome.item.clone()),
							policy,
							assets,
						);
					}
				}
				dropped.write(InventoryDropped { owner, bag: user.bag, outcome });
			}
			Err(reason) => {
				refused.write(InventoryDropRefused { owner, index: request.index, reason });
			}
		}
	}
}

fn visual_drop_bag(item: character_items::InventoryItem) -> Inventory {
	let mut bag = Inventory {
		items: vec![item],
		clothing: Vec::new(),
		weapons: Vec::new(),
		skills: Vec::new(),
	};
	let _ = bag.toggle(0);
	bag
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use character_inventory_user::InventoryUser;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryDropPolicy, InventoryItem,
		ItemColor, SkillMapKind, SkillMapSpec,
	};
	use character_persist::{CharacterId, SaveRoot};
	use characters::CharacterAppearance;
	use maybraid_character_controller::InventoryCyclingCategory;

	use super::*;
	use crate::inventory_cycling::ActiveInventoryCyclingCategory;
	use crate::stash::WorldStash;
	use crate::weapon::WorldPlayerLoadout;

	fn queued_loadout() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::clothing(
					ClothingMesh::Pants,
					ClothingMaterial::Cloth,
					ItemColor::Natural,
				),
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::firearm(FirearmMesh::Reltor),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: vec![0],
			weapons: vec![1, 2],
			skills: vec![3, 4],
		}
	}

	fn drop_world(inventory: Inventory) -> (World, Entity, Entity) {
		let mut world = World::new();
		world.init_resource::<Messages<InventoryDropRequest>>();
		world.init_resource::<Messages<InventoryDropped>>();
		world.init_resource::<Messages<InventoryDropRefused>>();
		world.init_resource::<ActiveInventoryCyclingCategory>();
		let bag = world.spawn(inventory.clone()).id();
		let owner = world
			.spawn((
				VegetationPlayer,
				InventoryUser::carrying(bag),
				Transform::from_xyz(3.0, 0.0, 1.0),
			))
			.id();
		world.insert_resource(WorldPlayerLoadout::new(
			CharacterId(7).to_hex(),
			CharacterAppearance::default(),
			inventory,
		));
		(world, owner, bag)
	}

	fn run_drop(
		world: &mut World,
		owner: Entity,
		index: usize,
		policy: InventoryDropPolicy,
	) -> anyhow::Result<()> {
		world
			.run_system_once(move |mut writer: MessageWriter<InventoryDropRequest>| {
				writer.write(InventoryDropRequest::new(owner, index).with_policy(policy));
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_inventory_drop_requests)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn drop_removes_the_item_and_refreshes_the_loadout() -> anyhow::Result<()> {
		let (mut world, owner, bag) = drop_world(queued_loadout());
		run_drop(&mut world, owner, 0, InventoryDropPolicy::PLACEHOLDER)?;
		let leftover = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(leftover.items.len(), 4);
		assert!(leftover.clothing.is_empty());
		assert_eq!(leftover.weapons, vec![0, 1]);
		let loadout = world.resource::<WorldPlayerLoadout>();
		assert_eq!(loadout.inventory.items.len(), 4);
		assert_eq!(loadout.inventory.clothing, leftover.clothing);
		assert_eq!(loadout.inventory.weapons, leftover.weapons);
		let accepted = world.resource_mut::<Messages<InventoryDropped>>().drain().count();
		assert_eq!(accepted, 1);
		Ok(())
	}

	#[test]
	fn equipped_primary_drop_promotes_the_cycle_queue() -> anyhow::Result<()> {
		let (mut world, owner, bag) = drop_world(queued_loadout());
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::Weapons));
		run_drop(&mut world, owner, 1, InventoryDropPolicy::PLACEHOLDER)?;
		let leftover = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(leftover.weapons, vec![1]);
		assert_eq!(
			leftover.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert_eq!(leftover.skills, vec![2, 3]);
		assert_eq!(
			world.resource::<ActiveInventoryCyclingCategory>().0,
			InventoryCyclingCategory::Weapons
		);
		assert_eq!(
			world
				.resource::<WorldPlayerLoadout>()
				.inventory
				.primary_weapon()
				.and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}

	#[test]
	fn dropped_bag_round_trips_through_persist() -> anyhow::Result<()> {
		let (mut world, owner, bag) = drop_world(queued_loadout());
		run_drop(&mut world, owner, 1, InventoryDropPolicy::PLACEHOLDER)?;
		let leftover = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?.clone();
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		character_inventory_user::save(&root, id, &leftover)?;
		let loaded = character_inventory_user::load(&root, id)?;
		assert_eq!(loaded.items.len(), leftover.items.len());
		assert_eq!(loaded.weapons, leftover.weapons);
		assert_eq!(loaded.skills, leftover.skills);
		assert_eq!(
			loaded.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}

	#[test]
	fn placeholder_policy_does_not_spawn_a_world_pickup() -> anyhow::Result<()> {
		let (mut world, owner, _) = drop_world(queued_loadout());
		run_drop(&mut world, owner, 2, InventoryDropPolicy::PLACEHOLDER)?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn pickup_flag_spawns_a_stash_without_touching_combat_hud() -> anyhow::Result<()> {
		let (mut world, owner, bag) = drop_world(queued_loadout());
		let policy =
			InventoryDropPolicy { spawn_world_pickup: true, ..InventoryDropPolicy::PLACEHOLDER };
		run_drop(&mut world, owner, 2, policy)?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		let leftover = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(leftover.weapons, vec![1]);
		Ok(())
	}

	#[test]
	fn missing_index_is_refused() -> anyhow::Result<()> {
		let (mut world, owner, bag) = drop_world(queued_loadout());
		run_drop(&mut world, owner, 99, InventoryDropPolicy::PLACEHOLDER)?;
		assert_eq!(
			world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?.items.len(),
			5
		);
		let refusals = world.resource_mut::<Messages<InventoryDropRefused>>().drain().count();
		assert_eq!(refusals, 1);
		Ok(())
	}
}
