//! Starter held kit for the vegetation player capsule.

use bevy::prelude::*;
use character_inventory_user::{spawn_bag, InventoryUser};
use character_items::{CharacterSheet, Inventory, InventoryItem};
use characters::{CharacterAppearance, CharacterRoot};
use damage::Health;
use firearm_user::{
	live_weapon_from_stats, spawn_held_firearm, spawn_held_kit, spawn_reticle, FirearmUser,
	FirearmUserSettings, FirearmUserSystems, GeneratedFirearm, WeaponSwap,
};
use grenade_user::{spawn_held_grenade, GrenadeThrow, GrenadeUser, GrenadeUserSettings};
use grenades::GrenadeDetonated;
use maybraid_character_controller::{
	CharacterControlSystems, CharacterIntent, InventoryCyclingCategory,
};

use crate::inventory_cycling::ActiveInventoryCyclingCategory;
use maybraid_skill_map::{spawn_skill_maps, SkillMapEquip, SkillMapSystems};
use maybraid_vfx::{SpawnVfxExt, VfxLibrary, VfxSpawn};
use player::{
	apply_character_mobility, CameraFollow, Player as MaybraidPlayer, PlayerCameraAim, PlayerLook,
	PlayerUse, PlayerVisual as MaybraidPlayerVisual, PlayerYawOwner,
};
use world_player::{
	CharacterSpecies, Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual,
	PlaygroundMode, RequestSetCharacter, RequestSetCharacterAppearance,
};

use crate::control::{InventoryEditCameraFollow, WorldGameplayEnabled};

/// Persisted character appearance, worn clothing, stats, and primary weapon for world entry.
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct WorldPlayerLoadout {
	pub key: String,
	pub name: String,
	pub appearance: CharacterAppearance,
	pub inventory: Inventory,
}

impl WorldPlayerLoadout {
	pub fn new(
		key: impl Into<String>,
		appearance: CharacterAppearance,
		inventory: Inventory,
	) -> Self {
		let appearance = appearance.with_inventory_clothing(&inventory);
		Self { key: key.into(), name: String::from("Player"), appearance, inventory }
	}

	/// Character display name shown on the vitals HUD. Empty input stays `"Player"`.
	pub fn with_name(mut self, name: impl Into<String>) -> Self {
		let name = name.into();
		let trimmed = name.trim();
		self.name = if trimmed.is_empty() { String::from("Player") } else { trimmed.to_string() };
		self
	}

	/// Replace the bag and rebuild worn garments from the new wear list.
	pub fn retarget_inventory(&mut self, inventory: Inventory) {
		let name = self.name.clone();
		*self = Self::new(self.key.clone(), self.appearance.clone(), inventory).with_name(name);
	}
}

#[derive(Component)]
pub(crate) struct AppliedWorldPlayerLoadout(pub WorldPlayerLoadout);

/// The replacement lifecycle already queued this body's configured visual.
#[derive(Component)]
pub(crate) struct WorldPlayerAppearanceRequested;

type WorldPlayerEquipment<'a> = (
	Entity,
	Option<&'a FirearmUser>,
	Option<&'a GrenadeUser>,
	Option<&'a InventoryUser>,
	Option<&'a maybraid_skill_map::SkillMapUser>,
	Option<&'a AppliedWorldPlayerLoadout>,
	Has<WorldPlayerAppearanceRequested>,
);

type WorldPlayerVisual<'a> = (Entity, &'a ChildOf, Has<MaybraidPlayerVisual>);

