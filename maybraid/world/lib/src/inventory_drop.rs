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
	InventoryDropped, InventoryMenuDropped,
};
use player::{look_forward, PlayerLook};
use world_player::Player as VegetationPlayer;

use crate::stash::{spawn_world_stash, WorldStashSettings};
use crate::weapon::{AppliedWorldPlayerLoadout, WorldPlayerLoadout};

/// Ahead of the player, into the third-person gap beside the inventory overlay.
const TOSS_FORWARD: f32 = 1.15;
const TOSS_CAMERA_RIGHT: f32 = 0.55;
const TOSS_SPREAD: f32 = 0.22;
const TOSS_START_Y: f32 = 0.85;
const TOSS_LAND_Y: f32 = 0.05;
const TOSS_APEX: f32 = 0.55;
const TOSS_SECS: f32 = 0.38;
const TOSS_SPIN: f32 = 7.0;
/// Golden-angle step so sequential landings do not stack.
const TOSS_GOLDEN: f32 = 2.399_963_2;

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct InventoryDropSystems;

/// Sequential menu-drop index. Each confirm lands on a different spot in the ring.
#[derive(Resource, Default, Debug)]
pub(crate) struct MenuDropScatter(u32);

/// Kinematic arc from chest height into the open world view.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct StashToss {
	pub(crate) start: Vec3,
	pub(crate) end: Vec3,
	elapsed: f32,
	duration: f32,
}

