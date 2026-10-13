//! Open a furniture chest: swing the lid, eject one stash, restock later.
//!
//! The deadline lives on [`CrateLoot`], keyed by the presented cell id plus the
//! slot, because the furniture presenter despawns the host outside the present ring.

use std::collections::HashMap;

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use character_items::{Inventory, ItemRng};
use furniture_assemblies::{FurnitureKitPart, PartKind};
use lod::gen::Id;
use lod::hcsg::HcsgNode;

use crate::WorldFurnished;

use crate::stash::{spawn_world_stash, StashPolicy, DEFAULT_CLAIM_RADIUS, DEFAULT_LOOT_SECS};

const LID_SWING_SECS: f32 = 0.45;
const LID_OPEN: f32 = std::f32::consts::FRAC_PI_2;
const EJECT_AHEAD: f32 = 1.2;
const RESTOCK_MIN_SECS: f32 = 10.0 * 60.0;
const RESTOCK_SPAN_SECS: f32 = 5.0 * 60.0;

/// Closed pose captured the frame the lid entity appears.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct ClosedLid(pub Transform);

/// In-progress hinge. Removed when the lid is open and the stash has been ejected.
#[derive(Component, Clone, Copy, Debug)]
pub(crate) struct LidSwing {
	key: CrateKey,
	duration: f32,
	elapsed: f32,
	eject_origin: Vec3,
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) struct CrateKey {
	pub cell: Id,
	pub finish_seed: u64,
	pub slot: u32,
}

#[derive(Clone, Debug)]
enum CratePhase {
	Opening { restock_at: f32, bag: Inventory },
	Open { restock_at: f32 },
}

/// Restock deadlines that outlive the furniture cell entity.
#[derive(Resource, Default, Debug)]
pub(crate) struct CrateLoot {
	crates: HashMap<CrateKey, CratePhase>,
}

/// Closed lids, packed so the claim systems stay under Bevy's system-param limit.
#[derive(SystemParam)]
pub(crate) struct ClosedCrateQuery<'w, 's> {
	crates: Option<Res<'w, CrateLoot>>,
	lids: Query<
		'w,
		's,
		(Entity, &'static FurnitureKitPart, &'static GlobalTransform),
		(With<ClosedLid>, Without<LidSwing>),
	>,
	parts: Query<'w, 's, (Entity, &'static FurnitureKitPart, &'static GlobalTransform)>,
	child_of: Query<'w, 's, &'static ChildOf>,
	hosts: Query<'w, 's, &'static HcsgNode<WorldFurnished>>,
}

impl ClosedCrateQuery<'_, '_> {
	pub(crate) fn floor(&self, origin: Vec3) -> Option<Vec3> {
		let crates = self.crates.as_deref()?;
		nearest_closed_crate_floor(
			origin,
			crates,
			&self.lids,
			&self.parts,
			&self.child_of,
			&self.hosts,
		)
	}
}

impl ClosedLid {
	/// Pitch about the rear rim. The lid GLB is floor-origin (`Y ∈ [0, 1]`) with
	/// plan `X,Z ∈ [-1, 1]`; kit `+Z` is the back, so the hinge is the floor
	/// edge at local `(0, 0, 1)`.
	fn swung(self, amount: f32) -> Transform {
		let closed = self.0;
		let rear = Vec3::new(0.0, 0.0, closed.scale.z);
		let hinge = closed.translation + closed.rotation * rear;
		let swing = Quat::from_rotation_x(amount.clamp(0.0, 1.0) * LID_OPEN);
		let local = closed.translation - hinge;
		Transform {
			translation: hinge + closed.rotation * swing * local,
			rotation: closed.rotation * swing,
			scale: closed.scale,
		}
	}
}

impl CrateLoot {
	fn phase(&self, key: CrateKey) -> Option<&CratePhase> {
		self.crates.get(&key)
	}

	fn is_closed(&self, key: CrateKey) -> bool {
		self.crates.get(&key).is_none()
	}