/// Give the world player its selected loadout once the character visual exists.
///
/// [`firearm_user`] fire/pose query [`MaybraidPlayer`] / [`PlayerLook`]. The
/// player-crate locomotion controller is stamped in [`crate::control`] so
/// column buoyancy runs; this only arms kit / identity once the visual exists.
pub(crate) fn arm_world_player(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	inventory_edit: Option<Res<InventoryEditCameraFollow>>,
	loadout: Option<Res<WorldPlayerLoadout>>,
	players: Query<WorldPlayerEquipment<'_>, With<VegetationPlayer>>,
	visuals: Query<WorldPlayerVisual<'_>, (With<VegetationPlayerVisual>, With<CharacterRoot>)>,
) {
	if !gameplay.0 && !inventory_edit.is_some_and(|edit| edit.0) {
		return;
	}
	for (
		player,
		firearm_user,
		grenade_user,
		inventory_user,
		skill_map_user,
		applied,
		appearance_requested,
	) in &players
	{
		let Some((visual, _, presented)) =
			visuals.iter().find(|(_, child, _)| child.parent() == player)
		else {
			continue;
		};
		if !presented {
			commands.entity(visual).insert((MaybraidPlayerVisual, PlayerYawOwner::Wish));
		}
		if skill_map_user.is_none() {
			spawn_skill_maps(&mut commands, player);
		}
		if loadout.is_none()
			&& applied.is_none()
			&& (firearm_user.is_some() || grenade_user.is_some())
		{
			continue;
		}
		if loadout
			.as_ref()
			.zip(applied)
			.is_some_and(|(loadout, applied)| loadout.as_ref() == &applied.0)
		{
			continue;
		}
		commands.entity(player).insert((
			MaybraidPlayer,
			PlayerLook::default(),
			PlayerCameraAim::default(),
			PlayerYawOwner::Wish,
		));
		if *mode == PlaygroundMode::Character && gameplay.0 {
			commands.entity(player).insert(CameraFollow);
		}

		teardown_held_weapon(&mut commands, player, firearm_user, grenade_user);
		if let Some(user) = inventory_user {
			commands.entity(user.bag).try_despawn();
			commands.entity(player).remove::<InventoryUser>();
		}
		let Some(loadout) = loadout.as_ref() else {
			commands
				.entity(player)
				.remove::<(AppliedWorldPlayerLoadout, WorldPlayerAppearanceRequested)>();
			commands.entity(player).insert(Health::default());
			apply_character_mobility(&mut commands, player, 1.0, 1.0);
			if applied.is_some() {
				commands.spawn(RequestSetCharacter { species: CharacterSpecies::Braidman });
			}
			spawn_held_firearm(&mut commands, player);
			continue;
		};

		let sheet = loadout.inventory.character_sheet();
		commands.entity(player).insert((
			Health::from_max(f32::from(sheet.health.max(1))),
			AppliedWorldPlayerLoadout(loadout.as_ref().clone()),
			Name::new(loadout.name.clone()),
		));
		apply_character_mobility(
			&mut commands,
			player,
			f32::from(sheet.running) / f32::from(CharacterSheet::BASE.running),
			f32::from(sheet.jump) / f32::from(CharacterSheet::BASE.jump),
		);
		spawn_bag(&mut commands, player, loadout.inventory.clone());
		commands.entity(player).insert(skill_map_equip_from(&loadout.inventory));
		if !appearance_requested {
			commands
				.spawn(RequestSetCharacterAppearance { appearance: loadout.appearance.clone() });
		}
		commands.entity(player).remove::<WorldPlayerAppearanceRequested>();
		hold_primary_weapon(&mut commands, player, &loadout.inventory);
	}
}

fn skill_map_equip_from(inventory: &Inventory) -> SkillMapEquip {
	SkillMapEquip::from_spec(inventory.primary_skill_map().and_then(InventoryItem::skill_map_spec))
}

pub(crate) fn hold_primary_weapon(commands: &mut Commands, player: Entity, inventory: &Inventory) {
	match inventory.primary_weapon() {
		Some(InventoryItem::Firearm { spec, stats }) => {
			let sheet = inventory.character_sheet();
			let live = live_weapon_from_stats(*stats, sheet.damage).with_weapon_identity(spec);
			spawn_held_kit(
				commands,
				player,
				FirearmUserSettings::default(),
				GeneratedFirearm::from_spec(*spec),
				live,
			);
		}
		Some(InventoryItem::Grenade { .. }) => {
			spawn_held_grenade(commands, player, GrenadeUserSettings::default());
		}
		_ => {}
	}
}

pub(crate) fn teardown_held_weapon(
	commands: &mut Commands,
	player: Entity,
	firearm: Option<&FirearmUser>,
	grenade: Option<&GrenadeUser>,
) {
	if let Some(firearm) = firearm {
		commands.entity(firearm.held).try_despawn();
	}
	if let Some(grenade) = grenade {
		commands.entity(grenade.held).try_despawn();
	}
	if firearm.is_some() || grenade.is_some() {
		commands
			.entity(player)
			.remove::<(FirearmUser, GrenadeUser, GrenadeThrow, PlayerUse)>();
	}
}

