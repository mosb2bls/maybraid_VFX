//! Generic equip / holster / commit ownership.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;
use maybraid_character_controller::{CharacterIntent, InventoryCyclingCategory};
use maybraid_controllers_core::{InputRouter, InputSource};

use crate::lifecycle::{EquipmentInventoryChanged, PendingEquipmentHold, PendingEquipmentTeardown};
use crate::selection::InventoryCycling;

/// Total holster + raise window. Midpoint is when the kit should change.
pub const EQUIPMENT_HOLSTER_SECS: f32 = 0.42;

const SWAP_PITCH: f32 = 0.95;
const SWAP_DROP: f32 = 0.22;
const SWAP_INSET: f32 = 0.08;

/// In-progress weapon cycle. Lives on the user so it survives the kit replace.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct EquipmentHolster {
	pub elapsed: f32,
	pub duration: f32,
	pub swapped: bool,
	/// When true, the queue steps backward at the swap midpoint.
	pub reverse: bool,
}

impl Default for EquipmentHolster {
	fn default() -> Self {
		Self::start()
	}
}

impl EquipmentHolster {
	pub fn start() -> Self {
		Self::with_direction(false)
	}

	pub fn with_direction(reverse: bool) -> Self {
		Self { elapsed: 0.0, duration: EQUIPMENT_HOLSTER_SECS, swapped: false, reverse }
	}

	pub fn begin(entity: &mut EntityCommands, reverse: bool) {
		entity.insert(Self::with_direction(reverse));
	}

	pub fn t(self) -> f32 {
		if self.duration <= 0.0 {
			return 1.0;
		}
		(self.elapsed / self.duration).clamp(0.0, 1.0)
	}

	/// 0 ready, 1 lowest (midpoint), 0 ready again.
	pub fn dip(self) -> f32 {
		(self.t() * std::f32::consts::PI).sin()
	}

	pub fn ready_to_swap(self) -> bool {
		!self.swapped && self.elapsed >= self.duration * 0.5
	}

	pub fn finished(self) -> bool {
		self.swapped && self.elapsed >= self.duration
	}

	pub fn mark_swapped(&mut self) {
		self.swapped = true;
	}

	pub fn advance(&mut self, dt: f32) {
		self.elapsed += dt;
	}

	/// Dip the posed kit down and in toward the body.
	pub fn apply_to(self, transform: &mut Transform) {
		let dip = self.dip();
		if dip <= 0.0 {
			return;
		}
		transform.rotation *= Quat::from_rotation_x(dip * SWAP_PITCH);
		transform.translation += Vec3::Y * (-SWAP_DROP * dip);
		transform.translation += transform.rotation * Vec3::new(SWAP_INSET * dip, 0.0, -0.04 * dip);
	}

	/// Cycle the weapon queue at the holster midpoint and request kit replace.
	pub fn commit(
		&mut self,
		bag: &mut Inventory,
		entity: &mut EntityCommands,
	) -> Option<Inventory> {
		if !self.ready_to_swap() {
			return None;
		}
		let stepped = if self.reverse { bag.cycle_weapons(-1) } else { bag.swap_active() };
		if !stepped {
			self.mark_swapped();
			return None;
		}
		let snapshot = bag.clone();
		PendingEquipmentTeardown::request(entity);
		PendingEquipmentHold::request(entity, snapshot.clone());
		self.mark_swapped();
		Some(snapshot)
	}
}

/// Item crates insert this while a hold must not start a new holster.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlocksEquipmentSwap;

pub fn advance_equipment_holster(time: Res<Time>, mut holsters: Query<&mut EquipmentHolster>) {
	let dt = time.delta_secs();
	for mut holster in &mut holsters {
		holster.advance(dt);
	}
}

pub fn clear_finished_equipment_holsters(
	mut commands: Commands,
	holsters: Query<(Entity, &EquipmentHolster)>,
) {
	for (entity, holster) in &holsters {
		if holster.finished() {
			commands.entity(entity).remove::<EquipmentHolster>();
		}
	}
}

