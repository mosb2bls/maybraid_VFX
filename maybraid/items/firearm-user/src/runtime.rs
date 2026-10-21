//! Trigger, routing, recoil clocks, and equipment realize.
//!
//! Held-mesh pose / reticle / rumble stay in [`crate::presentation`].

use bevy::prelude::*;
use firearms::FirearmWeaponSystems;
use maybraid_equipment::{EquipmentSystems, PendingEquipmentHold, PendingEquipmentTeardown};
use player::{PlayerSystems, PlayerUse};
use player_camera::PlayerCameraSystems;

use crate::fire::{
	advance_weapon_recoil, apply_fire_intents, apply_fire_routing, queue_weapon_recoil,
};
use crate::weapon::live_weapon_from_stats;
use crate::{FirearmUser, FirearmUserSettings, FirearmUserSystems, GeneratedFirearm};

#[derive(Resource, Default)]
struct FirearmUserRuntime;

/// Trigger translation, routing, and recoil. Tests and production call this
/// same function. The fire clock lives in [`firearms::register_firearm_runtime`].
/// Pose / reticle / audio stay on [`crate::register_firearm_user_presentation`].
pub fn register_firearm_user_runtime(app: &mut App) {
	if app.world().contains_resource::<FirearmUserRuntime>() {
		return;
	}
	firearms::register_firearm_runtime(app);
	app.init_resource::<FirearmUserRuntime>()
		.add_message::<maybraid_input::PadRumble>()
		.add_message::<maybraid_equipment::EquippedItemUse>()
		.add_message::<player_camera::CameraRecoilHandoff>()
		.add_systems(
			Update,
			(apply_fire_intents, apply_fire_routing, stamp_projectile_sources)
				.chain()
				.in_set(PlayerSystems::Intent),
		)
		.add_systems(
			Update,
			(realize_equipment_teardown, realize_equipment_hold.after(realize_equipment_teardown))
				.in_set(EquipmentSystems::Realize),
		)
		.add_systems(PostUpdate, queue_weapon_recoil.after(FirearmWeaponSystems::Fire))
		.add_systems(
			Update,
			(advance_weapon_recoil, player_camera::apply_recoil_handoff)
				.chain()
				.in_set(PlayerCameraSystems::Body)
				.in_set(FirearmUserSystems::Recoil),
		);
}

fn stamp_projectile_sources(
	users: Query<(Entity, &FirearmUser)>,
	missing: Query<(), Without<firearms::ProjectileSource>>,
	mut commands: Commands,
) {
	for (entity, user) in &users {
		if missing.contains(user.held) {
			commands.entity(user.held).insert(firearms::ProjectileSource(entity));
		}
	}
}

/// Despawn the held kit and drop the user relationship.
pub fn teardown_held_firearm(commands: &mut Commands, user: Entity, firearm: &FirearmUser) {
	commands.entity(firearm.held).try_despawn();
	commands.entity(user).remove::<(FirearmUser, PlayerUse)>();
}

fn realize_equipment_teardown(
	mut commands: Commands,
	pending: Query<(Entity, &FirearmUser), With<PendingEquipmentTeardown>>,
) {
	for (entity, firearm) in &pending {
		teardown_held_firearm(&mut commands, entity, firearm);
	}
}

fn realize_equipment_hold(mut commands: Commands, pending: Query<(Entity, &PendingEquipmentHold)>) {
	use crate::pose::spawn_held_kit;
	use character_items::InventoryItem;

	for (entity, hold) in &pending {
		let Some(InventoryItem::Firearm { spec, stats }) = hold.inventory().primary_weapon() else {
			continue;
		};
		let sheet = hold.inventory().character_sheet();
		let live = live_weapon_from_stats(*stats, sheet.damage).with_weapon_identity(spec);
		spawn_held_kit(
			&mut commands,
			entity,
			FirearmUserSettings::default(),
			GeneratedFirearm::from_spec(*spec),
			live,
		);
	}
}