/// Start the holster motion; the kit changes at the dip.
fn begin_weapon_swap(
	mut intents: MessageReader<CharacterIntent>,
	gameplay: Res<WorldGameplayEnabled>,
	category: Res<ActiveInventoryCyclingCategory>,
	mut commands: Commands,
	players: Query<
		(Entity, &InventoryUser, Has<WeaponSwap>, Option<&GrenadeThrow>),
		With<VegetationPlayer>,
	>,
	bags: Query<&Inventory>,
) {
	let intents = intents.read().collect::<Vec<_>>();
	let mut weapon_dir = None;
	for intent in &intents {
		match intent {
			CharacterIntent::SwapActive => {
				weapon_dir = Some(1i8);
				break;
			}
			CharacterIntent::CycleEquippedQueue(dir) if *dir != 0 => {
				weapon_dir = Some(*dir);
				break;
			}
			_ => {}
		}
	}
	let weapon_cycle = category.0 == InventoryCyclingCategory::Weapons && weapon_dir.is_some();
	if !gameplay.0 || !weapon_cycle {
		return;
	}
	let reverse = weapon_dir.is_some_and(|dir| dir < 0);
	for (player, user, swapping, grenade) in &players {
		if swapping || grenade.is_some_and(GrenadeThrow::blocks_swap) {
			continue;
		}
		let Ok(bag) = bags.get(user.bag) else {
			continue;
		};
		if bag.weapons.len() < 2 {
			continue;
		}
		commands.entity(player).insert(WeaponSwap::with_direction(reverse));
	}
}

/// Replace the held kit once the old gun has dipped out of the way.
fn commit_weapon_swap(
	mut commands: Commands,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	assets: Option<Res<AssetServer>>,
	mut players: Query<
		(Entity, &InventoryUser, Option<&FirearmUser>, Option<&GrenadeUser>, &mut WeaponSwap),
		With<VegetationPlayer>,
	>,
	mut bags: Query<&mut Inventory>,
) {
	for (player, user, firearm, grenade, mut swap) in &mut players {
		if !swap.ready_to_swap() {
			continue;
		}
		let Ok(mut bag) = bags.get_mut(user.bag) else {
			continue;
		};
		let stepped = if swap.reverse { bag.cycle_weapons(-1) } else { bag.swap_active() };
		if !stepped {
			swap.mark_swapped();
			continue;
		}
		let snapshot = bag.clone();
		teardown_held_weapon(&mut commands, player, firearm, grenade);
		if let Some(loadout) = loadout.as_deref_mut() {
			loadout.retarget_inventory(snapshot.clone());
			commands.entity(player).insert(AppliedWorldPlayerLoadout(loadout.clone()));
		}
		if assets.is_some() {
			hold_primary_weapon(&mut commands, player, &snapshot);
		}
		swap.mark_swapped();
	}
}

fn sync_skill_map_equip(
	mut users: Query<(&InventoryUser, &mut SkillMapEquip)>,
	bags: Query<&Inventory>,
) {
	for (user, mut equip) in &mut users {
		let Ok(bag) = bags.get(user.bag) else {
			continue;
		};
		let next = skill_map_equip_from(bag);
		if *equip != next {
			*equip = next;
		}
	}
}

fn spawn_world_reticle(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
) {
	spawn_reticle(&mut commands, &mut meshes, &mut materials);
}