/// Start the holster motion; the kit changes at the dip.
pub fn begin_equipment_holster(
	mut intents: MessageReader<CharacterIntent>,
	router: Res<InputRouter>,
	cycling: Query<&InventoryCycling>,
	mut commands: Commands,
	actors: Query<(Has<EquipmentHolster>, Has<BlocksEquipmentSwap>, Option<&InventoryUser>)>,
	bags: Query<&Inventory>,
) {
	let Some(dir) = weapon_queue_step(&mut intents) else {
		return;
	};
	let Some(actor) = router.actor(InputSource::local_pad()) else {
		return;
	};
	let category = cycling
		.get(actor.entity())
		.ok()
		.map(InventoryCycling::category)
		.unwrap_or_default();
	if category != InventoryCyclingCategory::Weapons {
		return;
	}
	let Ok((holstering, blocked, user)) = actors.get(actor.entity()) else {
		return;
	};
	if holstering || blocked {
		return;
	}
	let Some(user) = user else {
		return;
	};
	let Ok(bag) = bags.get(user.bag) else {
		return;
	};
	if bag.weapons.len() < 2 {
		return;
	}
	EquipmentHolster::begin(&mut commands.entity(actor.entity()), dir < 0);
}

/// Replace the held kit once the old gun has dipped out of the way.
pub fn commit_equipment_holster(
	mut commands: Commands,
	mut changes: MessageWriter<EquipmentInventoryChanged>,
	mut actors: Query<(Entity, &InventoryUser, &mut EquipmentHolster)>,
	mut bags: Query<&mut Inventory>,
) {
	for (actor, user, mut holster) in &mut actors {
		let Ok(mut bag) = bags.get_mut(user.bag) else {
			continue;
		};
		let mut entity = commands.entity(actor);
		if let Some(snapshot) = holster.commit(&mut bag, &mut entity) {
			changes.write(EquipmentInventoryChanged::new(actor, snapshot));
		}
	}
}

