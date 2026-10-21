//! Cook / windup clocks, routing cancel, recharge, and equipment realize.
//!
//! HUD / audio / held-mesh stay in [`crate::presentation`].

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use character_items::{Inventory, InventoryItem};
use maybraid_equipment::{
	BlocksEquipmentSwap, EquipmentSystems, PendingEquipmentHold, PendingEquipmentTeardown,
};
use player::{PlayerSystems, PlayerUse};

use crate::hold::spawn_held_grenade;
use crate::throw::{
	GrenadeThrow, GrenadeUser, GrenadeUserSettings, GrenadeUserSystems, advance_throw,
	apply_throw_intents, apply_throw_routing, interrupt_live_cooks,
};

#[derive(Resource, Default)]
struct GrenadeUserRuntime;

/// Cook / windup clocks, routing cancel, and owned recharge. Gameplay
/// assembly is the owner that calls this; a second call is a no-op. HUD /
/// audio / mesh realization stay on [`crate::register_grenade_user_presentation`].
pub fn register_grenade_user_runtime(app: &mut App) {
	if app.world().contains_resource::<GrenadeUserRuntime>() {
		return;
	}
	app.init_resource::<GrenadeUserRuntime>()
		.add_message::<maybraid_equipment::EquippedItemUse>()
		.add_message::<grenades::DropLiveGrenade>()
		.add_message::<grenades::SpawnThrownGrenade>()
		.add_message::<grenades::GrenadeDetonated>()
		.add_systems(Update, apply_throw_intents.in_set(PlayerSystems::Intent))
		.add_systems(
			Update,
			apply_throw_routing.after(apply_throw_intents).in_set(PlayerSystems::Intent),
		)
		.add_systems(Update, advance_throw.after(PlayerSystems::Intent))
		.add_systems(Update, sync_equipment_swap_block.before(EquipmentSystems::BeginHolster))
		.add_systems(
			Update,
			(realize_equipment_teardown, realize_equipment_hold.after(realize_equipment_teardown))
				.in_set(EquipmentSystems::Realize),
		)
		.add_systems(Update, interrupt_live_cooks.in_set(GrenadeUserSystems::Interrupt))
		.add_systems(
			PostUpdate,
			interrupt_live_cooks
				.in_set(GrenadeUserSystems::Interrupt)
				.after(damage::DamageSystems::Down),
		)
		.add_systems(Update, tick_owned_recharge);
}

pub fn teardown_held_grenade(commands: &mut Commands, user: Entity, grenade: &GrenadeUser) {
	commands.entity(grenade.held).try_despawn();
	commands
		.entity(user)
		.remove::<(GrenadeUser, GrenadeThrow, PlayerUse, BlocksEquipmentSwap)>();
}

fn sync_equipment_swap_block(
	mut commands: Commands,
	throws: Query<(Entity, &GrenadeThrow, Has<BlocksEquipmentSwap>)>,
) {
	for (entity, throw, blocked) in &throws {
		match (throw.blocks_swap(), blocked) {
			(true, false) => {
				commands.entity(entity).insert(BlocksEquipmentSwap);
			}
			(false, true) => {
				commands.entity(entity).remove::<BlocksEquipmentSwap>();
			}
			_ => {}
		}
	}
}

fn realize_equipment_teardown(
	mut commands: Commands,
	pending: Query<(Entity, &GrenadeUser), With<PendingEquipmentTeardown>>,
) {
	for (entity, grenade) in &pending {
		teardown_held_grenade(&mut commands, entity, grenade);
	}
}

fn realize_equipment_hold(mut commands: Commands, pending: Query<(Entity, &PendingEquipmentHold)>) {
	for (entity, hold) in &pending {
		if !matches!(hold.inventory().primary_weapon(), Some(InventoryItem::Grenade { .. })) {
			continue;
		}
		spawn_held_grenade(&mut commands, entity, GrenadeUserSettings::default());
	}
}

fn tick_owned_recharge(
	time: Res<Time>,
	mut bags: ParamSet<(Query<(Entity, &Inventory)>, Query<&mut Inventory>)>,
	mut due: Local<Vec<Entity>>,
) {
	let dt = time.delta_secs();
	if dt <= 0.0 {
		return;
	}
	due.clear();
	for (entity, bag) in bags.p0().iter() {
		if bag.items.iter().any(
			|item| matches!(item, InventoryItem::Grenade { recharge, .. } if recharge.remaining > 0.0),
		) {
			due.push(entity);
		}
	}
	let mut mutable = bags.p1();
	for entity in due.iter().copied() {
		let Ok(mut bag) = mutable.get_mut(entity) else {
			continue;
		};
		for item in &mut bag.items {
			if let InventoryItem::Grenade { recharge, .. } = item {
				if recharge.remaining > 0.0 {
					recharge.tick(dt);
				}
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;

	use super::*;
	use character_items::{GrenadeRecharge, GrenadeSpec, GrenadeStats};
	use std::time::Duration;

	#[test]
	fn owned_recharge_ticks_while_unequipped() -> anyhow::Result<()> {
		let mut world = World::new();
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_millis(500));
		world.insert_resource(time);
		world.spawn(Inventory {
			items: vec![InventoryItem::Grenade {
				spec: GrenadeSpec::standard(),
				stats: GrenadeStats::standard(),
				recharge: GrenadeRecharge { remaining: 1.0 },
			}],
			clothing: Vec::new(),
			weapons: vec![0],
			skills: Vec::new(),
			tanzes: Vec::new(),
		});
		world
			.run_system_once(tick_owned_recharge)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let bag = world
			.query::<&Inventory>()
			.single(&world)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let InventoryItem::Grenade { recharge, .. } = &bag.items[0] else {
			anyhow::bail!("expected grenade inventory item");
		};
		anyhow::ensure!(recharge.remaining < 1.0, "active recharge must count down");
		Ok(())
	}

	#[test]
	fn idle_recharge_does_not_dirty_inventory() -> anyhow::Result<()> {
		let mut world = World::new();
		let mut time = Time::<()>::default();
		time.advance_by(Duration::from_millis(16));
		world.insert_resource(time);
		let entity = world
			.spawn(Inventory {
				items: vec![InventoryItem::Grenade {
					spec: GrenadeSpec::standard(),
					stats: GrenadeStats::standard(),
					recharge: GrenadeRecharge { remaining: 0.0 },
				}],
				clothing: Vec::new(),
				weapons: vec![0],
				skills: Vec::new(),
				tanzes: Vec::new(),
			})
			.id();
		world.clear_trackers();
		world
			.run_system_once(tick_owned_recharge)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let bag = world
			.query::<Ref<Inventory>>()
			.get(&world, entity)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(!bag.is_changed(), "idle grenade recharge must not dirty the inventory");
		let InventoryItem::Grenade { recharge, .. } = &bag.items[0] else {
			anyhow::bail!("expected grenade inventory item");
		};
		anyhow::ensure!(recharge.remaining == 0.0, "idle recharge must stay at zero");
		Ok(())
	}
}
