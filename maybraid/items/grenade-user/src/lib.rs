//! Held grenade, overhand throw, and owned recharge clocks.

mod cook_hud;
mod feel;
mod hold;
mod throw;

use bevy::ecs::system::ParamSet;
use bevy::prelude::*;
use character_items::{Inventory, InventoryItem};
use characters::{CharacterMotionSystems, CharacterPoseOverlay, CharacterPoseSystems};
use maybraid_spatial_rumble::{SpatialRumblePlugin, SpatialRumbleSystems};
use player::{PlayerPoseSystems, PlayerSystems, PlayerUse};

pub use feel::{blast_presence, blast_rumble, rumble_presence, GrenadeBlastFeel};
pub use hold::{spawn_held_grenade, HeldGrenade, HoldingGrenade};
pub use throw::{
	apply_throw_intents, apply_throw_routing, landing_range, landing_time, launch_velocity,
	throw_aim, GrenadePhase, GrenadeThrow, GrenadeUser, GrenadeUserSettings, GrenadeUserSystems,
	STANDING_RELEASE_HEIGHT, THROW_GRAVITY,
};

/// Held-side 1:1 target of [`GrenadeUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = GrenadeUser)]
pub struct HeldByGrenade(Entity);

pub struct GrenadeUserPlugin;

/// Cook / windup clocks, routing cancel, and owned recharge. HUD / audio /
/// mesh realization stay on [`GrenadeUserPlugin`].
pub fn register_grenade_user_runtime(app: &mut App) {
	app.add_message::<maybraid_character_controller::EquippedItemUse>()
		.add_message::<grenades::DropLiveGrenade>()
		.add_message::<grenades::GrenadeDetonated>()
		.add_systems(Update, throw::apply_throw_intents.in_set(PlayerSystems::Intent))
		.add_systems(
			Update,
			throw::apply_throw_routing.after(throw::apply_throw_intents).in_set(PlayerSystems::Intent),
		)
		.add_systems(Update, throw::advance_throw.after(PlayerSystems::Intent))
		.add_systems(Update, throw::interrupt_live_cooks.in_set(GrenadeUserSystems::Interrupt))
		.add_systems(
			PostUpdate,
			throw::interrupt_live_cooks
				.in_set(GrenadeUserSystems::Interrupt)
				.after(damage::DamageSystems::Down),
		)
		.add_systems(Update, tick_owned_recharge);
}

impl Plugin for GrenadeUserPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<grenades::GrenadesPlugin>() {
			app.add_plugins(grenades::GrenadesPlugin);
		}
		if !app.is_plugin_added::<SpatialRumblePlugin>() {
			app.add_plugins(SpatialRumblePlugin);
		}
		register_grenade_user_runtime(app);
		app.add_systems(Startup, cook_hud::spawn_cook_fuse_hud)
			.add_systems(
				Update,
				(
					hold::stamp_holding_grenade,
					hold::pose_held_grenade,
					hold::apply_grenade_swap_pose,
					hold::sync_held_visibility,
				)
					.chain()
					.in_set(PlayerPoseSystems::Item),
			)
			.add_systems(
				Update,
				hold::sync_throw_arm
					.in_set(CharacterPoseOverlay::Grenade)
					.after(CharacterMotionSystems::Anim),
			)
			.add_systems(
				Update,
				throw::play_throw_jingles
					.after(CharacterPoseSystems::Overlay)
					.before(throw::advance_throw),
			)
			.add_systems(Update, cook_hud::sync_cook_fuse_hud.after(throw::advance_throw))
			.add_systems(
				Update,
				feel::pulse_blast_feel
					.after(grenades::apply_grenade_blasts)
					.before(SpatialRumbleSystems::Ingest),
			)
			.add_systems(
				Update,
				(hold::realize_held_grenade_visuals, despawn_orphaned_held_grenades).chain(),
			);
	}
}

pub fn teardown_held_grenade(commands: &mut Commands, user: Entity, grenade: &GrenadeUser) {
	commands.entity(grenade.held).try_despawn();
	commands.entity(user).remove::<(GrenadeUser, GrenadeThrow, PlayerUse)>();
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

fn despawn_orphaned_held_grenades(
	mut commands: Commands,
	held: Query<Entity, (With<HeldGrenade>, Without<HeldByGrenade>)>,
) {
	for entity in &held {
		commands.entity(entity).try_despawn();
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

	#[test]
	fn orphaned_held_grenade_despawns_when_the_user_is_gone() {
		let mut world = World::new();
		let held = world.spawn(HeldGrenade { scale: 1.0 }).id();
		world.run_system_once(despawn_orphaned_held_grenades).expect("orphan");
		assert!(!world.entities().contains(held));
	}

	#[test]
	fn held_grenade_stays_while_linked() {
		let mut world = World::new();
		let held = world.spawn(HeldGrenade { scale: 1.0 }).id();
		world.spawn(GrenadeUser::holding(held));
		world.flush();
		world.run_system_once(despawn_orphaned_held_grenades).expect("linked");
		assert!(world.entities().contains(held));
	}
}