fn spawn_detonation_vfx(
	mut detonations: MessageReader<GrenadeDetonated>,
	mut commands: Commands,
	library: Option<Res<VfxLibrary>>,
) {
	let Some(library) = library else {
		return;
	};
	for event in detonations.read() {
		commands.spawn_vfx(
			&library.fiery_explosion,
			VfxSpawn {
				transform: Transform::from_translation(event.position),
				scale: event.effect.scale,
				intensity: event.effect.intensity,
				playback: event.effect.playback,
				seed: event.effect.seed,
				..default()
			},
		);
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Startup, spawn_world_reticle).add_systems(
		Update,
		(
			arm_world_player,
			(
				begin_weapon_swap
					.after(crate::inventory_cycling::InventoryCyclingCategorySystems)
					.before(crate::inventory_cycling::InventoryCyclingSystems),
				commit_weapon_swap.after(FirearmUserSystems::Swap),
			)
				.chain()
				.after(CharacterControlSystems)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			sync_skill_map_equip
				.after(crate::inventory_cycling::InventoryCyclingSystems)
				.after(CharacterControlSystems)
				.before(SkillMapSystems::Spawn)
				.run_if(resource_equals(WorldGameplayEnabled(true))),
			spawn_detonation_vfx,
		),
	);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	};
	use characters::{CharacterAppearance, CharacterRoot};
	use world_player::{
		Player as VegetationPlayer, PlayerVisual as VegetationPlayerVisual, PlaygroundMode,
		RequestSetCharacterAppearance,
	};

	use crate::inventory_cycling::ActiveInventoryCyclingCategory;
	use crate::weapon::{arm_world_player, WorldPlayerAppearanceRequested, WorldPlayerLoadout};
	use crate::WorldGameplayEnabled;

	#[test]
	fn world_loadout_keeps_primary_weapon_and_worn_clothing() {
		let inventory = Inventory::with_starter_outfit(vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Tattered,
				ItemColor::Green,
			),
			InventoryItem::firearm(FirearmMesh::Reltor),
		]);
		let loadout = WorldPlayerLoadout::new("active", CharacterAppearance::default(), inventory)
			.with_name("Ada");
		assert_eq!(loadout.name, "Ada");
		assert_eq!(
			WorldPlayerLoadout::new("active", CharacterAppearance::default(), Inventory::default())
				.with_name("  ")
				.name,
			"Player"
		);
		assert_eq!(
			loadout.inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		if let CharacterAppearance::Braidman(config) = loadout.appearance {
			assert_eq!(config.clothing, vec![ClothingMesh::Pants]);
			assert_eq!(
				config.colors.clothing.first().map(|clothing| clothing.color),
				Some(ItemColor::Green)
			);
		}
	}

	#[test]
	fn respawn_does_not_replace_the_visual_twice() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(
			WorldPlayerLoadout::new("active", CharacterAppearance::default(), Inventory::default())
				.with_name("Ada"),
		);
		let player = world.spawn((VegetationPlayer, WorldPlayerAppearanceRequested)).id();
		world.spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));

		world
			.run_system_once(arm_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.query::<&RequestSetCharacterAppearance>().iter(&world).count(), 0);
		assert!(world.get::<WorldPlayerAppearanceRequested>(player).is_none());
		assert!(world.get::<maybraid_skill_map::SkillMapUser>(player).is_some());
		assert_eq!(world.get::<Name>(player).map(Name::as_str), Some("Ada"));
		Ok(())
	}

	#[test]
	fn pause_inventory_edit_still_applies_loadout() -> anyhow::Result<()> {
		use crate::InventoryEditCameraFollow;

		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(InventoryEditCameraFollow(true));
		world.insert_resource(
			WorldPlayerLoadout::new("active", CharacterAppearance::default(), Inventory::default())
				.with_name("Ada"),
		);
		let player = world.spawn(VegetationPlayer).id();
		world.spawn((VegetationPlayerVisual, CharacterRoot, ChildOf(player)));
		world
			.run_system_once(arm_world_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<Name>(player).map(Name::as_str), Some("Ada"));
		Ok(())
	}

	fn two_gun_bag() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::firearm(FirearmMesh::Reltor),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
		}
	}

	fn two_map_bag() -> Inventory {
		use character_items::{SkillMapKind, SkillMapSpec};

		Inventory {
			items: vec![
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Fireball, 1)),
				InventoryItem::skill_map(SkillMapSpec::new(SkillMapKind::Dumbwave, 2)),
			],
			clothing: Vec::new(),
			weapons: Vec::new(),
			skills: vec![0, 1],
		}
	}

	#[test]
	fn same_frame_category_then_y_starts_weapon_swap() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap};
		use maybraid_character_controller::{CharacterIntent, InventoryCyclingCategory};

		use crate::inventory_cycling::{
			apply_inventory_category_selection, ActiveInventoryCyclingCategory,
		};
		use crate::weapon::begin_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(game_commands::command::TextEntryFocus(false));
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::SkillMaps));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let player = world
			.spawn((VegetationPlayer, InventoryUser::carrying(bag), FirearmUser::holding(held)))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SelectInventoryCyclingCategory(
					InventoryCyclingCategory::Weapons,
				));
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_inventory_category_selection)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<WeaponSwap>(player).is_some());
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		Ok(())
	}

	#[test]
	fn y_starts_a_swap_without_changing_the_kit() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap};
		use maybraid_character_controller::CharacterIntent;

		use crate::weapon::begin_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(ActiveInventoryCyclingCategory(
			maybraid_character_controller::InventoryCyclingCategory::Weapons,
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let player = world
			.spawn((VegetationPlayer, InventoryUser::carrying(bag), FirearmUser::holding(held)))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<WeaponSwap>(player).is_some());
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		assert!(world.entities().contains(held));
		Ok(())
	}

	#[test]
	fn reverse_queue_step_starts_a_swap_without_rotating_early() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap};
		use maybraid_character_controller::CharacterIntent;

		use crate::weapon::begin_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(ActiveInventoryCyclingCategory(
			maybraid_character_controller::InventoryCyclingCategory::Weapons,
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let player = world
			.spawn((VegetationPlayer, InventoryUser::carrying(bag), FirearmUser::holding(held)))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::CycleEquippedQueue(-1));
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let swap = world.get::<WeaponSwap>(player).ok_or_else(|| anyhow::anyhow!("swap"))?;
		assert!(swap.reverse);
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		Ok(())
	}

	#[test]
	fn reverse_queue_commits_through_weapon_swap() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap, WEAPON_SWAP_SECS};

		use crate::weapon::commit_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory.clone(),
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		world.spawn((
			VegetationPlayer,
			InventoryUser::carrying(bag),
			FirearmUser::holding(held),
			WeaponSwap {
				elapsed: WEAPON_SWAP_SECS * 0.5,
				duration: WEAPON_SWAP_SECS,
				swapped: false,
				reverse: true,
			},
		));
		world
			.run_system_once(commit_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}

	#[test]
	fn y_swaps_the_queued_primary() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use firearm_user::{FirearmUser, WeaponSwap, WEAPON_SWAP_SECS};

		use crate::weapon::commit_weapon_swap;

		let inventory = two_gun_bag();
		let mut world = World::new();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory.clone(),
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		world.spawn((
			VegetationPlayer,
			InventoryUser::carrying(bag),
			FirearmUser::holding(held),
			WeaponSwap {
				elapsed: WEAPON_SWAP_SECS * 0.5,
				duration: WEAPON_SWAP_SECS,
				swapped: false,
				reverse: false,
			},
		));
		world
			.run_system_once(commit_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert!(!world.entities().contains(held));
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(
			loadout.inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}

	#[test]
	fn grenade_primary_spawns_a_held_visual() -> anyhow::Result<()> {
		use grenade_user::GrenadeUser;

		use crate::weapon::hold_primary_weapon;

		let inventory = Inventory {
			items: vec![InventoryItem::standard_grenade()],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
		};
		let mut world = World::new();
		let player = world.spawn(VegetationPlayer).id();
		world
			.run_system_once(move |mut commands: Commands| {
				hold_primary_weapon(&mut commands, player, &inventory);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<GrenadeUser>(player).is_some());
		assert!(world.get::<firearm_user::FirearmUser>(player).is_none());
		Ok(())
	}

	#[test]
	fn throw_blocks_weapon_swap() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use grenade_user::{GrenadePhase, GrenadeThrow, GrenadeUser};
		use maybraid_character_controller::CharacterIntent;

		use crate::weapon::begin_weapon_swap;

		let inventory = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::standard_grenade(),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(ActiveInventoryCyclingCategory(
			maybraid_character_controller::InventoryCyclingCategory::Weapons,
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let player = world
			.spawn((
				VegetationPlayer,
				InventoryUser::carrying(bag),
				GrenadeUser::holding(held),
				GrenadeThrow {
					phase: GrenadePhase::Windup { age: 0.1 },
					use_latched: true,
					fuse: 2.0,
				},
			))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<firearm_user::WeaponSwap>(player).is_none());
		Ok(())
	}

	#[test]
	fn cooking_allows_weapon_swap() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use grenade_user::{GrenadePhase, GrenadeThrow, GrenadeUser};
		use maybraid_character_controller::CharacterIntent;

		use crate::weapon::begin_weapon_swap;

		let inventory = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::standard_grenade(),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(ActiveInventoryCyclingCategory(
			maybraid_character_controller::InventoryCyclingCategory::Weapons,
		));
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let player = world
			.spawn((
				VegetationPlayer,
				InventoryUser::carrying(bag),
				GrenadeUser::holding(held),
				GrenadeThrow {
					phase: GrenadePhase::Cooking { age: 0.4 },
					use_latched: true,
					fuse: 2.0,
				},
			))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_weapon_swap)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<firearm_user::WeaponSwap>(player).is_some());
		Ok(())
	}

	#[test]
	fn y_cycles_the_presented_skill_map_in_the_selected_category() -> anyhow::Result<()> {
		use character_inventory_user::InventoryUser;
		use character_items::{SkillMapKind, SkillMapSpec};
		use maybraid_character_controller::{CharacterIntent, InventoryCyclingCategory};
		use maybraid_skill_map::SkillMapEquip;

		use crate::inventory_cycling::{
			cycle_equipped_inventory_queues, ActiveInventoryCyclingCategory,
		};
		use crate::weapon::sync_skill_map_equip;

		let inventory = two_map_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(game_commands::command::TextEntryFocus(false));
		world.insert_resource(ActiveInventoryCyclingCategory(InventoryCyclingCategory::SkillMaps));
		world.insert_resource(WorldPlayerLoadout::new(
			"active",
			CharacterAppearance::default(),
			inventory.clone(),
		));
		let bag = world.spawn(inventory).id();
		let player = world
			.spawn((
				VegetationPlayer,
				InventoryUser::carrying(bag),
				SkillMapEquip::from_spec(Some(SkillMapSpec::new(SkillMapKind::Fireball, 1))),
			))
			.id();
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(cycle_equipped_inventory_queues)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_skill_map_equip)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		assert_eq!(
			world.get::<SkillMapEquip>(player).copied(),
			Some(SkillMapEquip::from_spec(Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))))
		);
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(
			loadout.inventory.primary_skill_map().and_then(InventoryItem::skill_map_spec),
			Some(SkillMapSpec::new(SkillMapKind::Dumbwave, 2))
		);
		Ok(())
	}

	#[test]
	fn detonation_vfx_forwards_grenade_effect_scale() -> Result<(), String> {
		use character_items::GrenadeStats;
		use grenades::{GrenadeDetonated, GrenadeEffect};
		use maybraid_vfx::{
			EffectDefinition, FlipbookAsset, ScaleBounds, VfxFlipbooks, VfxInstance, VfxLibrary,
			FIERY_EXPLOSION,
		};

		let mut world = World::new();
		world.init_resource::<Messages<GrenadeDetonated>>();
		let blank = EffectDefinition::new("stub", []);
		let fiery = EffectDefinition::new(FIERY_EXPLOSION, [])
			.with_scale_bounds(ScaleBounds { min: 0.25, max: 20.0 });
		world.insert_resource(VfxLibrary {
			flipbooks: VfxFlipbooks {
				fire: FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				smoke: FlipbookAsset {
					image: Handle::default(),
					grid: UVec2::ONE,
					frame_count: 1,
					playback: 1.0,
				},
				spark: Handle::default(),
			},
			flash: blank.clone(),
			fireball: blank.clone(),
			smoke: blank.clone(),
			sparks: blank,
			fiery_explosion: fiery,
		});
		let stats = GrenadeStats::standard();
		world.resource_mut::<Messages<GrenadeDetonated>>().write(GrenadeDetonated {
			position: Vec3::new(1.0, 2.0, 3.0),
			source: Entity::PLACEHOLDER,
			effect: GrenadeEffect::from_stats(&stats),
			radius: stats.blast_radius,
			damage: stats.blast_damage,
		});
		world
			.run_system_once(super::spawn_detonation_vfx)
			.map_err(|error| format!("{error:?}"))?;
		let mut query = world.query::<(&Transform, &VfxInstance)>();
		let Some((transform, instance)) = query.iter(&world).next() else {
			return Err(String::from("missing detonation vfx"));
		};
		assert_eq!(instance.name, FIERY_EXPLOSION);
		assert!((transform.scale.x - stats.effect_scale).abs() < 1e-4);
		Ok(())
	}
}