fn weapon_queue_step(intents: &mut MessageReader<CharacterIntent>) -> Option<i8> {
	for intent in intents.read() {
		match *intent {
			CharacterIntent::SwapActive => return Some(1),
			CharacterIntent::CycleEquippedQueue(dir) if dir != 0 => return Some(dir),
			_ => {}
		}
	}
	None
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use character_inventory_user::InventoryUser;
	use character_items::{FirearmMesh, Inventory, InventoryItem};
	use maybraid_character_controller::{CharacterIntent, InventoryCyclingCategory};
	use maybraid_controllers_core::{ActorTarget, InputRouter, InputSource};

	use super::*;
	use crate::dispatch::EquippedItemHandler;
	use crate::lifecycle::EquipmentInventoryChanged;
	use crate::selection::{
		apply_inventory_category_selection, ActiveInventoryCyclingCategory, InventoryCycling,
	};

	fn two_gun_bag() -> Inventory {
		Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::firearm(FirearmMesh::Reltor),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		}
	}

	fn assign_actor(world: &mut World, actor: Entity) {
		world.init_resource::<InputRouter>();
		if world.get::<InventoryCycling>(actor).is_none() {
			world.entity_mut(actor).insert(InventoryCycling::default());
		}
		world
			.resource_mut::<InputRouter>()
			.assign_actor(InputSource::local_pad(), ActorTarget::new(actor));
	}

	#[test]
	fn dip_peaks_at_the_midpoint() {
		let start = EquipmentHolster::start();
		assert_eq!(start.dip(), 0.0);
		let mid = EquipmentHolster {
			elapsed: EQUIPMENT_HOLSTER_SECS * 0.5,
			duration: EQUIPMENT_HOLSTER_SECS,
			swapped: false,
			reverse: false,
		};
		assert!((mid.dip() - 1.0).abs() < 1e-5);
		assert!(mid.ready_to_swap());
		let done = EquipmentHolster {
			elapsed: EQUIPMENT_HOLSTER_SECS,
			duration: EQUIPMENT_HOLSTER_SECS,
			swapped: true,
			reverse: false,
		};
		assert!(done.dip().abs() < 1e-5);
		assert!(done.finished());
		assert!(!done.ready_to_swap());
		assert!(!EquipmentHolster {
			elapsed: EQUIPMENT_HOLSTER_SECS,
			duration: EQUIPMENT_HOLSTER_SECS,
			swapped: false,
			reverse: false,
		}
		.finished());
	}

	#[test]
	fn apply_to_lowers_the_bore() {
		let mut transform = Transform::IDENTITY;
		EquipmentHolster {
			elapsed: EQUIPMENT_HOLSTER_SECS * 0.5,
			duration: EQUIPMENT_HOLSTER_SECS,
			swapped: false,
			reverse: false,
		}
		.apply_to(&mut transform);
		assert!(transform.translation.y < 0.0);
		assert!((transform.rotation * Vec3::Z).y < 0.0);
	}

	#[test]
	fn same_frame_category_then_y_starts_weapon_swap() -> anyhow::Result<()> {
		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<ActiveInventoryCyclingCategory>();
		let bag = world.spawn(inventory).id();
		let actor = world.spawn(InventoryUser::carrying(bag)).id();
		assign_actor(&mut world, actor);
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
			.run_system_once(begin_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<EquipmentHolster>(actor).is_some());
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		Ok(())
	}

	#[test]
	fn y_starts_a_swap_without_changing_the_kit() -> anyhow::Result<()> {
		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let actor = world.spawn(InventoryUser::carrying(bag)).id();
		assign_actor(&mut world, actor);
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.get::<EquipmentHolster>(actor).is_some());
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
		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let bag = world.spawn(inventory).id();
		let actor = world.spawn(InventoryUser::carrying(bag)).id();
		assign_actor(&mut world, actor);
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::CycleEquippedQueue(-1));
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let holster =
			world.get::<EquipmentHolster>(actor).ok_or_else(|| anyhow::anyhow!("holster"))?;
		assert!(holster.reverse);
		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		Ok(())
	}

	#[test]
	fn reverse_queue_commits_through_weapon_swap() -> anyhow::Result<()> {
		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		let bag = world.spawn(inventory).id();
		world.spawn((
			InventoryUser::carrying(bag),
			EquipmentHolster {
				elapsed: EQUIPMENT_HOLSTER_SECS * 0.5,
				duration: EQUIPMENT_HOLSTER_SECS,
				swapped: false,
				reverse: true,
			},
		));
		world
			.run_system_once(commit_equipment_holster)
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
		let inventory = two_gun_bag();
		let mut world = World::new();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		let bag = world.spawn(inventory).id();
		let held = world.spawn_empty().id();
		let actor = world
			.spawn((
				InventoryUser::carrying(bag),
				EquipmentHolster {
					elapsed: EQUIPMENT_HOLSTER_SECS * 0.5,
					duration: EQUIPMENT_HOLSTER_SECS,
					swapped: false,
					reverse: false,
				},
			))
			.id();
		world
			.run_system_once(commit_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let bag = world.get::<Inventory>(bag).ok_or_else(|| anyhow::anyhow!("bag"))?;
		assert_eq!(
			bag.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert!(world.entities().contains(held));
		assert!(world.get::<PendingEquipmentTeardown>(actor).is_some());
		assert!(world.get::<PendingEquipmentHold>(actor).is_some());
		assert_eq!(
			world.get::<EquippedItemHandler>(actor).copied(),
			Some(EquippedItemHandler::Firearm)
		);
		let change = world
			.resource_mut::<Messages<EquipmentInventoryChanged>>()
			.drain()
			.next()
			.ok_or_else(|| anyhow::anyhow!("change"))?;
		assert_eq!(change.actor(), actor);
		assert_eq!(
			change.inventory().primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		Ok(())
	}

	#[test]
	fn throw_blocks_weapon_swap() -> anyhow::Result<()> {
		let inventory = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::standard_grenade(),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let bag = world.spawn(inventory).id();
		let actor = world.spawn((InventoryUser::carrying(bag), BlocksEquipmentSwap)).id();
		assign_actor(&mut world, actor);
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<EquipmentHolster>(actor).is_none());
		Ok(())
	}

	#[test]
	fn cooking_allows_weapon_swap() -> anyhow::Result<()> {
		let inventory = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::standard_grenade(),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		let bag = world.spawn(inventory).id();
		let actor = world.spawn(InventoryUser::carrying(bag)).id();
		assign_actor(&mut world, actor);
		world
			.run_system_once(|mut writer: MessageWriter<CharacterIntent>| {
				writer.write(CharacterIntent::SwapActive);
			})
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(begin_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<EquipmentHolster>(actor).is_some());
		Ok(())
	}

	#[test]
	fn holster_midpoint_commits_the_handler() -> anyhow::Result<()> {
		let inventory = Inventory {
			items: vec![
				InventoryItem::firearm(FirearmMesh::Bullpup),
				InventoryItem::standard_grenade(),
			],
			clothing: Vec::new(),
			weapons: vec![0, 1],
			skills: Vec::new(),
			tanzes: Vec::new(),
		};
		let mut world = World::new();
		world.init_resource::<Messages<EquipmentInventoryChanged>>();
		let bag = world.spawn(inventory).id();
		let actor = world
			.spawn((
				InventoryUser::carrying(bag),
				EquippedItemHandler::Firearm,
				EquipmentHolster {
					elapsed: EQUIPMENT_HOLSTER_SECS * 0.5,
					duration: EQUIPMENT_HOLSTER_SECS,
					swapped: false,
					reverse: false,
				},
			))
			.id();
		world
			.run_system_once(commit_equipment_holster)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<EquippedItemHandler>(actor).copied(),
			Some(EquippedItemHandler::Grenade)
		);
		Ok(())
	}
}