	/// Begin an open. A second call while opening or waiting to restock does nothing.
	fn start_open(&mut self, key: CrateKey, now: f32) -> bool {
		if !self.is_closed(key) {
			return false;
		}
		self.crates.insert(
			key,
			CratePhase::Opening {
				restock_at: now + restock_secs(key.finish_seed),
				bag: roll_crate_bag(key.finish_seed),
			},
		);
		true
	}

	#[cfg(test)]
	fn restock_at(&self, key: CrateKey) -> Option<f32> {
		match self.crates.get(&key) {
			Some(CratePhase::Opening { restock_at, .. } | CratePhase::Open { restock_at }) => {
				Some(*restock_at)
			}
			None => None,
		}
	}

	/// Move `Opening` to `Open` and return the bag to eject. Later calls return nothing.
	fn finish_open(&mut self, key: CrateKey) -> Option<Inventory> {
		let Some(CratePhase::Opening { restock_at, bag }) = self.crates.remove(&key) else {
			return None;
		};
		self.crates.insert(key, CratePhase::Open { restock_at });
		Some(bag)
	}

	fn close_if_due(&mut self, key: CrateKey, now: f32) -> bool {
		match self.crates.get(&key) {
			Some(CratePhase::Open { restock_at }) if now >= *restock_at => {
				self.crates.remove(&key);
				true
			}
			_ => false,
		}
	}
}

/// Stand-in until [#822](https://github.com/ramate-io/maybraid/issues/822) owns the loot table.
/// `finish_seed % 8 == 0` is an empty crate; every other seed rolls one firearm.
fn roll_crate_bag(finish_seed: u64) -> Inventory {
	if finish_seed % 8 == 0 {
		return Inventory::default();
	}
	let mut rng = ItemRng::from_seed(finish_seed);
	let mut bag = Inventory {
		items: rng.random_starter_firearms(1),
		clothing: Vec::new(),
		weapons: Vec::new(),
		skills: Vec::new(),
	};
	// Queue the firearm so the ground pile renders. Claim absorb ignores the queue.
	let _ = bag.toggle(0);
	bag
}

fn restock_secs(finish_seed: u64) -> f32 {
	RESTOCK_MIN_SECS + (finish_seed % 10_000) as f32 / 10_000.0 * RESTOCK_SPAN_SECS
}

pub(crate) fn stamp_closed_lids(
	mut commands: Commands,
	lids: Query<(Entity, &FurnitureKitPart, &Transform), Without<ClosedLid>>,
) {
	for (entity, part, transform) in &lids {
		if part.kind != PartKind::ChestLid {
			continue;
		}
		commands.entity(entity).insert(ClosedLid(*transform));
	}
}

pub(crate) fn open_nearest_crate(
	origin: Vec3,
	now: f32,
	commands: &mut Commands,
	crates: &mut CrateLoot,
	lids: &Query<
		(Entity, &FurnitureKitPart, &GlobalTransform),
		(With<ClosedLid>, Without<LidSwing>),
	>,
	parts: &Query<(Entity, &FurnitureKitPart, &GlobalTransform)>,
	child_of: &Query<&ChildOf>,
	hosts: &Query<&HcsgNode<WorldFurnished>>,
) {
	let Some(hit) = nearest_closed_crate(origin, crates, lids, parts, child_of, hosts) else {
		return;
	};
	let lid = hit.lid;
	let key = hit.key;
	let eject_origin = hit.eject_origin;
	if !crates.start_open(key, now) {
		return;
	}
	commands.entity(lid).insert(LidSwing {
		key,
		duration: LID_SWING_SECS,
		elapsed: 0.0,
		eject_origin,
	});
}