pub(crate) fn configure(app: &mut App) {
	app.init_resource::<ActiveInventoryDropPolicy>()
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

fn spawn_menu_inventory_pickups(
	mut drops: MessageReader<InventoryMenuDropped>,
	mut commands: Commands,
	settings: Option<Res<WorldStashSettings>>,
	assets: Option<Res<AssetServer>>,
	mut scatter: ResMut<MenuDropScatter>,
	players: Query<(&Transform, Option<&PlayerLook>), With<VegetationPlayer>>,
	cameras: Query<&Transform, (With<Camera3d>, Without<VegetationPlayer>)>,
) {
	let Some((player, look)) = players.iter().next() else {
		return;
	};
	let camera = cameras.iter().next();
	let (forward, right) = menu_drop_axes(player, look, camera);
	let origin = player.translation;
	let assets = assets.as_deref();
	let policy = settings
		.as_ref()
		.map(|settings| settings.ephemeral_policy())
		.unwrap_or_default();
	for drop in drops.read() {
		let n = scatter.0;
		scatter.0 = scatter.0.wrapping_add(1);
		let start = menu_drop_start(origin);
		let end = menu_drop_landing(origin, forward, right, n);
		let Some(host) = spawn_world_stash(
			&mut commands,
			Transform::from_translation(start),
			visual_drop_bag(drop.item.clone()),
			policy,
			assets,
		) else {
			continue;
		};
		commands
			.entity(host)
			.insert(StashToss { start, end, elapsed: 0.0, duration: TOSS_SECS });
	}
}

fn tick_stash_tosses(
	time: Res<Time>,
	mut commands: Commands,
	mut tosses: Query<(Entity, &mut Transform, &mut StashToss)>,
) {
	let dt = time.delta_secs();
	for (entity, mut transform, mut toss) in &mut tosses {
		toss.elapsed += dt;
		let t = (toss.elapsed / toss.duration.max(1e-4)).min(1.0);
		transform.translation = toss_sample(toss.start, toss.end, t);
		transform.rotate_y(dt * TOSS_SPIN);
		if t >= 1.0 {
			transform.translation = toss.end;
			commands.entity(entity).remove::<StashToss>();
		}
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

fn menu_drop_start(origin: Vec3) -> Vec3 {
	origin + Vec3::Y * TOSS_START_Y
}

fn menu_drop_landing(origin: Vec3, forward: Vec3, right: Vec3, n: u32) -> Vec3 {
	let angle = n as f32 * TOSS_GOLDEN;
	let spread = TOSS_SPREAD * (1.0 + n as f32 * 0.08).min(1.6);
	origin
		+ forward * TOSS_FORWARD
		+ right * TOSS_CAMERA_RIGHT
		+ (right * angle.cos() + forward * angle.sin()) * spread
		+ Vec3::Y * TOSS_LAND_Y
}

fn toss_sample(start: Vec3, end: Vec3, t: f32) -> Vec3 {
	let t = t.clamp(0.0, 1.0);
	start.lerp(end, t) + Vec3::Y * (4.0 * t * (1.0 - t) * TOSS_APEX)
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

	fn write_menu_drop(world: &mut World, item: InventoryItem, index: usize) -> anyhow::Result<()> {
		world
			.run_system_once(move |mut writer: MessageWriter<InventoryMenuDropped>| {
				writer.write(InventoryMenuDropped { item: item.clone(), index });
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
		use character_items::InventoryMenuDropped;
		use damage::DespawnAfter;

		let (mut world, _, _) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<InventoryMenuDropped>>();
		write_menu_drop(&mut world, queued_loadout().items[2].clone(), 2)?;
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
		use character_items::InventoryMenuDropped;

		let (mut world, _, _) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<InventoryMenuDropped>>();
		write_menu_drop(&mut world, queued_loadout().items[2].clone(), 2)?;
		spawn_menu_drops(&mut world)?;
		let toss = world
			.query::<&StashToss>()
			.iter(&world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("stash toss"))?;
		let origin = Vec3::new(3.0, 0.0, 1.0);
		assert!((toss.start - menu_drop_start(origin)).length() < 1e-4);
		// Identity facing is −Z, right is +X — into the open strip, not underfoot.
		assert!(toss.end.z < origin.z - 0.8);
		assert!(toss.end.x > origin.x + 0.4);
		assert!((toss.end.y - TOSS_LAND_Y).abs() < 1e-4);
		let pose = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| *transform)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!((pose.translation - toss.start).length() < 1e-4);
		Ok(())
	}

	#[test]
	fn menu_drop_landing_follows_player_yaw() {
		let origin = Vec3::new(3.0, 0.0, 1.0);
		let yawed = Transform::from_translation(origin)
			.with_rotation(Quat::from_rotation_y(std::f32::consts::FRAC_PI_2));
		let (forward, right) = menu_drop_axes(&yawed, None, None);
		let end = menu_drop_landing(origin, forward, right, 0);
		assert!(end.x < origin.x - 0.8, "yaw π/2 faces −X, got {end:?}");
		assert!(end.z < origin.z - 0.4, "camera-right fallback is −Z, got {end:?}");
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
		let end = menu_drop_landing(player.translation, forward, right, 0);
		let player_right_end = menu_drop_landing(player.translation, Vec3::NEG_Z, Vec3::X, 0);
		assert!(
			right.dot(Vec3::Z) > 0.7,
			"camera on −X looking at the player has +Z right, got {right:?}"
		);
		assert!(
			end.z > player_right_end.z + 0.4,
			"camera-right should pull the landing toward +Z, got {end:?} vs {player_right_end:?}"
		);
		assert!(
			end.x < player_right_end.x - 0.4,
			"should not use player-right +X as the strip, got {end:?} vs {player_right_end:?}"
		);
	}

	#[test]
	fn sequential_menu_drops_land_apart() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;
		use character_items::InventoryMenuDropped;

		let (mut world, _, _) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<InventoryMenuDropped>>();
		let item = queued_loadout().items[2].clone();
		world
			.run_system_once(move |mut writer: MessageWriter<InventoryMenuDropped>| {
				writer.write(InventoryMenuDropped { item: item.clone(), index: 2 });
				writer.write(InventoryMenuDropped { item: item.clone(), index: 2 });
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		spawn_menu_drops(&mut world)?;
		let ends: Vec<Vec3> =
			world.query::<&StashToss>().iter(&world).map(|toss| toss.end).collect();
		assert_eq!(ends.len(), 2);
		assert!(
			(ends[0] - ends[1]).length() > 0.25,
			"sequential drops should not stack, got {ends:?}"
		);
		Ok(())
	}

	#[test]
	fn stash_toss_arcs_then_settles() -> anyhow::Result<()> {
		use crate::stash::WorldStashSettings;
		use character_items::InventoryMenuDropped;
		use std::time::Duration;

		let (mut world, _, _) = drop_world(queued_loadout());
		world.init_resource::<WorldStashSettings>();
		world.init_resource::<Messages<InventoryMenuDropped>>();
		write_menu_drop(&mut world, queued_loadout().items[2].clone(), 2)?;
		spawn_menu_drops(&mut world)?;
		let toss = world
			.query::<&StashToss>()
			.iter(&world)
			.next()
			.copied()
			.ok_or_else(|| anyhow::anyhow!("stash toss"))?;
		let mid = toss_sample(toss.start, toss.end, 0.5);
		let chord = toss.start.lerp(toss.end, 0.5);
		assert!(
			mid.y > chord.y + 0.4,
			"apex should lift above the start–end chord, got {mid:?} vs {chord:?}"
		);

		world
			.resource_mut::<Time>()
			.advance_by(Duration::from_secs_f32(TOSS_SECS * 0.5));
		world
			.run_system_once(tick_stash_tosses)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let halfway = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| transform.translation)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!(halfway.y > chord.y + 0.2);

		world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(TOSS_SECS));
		world
			.run_system_once(tick_stash_tosses)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.query::<&StashToss>().iter(&world).count(), 0);
		let landed = world
			.query::<(&Transform, &WorldStash)>()
			.iter(&world)
			.next()
			.map(|(transform, _)| transform.translation)
			.ok_or_else(|| anyhow::anyhow!("stash pose"))?;
		assert!((landed - toss.end).length() < 1e-3);
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
