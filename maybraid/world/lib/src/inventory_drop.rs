//! Apply [`InventoryDropRequest`] to a host bag and refresh the live loadout.
//!
//! Combat HUD is not consulted. Cycle-queue fixup lives on
//! [`character_items::Inventory::try_drop`]; this adapter persists the leftover
//! bag through [`WorldPlayerLoadout`] and optionally spawns a world stash when
//! the request's placeholder policy asks for a pickup.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{
	ActiveInventoryDropPolicy, Inventory, InventoryDropPolicy, InventoryDropRefused,
	InventoryDropRequest, InventoryDropped, InventoryMenuDropped,
};
use player::{look_forward, PlayerLook};
use world_player::Player as VegetationPlayer;

use crate::stash::{spawn_world_stash, WorldStashSettings};
use crate::weapon::WorldPlayerLoadout;

/// Spawn just outside the capsule, into the third-person gap beside the overlay.
const TOSS_START_FORWARD: f32 = 0.55;
const TOSS_START_RIGHT: f32 = 0.28;
const TOSS_START_Y: f32 = 0.95;
const TOSS_START_FAN: f32 = 0.22;
const TOSS_SPEED: f32 = 3.6;
const TOSS_CAMERA_RIGHT: f32 = 1.4;
const TOSS_UP: f32 = 4.2;
const TOSS_SPREAD: f32 = 2.2;
const TOSS_SPIN: f32 = 8.0;
const TOSS_GRAVITY: f32 = -9.81;
const TOSS_FLOOR_Y: f32 = 0.05;
/// Golden-angle step so sequential launches do not share a velocity.
const TOSS_GOLDEN: f32 = 2.399_963_2;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InventoryDropSystems;

/// Sequential menu-drop index. Each confirm gets a different launch velocity.
#[derive(Resource, Default, Debug)]
pub(crate) struct MenuDropScatter(u32);

/// Ballistic toss integrated on [`Time`] so it still falls while gameplay is paused.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct StashToss {
	pub(crate) velocity: Vec3,
	floor_y: f32,
}