pub(crate) fn tick_lid_swings(
	time: Res<Time>,
	mut commands: Commands,
	assets: Option<Res<AssetServer>>,
	mut crates: ResMut<CrateLoot>,
	mut lids: Query<(Entity, &mut LidSwing, &ClosedLid, &mut Transform)>,
) {
	let dt = time.delta_secs();
	for (entity, mut swing, closed, mut transform) in &mut lids {
		let amount = advance_swing(&mut swing, dt);
		*transform = closed.swung(amount);
		if amount < 1.0 {
			continue;
		}
		let key = swing.key;
		let eject_origin = swing.eject_origin;
		commands.entity(entity).remove::<LidSwing>();
		if let Some(bag) = crates.finish_open(key) {
			eject_bag(&mut commands, assets.as_deref(), eject_origin, bag);
		}
	}
}

pub(crate) fn sync_crate_lid_poses(
	time: Res<Time>,
	mut commands: Commands,
	assets: Option<Res<AssetServer>>,
	mut crates: ResMut<CrateLoot>,
	child_of: Query<&ChildOf>,
	hosts: Query<&HcsgNode<WorldFurnished>>,
	parts: Query<(Entity, &FurnitureKitPart, &GlobalTransform)>,
	mut lids: Query<(
		Entity,
		&FurnitureKitPart,
		&ClosedLid,
		&GlobalTransform,
		&mut Transform,
		Option<&LidSwing>,
	)>,
) {
	let now = time.elapsed_secs();
	for (entity, part, closed, global, mut transform, swing) in &mut lids {
		if part.kind != PartKind::ChestLid || swing.is_some() {
			continue;
		}
		let Some(cell) = presented_cell_id(entity, &child_of, &hosts) else {
			continue;
		};
		let key = CrateKey { cell, finish_seed: part.finish_seed, slot: part.slot };
		if matches!(crates.phase(key), Some(CratePhase::Opening { .. })) {
			let floor_y = trunk_floor_y(key, &parts, &child_of, &hosts)
				.unwrap_or_else(|| global.translation().y);
			let eject_origin = eject_point(global, floor_y);
			if let Some(bag) = crates.finish_open(key) {
				eject_bag(&mut commands, assets.as_deref(), eject_origin, bag);
			}
		}
		if crates.close_if_due(key, now) {
			apply_pose(&mut transform, closed.0);
			continue;
		}
		match crates.phase(key) {
			Some(CratePhase::Open { .. }) => apply_pose(&mut transform, closed.swung(1.0)),
			_ => apply_pose(&mut transform, closed.0),
		}
	}
}

fn advance_swing(swing: &mut LidSwing, dt: f32) -> f32 {
	if swing.duration <= f32::EPSILON {
		return 1.0;
	}
	swing.elapsed = (swing.elapsed + dt.max(0.0)).min(swing.duration);
	swing.elapsed / swing.duration
}

struct ClosedCrateHit {
	lid: Entity,
	key: CrateKey,
	floor: Vec3,
	eject_origin: Vec3,
}

/// Ground point of the nearest closed stocked crate, for the claim rings.
pub(crate) fn nearest_closed_crate_floor(
	origin: Vec3,
	crates: &CrateLoot,
	lids: &Query<
		(Entity, &FurnitureKitPart, &GlobalTransform),
		(With<ClosedLid>, Without<LidSwing>),
	>,
	parts: &Query<(Entity, &FurnitureKitPart, &GlobalTransform)>,
	child_of: &Query<&ChildOf>,
	hosts: &Query<&HcsgNode<WorldFurnished>>,
) -> Option<Vec3> {
	nearest_closed_crate(origin, crates, lids, parts, child_of, hosts).map(|hit| hit.floor)
}

fn nearest_closed_crate(
	origin: Vec3,
	crates: &CrateLoot,
	lids: &Query<
		(Entity, &FurnitureKitPart, &GlobalTransform),
		(With<ClosedLid>, Without<LidSwing>),
	>,
	parts: &Query<(Entity, &FurnitureKitPart, &GlobalTransform)>,
	child_of: &Query<&ChildOf>,
	hosts: &Query<&HcsgNode<WorldFurnished>>,
) -> Option<ClosedCrateHit> {
	let mut best: Option<(ClosedCrateHit, f32)> = None;
	for (entity, part, global) in lids {
		if part.kind != PartKind::ChestLid {
			continue;
		}
		let Some(cell) = presented_cell_id(entity, child_of, hosts) else {
			continue;
		};
		let key = CrateKey { cell, finish_seed: part.finish_seed, slot: part.slot };
		if !crates.is_closed(key) {
			continue;
		}
		let distance = origin.xz().distance(global.translation().xz());
		if distance > DEFAULT_CLAIM_RADIUS {
			continue;
		}
		if best.as_ref().is_some_and(|(_, nearest)| distance >= *nearest) {
			continue;
		}
		let floor_y =
			trunk_floor_y(key, parts, child_of, hosts).unwrap_or_else(|| global.translation().y);
		let floor = Vec3::new(global.translation().x, floor_y, global.translation().z);
		best = Some((
			ClosedCrateHit { lid: entity, key, floor, eject_origin: eject_point(global, floor_y) },
			distance,
		));
	}
	best.map(|(hit, _)| hit)
}

fn trunk_floor_y(
	key: CrateKey,
	parts: &Query<(Entity, &FurnitureKitPart, &GlobalTransform)>,
	child_of: &Query<&ChildOf>,
	hosts: &Query<&HcsgNode<WorldFurnished>>,
) -> Option<f32> {
	parts.iter().find_map(|(entity, part, global)| {
		if part.kind != PartKind::ChestTrunk
			|| part.slot != key.slot
			|| part.finish_seed != key.finish_seed
		{
			return None;
		}
		let cell = presented_cell_id(entity, child_of, hosts)?;
		(cell == key.cell).then_some(global.translation().y)
	})
}

fn eject_point(lid: &GlobalTransform, floor_y: f32) -> Vec3 {
	let ahead = lid.translation() + flat_facing(lid.forward().as_vec3()) * EJECT_AHEAD;
	Vec3::new(ahead.x, floor_y, ahead.z)
}

fn flat_facing(forward: Vec3) -> Vec3 {
	let flat = Vec3::new(forward.x, 0.0, forward.z);
	if flat.length_squared() > 1e-6 {
		flat.normalize()
	} else {
		Vec3::NEG_Z
	}
}

fn eject_bag(commands: &mut Commands, assets: Option<&AssetServer>, origin: Vec3, bag: Inventory) {
	spawn_world_stash(
		commands,
		Transform::from_translation(origin),
		bag,
		StashPolicy::ephemeral(DEFAULT_LOOT_SECS),
		assets,
	);
}

fn apply_pose(transform: &mut Transform, want: Transform) {
	let moved = transform.translation.distance_squared(want.translation) > 1e-8;
	let turned = transform.rotation.dot(want.rotation).abs() < 0.9999;
	if moved || turned {
		*transform = want;
	}
}

fn presented_cell_id(
	start: Entity,
	child_of: &Query<&ChildOf>,
	hosts: &Query<&HcsgNode<WorldFurnished>>,
) -> Option<Id> {
	let mut current = Some(start);
	for _ in 0..24 {
		let entity = current?;
		if let Ok(node) = hosts.get(entity) {
			return Some(node.id);
		}
		current = child_of.get(entity).ok().map(|child| child.parent());
	}
	None
}