pub(crate) fn configure(app: &mut App) {
	app.insert_resource(ActiveInventoryDropPolicy(InventoryDropPolicy {
		spawn_world_pickup: true,
		..InventoryDropPolicy::PLACEHOLDER
	}))
	.init_resource::<MenuDropScatter>()
	.add_message::<InventoryDropRequest>()
	.add_message::<InventoryDropped>()
	.add_message::<InventoryDropRefused>()
	.add_message::<InventoryMenuDropped>()
	.configure_sets(Update, InventoryDropSystems)
	.add_systems(
		Update,
		(apply_inventory_drop_requests, spawn_menu_inventory_pickups, tick_stash_tosses)
			.chain()
			.in_set(InventoryDropSystems),
	);
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn apply_inventory_drop_requests(
	mut requests: MessageReader<InventoryDropRequest>,
	mut dropped: MessageWriter<InventoryDropped>,
	mut refused: MessageWriter<InventoryDropRefused>,
	mut loadout: Option<ResMut<WorldPlayerLoadout>>,
	users: Query<(&InventoryUser, Has<VegetationPlayer>)>,
	mut bags: Query<&mut Inventory>,
) {
	for request in requests.read() {
		let Ok((user, is_player)) = users.get(request.owner) else {
			refused.write(InventoryDropRefused {
				owner: request.owner,
				index: request.index,
				reason: character_items::InventoryDropError::MissingIndex,
			});
			continue;
		};
		let owner = request.owner;
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
				if is_player {
					if let Some(loadout) = loadout.as_deref_mut() {
						// Leave AppliedWorldPlayerLoadout stale so arm_world_player
						// re-holds the promoted primary and refreshes clothing/stats.
						loadout.retarget_inventory(bag.clone());
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

fn spawn_menu_inventory_pickups(
	mut drops: MessageReader<InventoryDropped>,
	mut commands: Commands,
	settings: Option<Res<WorldStashSettings>>,
	assets: Option<Res<AssetServer>>,
	mut scatter: ResMut<MenuDropScatter>,
	owners: Query<(&Transform, Option<&PlayerLook>)>,
	cameras: Query<&Transform, With<Camera3d>>,
) {
	let camera = cameras.iter().next();
	let assets = assets.as_deref();
	let policy = settings
		.as_ref()
		.map(|settings| settings.ephemeral_policy())
		.unwrap_or_default();
	for drop in drops.read() {
		if !drop.outcome.spawn_world_pickup {
			continue;
		}
		let Ok((pose, look)) = owners.get(drop.owner) else {
			continue;
		};
		let (forward, right) = menu_drop_axes(pose, look, camera);
		let origin = pose.translation;
		let n = scatter.0;
		scatter.0 = scatter.0.wrapping_add(1);
		let start = menu_drop_start(origin, forward, right, n);
		let velocity = menu_drop_impulse(forward, right, n);
		let Some(host) = spawn_world_stash(
			&mut commands,
			Transform::from_translation(start),
			visual_drop_bag(drop.outcome.item.clone()),
			policy,
			assets,
		) else {
			continue;
		};
		commands
			.entity(host)
			.insert(StashToss { velocity, floor_y: origin.y + TOSS_FLOOR_Y });
	}
}

fn tick_stash_tosses(
	time: Res<Time>,
	mut commands: Commands,
	mut tosses: Query<(Entity, &mut Transform, &mut StashToss)>,
) {
	let dt = time.delta_secs();
	if dt <= 0.0 {
		return;
	}
	for (entity, mut transform, mut toss) in &mut tosses {
		toss.velocity.y += TOSS_GRAVITY * dt;
		transform.translation += toss.velocity * dt;
		transform.rotate_y(dt * TOSS_SPIN);
		if transform.translation.y > toss.floor_y || toss.velocity.y > 0.0 {
			continue;
		}
		transform.translation.y = toss.floor_y;
		commands.entity(entity).remove::<StashToss>();
	}
}

fn menu_drop_axes(
	player: &Transform,
	look: Option<&PlayerLook>,
	camera: Option<&Transform>,
) -> (Vec3, Vec3) {
	let player_forward = flatten_xz(player.forward().as_vec3());
	let look_flat = look.map(|look| flatten_xz(look_forward(look)));
	let camera_forward = camera.map(|camera| flatten_xz(-camera.local_z().as_vec3()));
	let forward = look_flat
		.filter(|dir| dir.length_squared() > 0.5)
		.or_else(|| (player_forward.length_squared() > 0.5).then_some(player_forward))
		.or(camera_forward)
		.unwrap_or(Vec3::NEG_Z);

	let player_right = flatten_xz(player.right().as_vec3());
	let camera_right = camera.map(|camera| flatten_xz(camera.right().as_vec3()));
	let right = camera_right
		.filter(|dir| dir.length_squared() > 0.5)
		.or_else(|| (player_right.length_squared() > 0.5).then_some(player_right))
		.unwrap_or_else(|| {
			let derived = flatten_xz(forward.cross(Vec3::Y));
			if derived.length_squared() > 0.5 {
				derived
			} else {
				Vec3::X
			}
		});

	(forward, right)
}

fn flatten_xz(dir: Vec3) -> Vec3 {
	let flat = Vec3::new(dir.x, 0.0, dir.z);
	if flat.length_squared() > 1e-6 {
		flat.normalize()
	} else {
		Vec3::ZERO
	}
}

fn menu_drop_start(origin: Vec3, forward: Vec3, right: Vec3, n: u32) -> Vec3 {
	let angle = n as f32 * TOSS_GOLDEN;
	let fan = (right * angle.cos() + forward * angle.sin()) * TOSS_START_FAN;
	origin + forward * TOSS_START_FORWARD + right * TOSS_START_RIGHT + fan + Vec3::Y * TOSS_START_Y
}

fn menu_drop_impulse(forward: Vec3, right: Vec3, n: u32) -> Vec3 {
	let angle = n as f32 * TOSS_GOLDEN;
	let spread = TOSS_SPREAD * (1.0 + n as f32 * 0.25).min(2.6);
	let lateral = (right * angle.cos() + forward * angle.sin()) * spread;
	forward * TOSS_SPEED + right * TOSS_CAMERA_RIGHT + lateral + Vec3::Y * TOSS_UP
}

fn visual_drop_bag(item: character_items::InventoryItem) -> Inventory {
	let mut bag = Inventory {
		items: vec![item],
		clothing: Vec::new(),
		weapons: Vec::new(),
		skills: Vec::new(),
		tanzes: Vec::new(),
	};
	let _ = bag.toggle(0);
	bag
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use character_inventory_user::InventoryUser;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryDrop, InventoryDropPolicy,
		InventoryDropped, InventoryItem, ItemColor, SkillMapKind, SkillMapSpec,
	};
	use character_persist::{CharacterId, SaveRoot};
	use characters::CharacterAppearance;
	use firearm_user::FirearmUser;
	use grenade_user::GrenadeUser;
	use maybraid_character_controller::InventoryCyclingCategory;

	use super::*;
	use crate::inventory_cycling::ActiveInventoryCyclingCategory;
	use crate::stash::WorldStash;
	use crate::weapon::{
		hold_primary_weapon, teardown_held_weapon, AppliedWorldPlayerLoadout, WorldPlayerLoadout,
	};

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
			tanzes: Vec::new(),
		}
	}

	fn drop_world(inventory: Inventory) -> (World, Entity, Entity) {
		let mut world = World::new();
		world.init_resource::<Messages<InventoryDropRequest>>();
		world.init_resource::<Messages<InventoryDropped>>();
		world.init_resource::<Messages<InventoryDropRefused>>();
		world.init_resource::<Messages<InventoryMenuDropped>>();
		world.init_resource::<ActiveInventoryCyclingCategory>();
		world.init_resource::<MenuDropScatter>();
		world.init_resource::<Time>();
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

	fn gun_then_grenade() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::standard_grenade(),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		}
	}

	#[test]
	fn drop_equipped_primary_replaces_the_live_weapon() -> anyhow::Result<()> {
		let (mut world, owner, _) = drop_world(gun_then_grenade());
		let held = world.spawn_empty().id();
		let armed = world.resource::<WorldPlayerLoadout>().clone();
		world
			.entity_mut(owner)
			.insert((FirearmUser::holding(held), AppliedWorldPlayerLoadout(armed)));
		run_drop(&mut world, owner, 0, InventoryDropPolicy::PLACEHOLDER)?;

		let applied = world
			.get::<AppliedWorldPlayerLoadout>(owner)
			.ok_or_else(|| anyhow::anyhow!("applied"))?;
		let loadout = world.resource::<WorldPlayerLoadout>();
		assert_eq!(
			applied.0.inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		assert!(
			matches!(loadout.inventory.primary_weapon(), Some(InventoryItem::Grenade { .. })),
			"loadout must promote the leftover grenade"
		);
		assert_ne!(
			&applied.0, loadout,
			"adapter must not mark the leftover applied; arm_world_player should reconcile"
		);
		assert_eq!(
			world.get::<FirearmUser>(owner).map(|user| user.held),
			Some(held),
			"live hold stays stale until reconciliation"
		);

		let leftover = loadout.inventory.clone();
		world
			.run_system_once(
				move |mut commands: Commands,
				      users: Query<(Entity, Option<&FirearmUser>, Option<&GrenadeUser>)>| {
					let Ok((player, firearm, grenade)) = users.get(owner) else {
						return;
					};
					teardown_held_weapon(&mut commands, player, firearm, grenade);
					hold_primary_weapon(&mut commands, player, &leftover);
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(
			!world.entities().contains(held),
			"dropping the equipped primary must despawn the held gun"
		);
		assert!(world.get::<FirearmUser>(owner).is_none());
		assert!(
			world.get::<GrenadeUser>(owner).is_some(),
			"live weapon must change to the leftover primary"
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
		use crate::stash::WorldStashSettings;

		let (mut world, owner, bag) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		let policy =
			InventoryDropPolicy { spawn_world_pickup: true, ..InventoryDropPolicy::PLACEHOLDER };
		run_drop(&mut world, owner, 2, policy)?;
		spawn_menu_drops(&mut world)?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		let leftover = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(leftover.weapons, vec![1]);
		Ok(())
	}

	fn npc_drop_world(inventory: Inventory, at: Vec3) -> (World, Entity, Entity) {
		let mut world = World::new();
		world.init_resource::<Messages<InventoryDropRequest>>();
		world.init_resource::<Messages<InventoryDropped>>();
		world.init_resource::<Messages<InventoryDropRefused>>();
		world.init_resource::<Messages<InventoryMenuDropped>>();
		world.init_resource::<MenuDropScatter>();
		world.init_resource::<Time>();
		let bag = world.spawn(inventory).id();
		let owner = world
			.spawn((InventoryUser::carrying(bag), Transform::from_translation(at)))
			.id();
		(world, owner, bag)
	}

	#[test]
	fn npc_drop_spawns_a_pickup_from_the_owner_not_the_player() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;

		let npc_at = Vec3::new(40.0, 0.0, -12.0);
		let (mut world, npc, npc_bag) = npc_drop_world(queued_loadout(), npc_at);
		world.init_resource::<WorldStashSettings>();
		let player_bag = world.spawn(queued_loadout()).id();
		world.spawn((
			VegetationPlayer,
			InventoryUser::carrying(player_bag),
			Transform::from_xyz(3.0, 0.0, 1.0),
		));
		let policy =
			InventoryDropPolicy { spawn_world_pickup: true, ..InventoryDropPolicy::PLACEHOLDER };
		run_drop(&mut world, npc, 2, policy)?;
		spawn_menu_drops(&mut world)?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		let leftover = world.get::<Inventory>(npc_bag).ok_or_else(|| anyhow::anyhow!("npc bag"))?;
		assert_eq!(leftover.weapons, vec![1]);
		let player_leftover = world
			.get::<Inventory>(player_bag)
			.ok_or_else(|| anyhow::anyhow!("player bag"))?;
		assert_eq!(player_leftover.items.len(), 5);
		let start = menu_drop_start(npc_at, Vec3::NEG_Z, Vec3::X, 0);
		let pose = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| *transform)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!(
			(pose.translation - start).length() < 1e-4,
			"NPC pickup should leave from the owner, got {pose:?} expected {start:?}"
		);
		Ok(())
	}

	#[test]
	fn npc_drop_without_a_player_still_spawns_a_pickup() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;

		let npc_at = Vec3::new(-8.0, 0.0, 22.0);
		let (mut world, npc, _) = npc_drop_world(queued_loadout(), npc_at);
		world.init_resource::<WorldStashSettings>();
		let policy =
			InventoryDropPolicy { spawn_world_pickup: true, ..InventoryDropPolicy::PLACEHOLDER };
		run_drop(&mut world, npc, 2, policy)?;
		spawn_menu_drops(&mut world)?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		let start = menu_drop_start(npc_at, Vec3::NEG_Z, Vec3::X, 0);
		let pose = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| *transform)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!((pose.translation - start).length() < 1e-4);
		Ok(())
	}

	fn write_accepted_drop(
		world: &mut World,
		owner: Entity,
		bag: Entity,
		item: InventoryItem,
		index: usize,
	) -> anyhow::Result<()> {
		world
			.run_system_once(move |mut writer: MessageWriter<InventoryDropped>| {
				writer.write(InventoryDropped {
					owner,
					bag,
					outcome: InventoryDrop {
						slot: item.slot(),
						item: item.clone(),
						index,
						was_selected: false,
						was_primary: false,
						spawn_world_pickup: true,
					},
				});
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn spawn_menu_drops(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(super::spawn_menu_inventory_pickups)
			.map_err(|error| anyhow::anyhow!("{error:?}"))
	}

	#[test]
	fn menu_drop_spawns_an_ephemeral_stash_with_despawn_cool_off() -> anyhow::Result<()> {
		use crate::stash::{StashPolicy, WorldStashSettings};
		use damage::DespawnAfter;

		let (mut world, owner, bag) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		write_accepted_drop(&mut world, owner, bag, queued_loadout().items[2].clone(), 2)?;
		spawn_menu_drops(&mut world)?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		let ttl = world
			.query::<&DespawnAfter>()
			.iter(&world)
			.next()
			.ok_or_else(|| anyhow::anyhow!("despawn cool-off"))?;
		assert!((ttl.remaining_secs() - StashPolicy::default().loot_secs).abs() < 0.01);
		Ok(())
	}

	#[test]
	fn menu_drop_tosses_forward_and_right_of_the_player() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;

		let (mut world, owner, bag) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		write_accepted_drop(&mut world, owner, bag, queued_loadout().items[2].clone(), 2)?;
		spawn_menu_drops(&mut world)?;
		let origin = Vec3::new(3.0, 0.0, 1.0);
		let start = menu_drop_start(origin, Vec3::NEG_Z, Vec3::X, 0);
		let pose = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| *transform)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!((pose.translation - start).length() < 1e-4);
		let toss = world
			.query::<&StashToss>()
			.iter(&world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("stash toss"))?;
		// Identity facing is −Z, right is +X — into the open strip, not underfoot.
		assert!(toss.velocity.z < -2.0);
		assert!(toss.velocity.x > 2.0);
		assert!(
			toss.velocity.y > 2.5,
			"toss should leave the hand with upward speed, got {toss:?}"
		);
		Ok(())
	}

	#[test]
	fn menu_drop_impulse_follows_player_yaw() {
		let origin = Vec3::new(3.0, 0.0, 1.0);
		let yawed = Transform::from_translation(origin)
			.with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
		let (forward, right) = menu_drop_axes(&yawed, None, None);
		let impulse = menu_drop_impulse(forward, right, 0);
		assert!(impulse.x < -2.0, "yaw π/2 faces −X, got {impulse:?}");
		assert!(impulse.z < -0.5, "camera-right fallback is −Z, got {impulse:?}");
		assert!(impulse.y > 2.5);
	}

	#[test]
	fn menu_drop_prefers_look_yaw_over_identity_body() {
		let player = Transform::from_xyz(0.0, 0.0, 0.0);
		let look = PlayerLook { yaw: std::f32::consts::FRAC_PI_2, ..Default::default() };
		let (forward, _) = menu_drop_axes(&player, Some(&look), None);
		assert!(
			forward.dot(Vec3::NEG_X) > 0.9,
			"PlayerLook yaw should beat locked body rotation, got {forward:?}"
		);
	}

	#[test]
	fn menu_drop_uses_camera_right_for_the_open_strip() {
		let player = Transform::from_xyz(0.0, 0.0, 0.0);
		let camera =
			Transform::from_xyz(-4.0, 2.0, 0.0).looking_at(Vec3::new(0.0, 0.8, 0.0), Vec3::Y);
		let (forward, right) = menu_drop_axes(&player, None, Some(&camera));
		let impulse = menu_drop_impulse(forward, right, 0);
		let player_right = menu_drop_impulse(Vec3::NEG_Z, Vec3::X, 0);
		assert!(
			right.dot(Vec3::Z) > 0.7,
			"camera on −X looking at the player has +Z right, got {right:?}"
		);
		assert!(
			impulse.z > player_right.z + 1.5,
			"camera-right should pull the toss toward +Z, got {impulse:?} vs {player_right:?}"
		);
		assert!(
			impulse.x < player_right.x - 1.5,
			"should not use player-right +X as the strip, got {impulse:?} vs {player_right:?}"
		);
	}

	#[test]
	fn sequential_menu_drops_launch_apart() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;

		let (mut world, owner, bag) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		let item = queued_loadout().items[2].clone();
		write_accepted_drop(&mut world, owner, bag, item.clone(), 2)?;
		write_accepted_drop(&mut world, owner, bag, item, 2)?;
		spawn_menu_drops(&mut world)?;
		let velocities: Vec<Vec3> =
			world.query::<&StashToss>().iter(&world).map(|toss| toss.velocity).collect();
		assert_eq!(velocities.len(), 2);
		assert!(
			(velocities[0] - velocities[1]).length() > 2.0,
			"sequential drops should not share a launch, got {velocities:?}"
		);
		Ok(())
	}

	#[test]
	fn stash_toss_falls_under_gravity_then_settles() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;
		use std::time::Duration;

		let (mut world, owner, bag) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		write_accepted_drop(&mut world, owner, bag, queued_loadout().items[2].clone(), 2)?;
		spawn_menu_drops(&mut world)?;
		let start = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| transform.translation)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;

		world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(0.15));
		world
			.run_system_once(tick_stash_tosses)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let mid = world
			.query::<(&Transform, &StashToss, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, toss, _)| (transform.translation, toss.velocity))
			.ok_or_else(|| anyhow::anyhow!("in-flight toss"))?;
		assert!(mid.0.y > start.y, "first beat should still be climbing, got {mid:?}");
		assert!(mid.1.y < TOSS_UP, "gravity should bleed upward speed, got {mid:?}");

		for _ in 0..20 {
			world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(0.1));
			world
				.run_system_once(tick_stash_tosses)
				.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		}
		assert_eq!(world.query::<&StashToss>().iter(&world).count(), 0);
		let landed = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| transform.translation)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!((landed.y - TOSS_FLOOR_Y).abs() < 1e-3);
		assert!((Vec2::new(landed.x, landed.z) - Vec2::new(start.x, start.z)).length() > 0.8);
		Ok(())
	}

	#[test]
	fn request_policy_is_enforced_at_the_bag_mutation() -> anyhow::Result<()> {
		let (mut world, owner, bag) = drop_world(queued_loadout());
		let no_equipped =
			InventoryDropPolicy { allow_equipped: false, ..InventoryDropPolicy::PLACEHOLDER };
		run_drop(&mut world, owner, 1, no_equipped)?;
		assert_eq!(
			world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?.weapons,
			vec![1, 2]
		);
		let refusals = world.resource_mut::<Messages<InventoryDropRefused>>().drain().count();
		assert_eq!(refusals, 1);
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