#[cfg(test)]
pub(crate) fn spawn_furniture_host(world: &mut World, id: Id) -> Entity {
	use std::sync::Arc;

	use lod::gen::Version;
	use maputo::FurnitureCellExtent;

	let extent = FurnitureCellExtent::from_id(id)
		.unwrap_or_else(|| FurnitureCellExtent::from_cell_index(0, 0));
	let (value, bounds) = WorldFurnished::empty_host(extent);
	world
		.spawn(HcsgNode { id, version: Version(1), bounds, value: Arc::new(value) })
		.id()
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::stash::WorldStash;
	use bevy::ecs::system::RunSystemOnce;
	use character_inventory_user::InventoryUser;
	use maybraid_character_controller::CharacterIntent;
	use std::time::Duration;
	use world_player::Player as VegetationPlayer;

	use crate::stash::claim_nearby_stashes;

	fn lid_bundle(seed: u64, at: Vec3) -> impl Bundle {
		let transform = Transform::from_translation(at);
		(
			FurnitureKitPart { kind: PartKind::ChestLid, finish_seed: seed, slot: 0 },
			ClosedLid(transform),
			transform,
			GlobalTransform::from(transform),
		)
	}

	fn spawn_crate(world: &mut World, seed: u64, at: Vec3) -> Entity {
		let host = spawn_furniture_host(world, Id::Universal);
		world.spawn((lid_bundle(seed, at), ChildOf(host))).id()
	}

	fn player(world: &mut World, at: Vec3) -> Entity {
		let bag = world.spawn(Inventory::default()).id();
		world
			.spawn((
				VegetationPlayer,
				Transform::from_translation(at),
				InventoryUser::carrying(bag),
			))
			.id()
	}

	fn press_x(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::StartInteraction);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(claim_nearby_stashes)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	fn ready(world: &mut World) {
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<CrateLoot>();
		world.init_resource::<Time>();
	}

	#[test]
	fn restock_window_is_ten_to_fifteen_minutes() {
		assert_eq!(restock_secs(0), RESTOCK_MIN_SECS);
		assert!(restock_secs(9_999) < RESTOCK_MIN_SECS + RESTOCK_SPAN_SECS);
		assert!(restock_secs(10_000) >= RESTOCK_MIN_SECS);
		assert!(restock_secs(3) - RESTOCK_MIN_SECS <= RESTOCK_SPAN_SECS);
	}

	#[test]
	fn open_lid_hinges_on_the_rear_rim() {
		let closed = ClosedLid(Transform {
			translation: Vec3::new(0.0, 0.28, 0.0),
			rotation: Quat::IDENTITY,
			scale: Vec3::new(0.51, 0.22, 0.51),
		});
		let shut = closed.swung(0.0);
		assert!(shut.translation.distance(closed.0.translation) < 1e-4);
		assert!(shut.rotation.angle_between(Quat::IDENTITY) < 1e-3);
		let open = closed.swung(1.0);
		assert!(open.rotation.angle_between(Quat::IDENTITY) > 1.0);
		let rear = Vec3::new(0.0, 0.0, 1.0);
		assert!(open.transform_point(rear).distance(closed.0.transform_point(rear)) < 1e-3);
		let front = Vec3::new(0.0, 0.0, -1.0);
		let front_open = open.transform_point(front);
		assert!(front_open.y > closed.0.transform_point(front).y + closed.0.scale.z);
		assert!(front_open.y > closed.0.translation.y);
	}

	#[test]
	fn rolled_firearm_is_queued_for_display() {
		let bag = roll_crate_bag(1);
		assert_eq!(bag.items.len(), 1);
		assert_eq!(bag.weapons, vec![0]);
		let empty = roll_crate_bag(0);
		assert!(empty.items.is_empty());
		assert!(empty.weapons.is_empty());
	}

	#[test]
	fn start_open_keeps_the_first_deadline() {
		let mut crates = CrateLoot::default();
		let key = CrateKey { cell: Id::Universal, finish_seed: 3, slot: 0 };
		assert!(crates.start_open(key, 10.0));
		let first = crates.restock_at(key);
		assert!(!crates.start_open(key, 99.0));
		assert_eq!(crates.restock_at(key), first);
		assert!((first.unwrap() - (10.0 + restock_secs(3))).abs() < 1e-3);
	}

	#[test]
	fn x_swings_the_lid_and_ejects_one_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		ready(&mut world);
		player(&mut world, Vec3::new(0.0, 0.0, 3.0));
		let lid = spawn_crate(&mut world, 1, Vec3::ZERO);
		press_x(&mut world)?;
		assert!(world.get::<LidSwing>(lid).is_some());
		world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(LID_SWING_SECS));
		world
			.run_system_once(tick_lid_swings)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<LidSwing>(lid).is_none());
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		assert_eq!(world.query::<&crate::stash::StashDisplayedItem>().iter(&world).count(), 1);
		let open = world.get::<Transform>(lid).ok_or_else(|| anyhow::anyhow!("lid"))?;
		assert!(open.rotation.angle_between(Quat::IDENTITY) > 1.0);

		press_x(&mut world)?;
		world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(LID_SWING_SECS));
		world
			.run_system_once(tick_lid_swings)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 1);
		assert!(world.get::<LidSwing>(lid).is_none());
		Ok(())
	}

	#[test]
	fn empty_roll_opens_without_a_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		ready(&mut world);
		player(&mut world, Vec3::ZERO);
		spawn_crate(&mut world, 0, Vec3::ZERO);
		press_x(&mut world)?;
		world.resource_mut::<Time>().advance_by(Duration::from_secs_f32(LID_SWING_SECS));
		world
			.run_system_once(tick_lid_swings)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		let key = CrateKey { cell: Id::Universal, finish_seed: 0, slot: 0 };
		assert!(world.resource::<CrateLoot>().restock_at(key).is_some());
		Ok(())
	}

	#[test]
	fn a_nearer_stash_wins_the_press() -> anyhow::Result<()> {
		use crate::stash::{spawn_world_stash, StashPolicy};
		use character_items::{FirearmMesh, InventoryItem};

		let mut world = World::new();
		ready(&mut world);
		player(&mut world, Vec3::ZERO);
		spawn_crate(&mut world, 1, Vec3::new(2.0, 0.0, 0.0));
		world
			.run_system_once(|mut commands: Commands| {
				spawn_world_stash(
					&mut commands,
					Transform::from_xyz(0.5, 0.0, 0.0),
					Inventory {
						items: vec![InventoryItem::firearm(FirearmMesh::Bullpup)],
						..default()
					},
					StashPolicy::default(),
					None,
				);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		press_x(&mut world)?;
		assert_eq!(world.query::<&LidSwing>().iter(&world).count(), 0);
		assert!(world.resource::<CrateLoot>().crates.is_empty());
		Ok(())
	}

	#[test]
	fn returning_before_the_deadline_keeps_the_lid_open() -> anyhow::Result<()> {
		let mut world = World::new();
		ready(&mut world);
		let lid = spawn_crate(&mut world, 1, Vec3::ZERO);
		let key = CrateKey { cell: Id::Universal, finish_seed: 1, slot: 0 };
		world
			.resource_mut::<CrateLoot>()
			.crates
			.insert(key, CratePhase::Open { restock_at: 5_000.0 });
		world
			.run_system_once(sync_crate_lid_poses)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let pose = world.get::<Transform>(lid).ok_or_else(|| anyhow::anyhow!("lid"))?;
		assert!(pose.rotation.angle_between(Quat::IDENTITY) > 1.0);
		assert_eq!(world.resource::<CrateLoot>().restock_at(key), Some(5_000.0));
		Ok(())
	}

	#[test]
	fn a_passed_deadline_closes_the_lid_without_a_new_stash() -> anyhow::Result<()> {
		let mut world = World::new();
		ready(&mut world);
		let lid = spawn_crate(&mut world, 1, Vec3::ZERO);
		world.entity_mut(lid).insert(Transform {
			rotation: Quat::from_rotation_x(-LID_OPEN),
			..Transform::from_translation(Vec3::ZERO)
		});
		let key = CrateKey { cell: Id::Universal, finish_seed: 1, slot: 0 };
		world
			.resource_mut::<CrateLoot>()
			.crates
			.insert(key, CratePhase::Open { restock_at: 0.0 });
		world.resource_mut::<Time>().advance_by(Duration::from_secs(1));
		world
			.run_system_once(sync_crate_lid_poses)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<CrateLoot>().restock_at(key).is_none());
		assert_eq!(world.query::<&WorldStash>().iter(&world).count(), 0);
		let pose = world.get::<Transform>(lid).ok_or_else(|| anyhow::anyhow!("lid"))?;
		assert!(pose.rotation.angle_between(Quat::IDENTITY) < 1e-3);
		Ok(())
	}
}
