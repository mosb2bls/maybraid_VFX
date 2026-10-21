//! Routing cancel / drop and swap / death interrupt.

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{Inventory, InventoryItem};
use damage::Downed;
use firearm_user::WeaponSwap;
use grenades::{DropLiveGrenade, GrenadeEffect};
use maybraid_controllers_core::{InputSource, RoutingEvent, RoutingKind};

use super::{GrenadeThrow, GrenadeUser};
use crate::hold::HeldGrenade;

/// Cancel or drop from published [`RoutingEvent`] messages.
///
/// Each event is applied to the actor it names. Disconnect / reassign /
/// removal drop a live cook on [`RoutingEvent::lost_actor`]. Capture only
/// cancels a leftover latch on that same owner; a pin-pulled charge keeps
/// cooking. Cleanup does not read [`maybraid_equipment::RoutedItemActor`].
pub fn apply_throw_routing(
	mut events: MessageReader<RoutingEvent>,
	mut drops: MessageWriter<DropLiveGrenade>,
	velocities: Query<&LinearVelocity>,
	carriers: Query<&InventoryUser>,
	mut bags: Query<&mut Inventory>,
	held: Query<&Transform, With<HeldGrenade>>,
	mut users: Query<(
		Entity,
		&GrenadeUser,
		&mut GrenadeThrow,
		Option<&Transform>,
		Option<&GlobalTransform>,
	)>,
) {
	let source = InputSource::local_pad();
	for event in events.read() {
		if event.source() != source {
			continue;
		}
		let Some(actor) = event.lost_actor() else {
			continue;
		};
		let Ok((user_entity, user, mut throw, local, global)) = users.get_mut(actor.entity())
		else {
			continue;
		};
		match event.kind() {
			RoutingKind::Captured => {
				throw.apply_routing(*event);
			}
			RoutingKind::Disconnected | RoutingKind::Removed | RoutingKind::Reassigned => {
				let Some(fuse) = throw.apply_routing(*event) else {
					continue;
				};
				drop_live_cook(
					&mut drops,
					&velocities,
					&carriers,
					&mut bags,
					&held,
					user_entity,
					user,
					local,
					global,
					fuse,
				);
			}
			_ => {}
		}
	}
}

pub fn interrupt_live_cooks(
	mut drops: MessageWriter<DropLiveGrenade>,
	velocities: Query<&LinearVelocity>,
	carriers: Query<&InventoryUser>,
	mut bags: Query<&mut Inventory>,
	held: Query<&Transform, With<HeldGrenade>>,
	mut users: Query<
		(Entity, &GrenadeUser, &mut GrenadeThrow, Option<&Transform>, Option<&GlobalTransform>),
		Or<(With<Downed>, With<WeaponSwap>)>,
	>,
) {
	for (user_entity, user, mut throw, local, global) in &mut users {
		let Some(fuse) = throw.take_live_cook() else {
			continue;
		};
		drop_live_cook(
			&mut drops,
			&velocities,
			&carriers,
			&mut bags,
			&held,
			user_entity,
			user,
			local,
			global,
			fuse,
		);
	}
}

fn drop_live_cook(
	drops: &mut MessageWriter<DropLiveGrenade>,
	velocities: &Query<&LinearVelocity>,
	carriers: &Query<&InventoryUser>,
	bags: &mut Query<&mut Inventory>,
	held: &Query<&Transform, With<HeldGrenade>>,
	user_entity: Entity,
	user: &GrenadeUser,
	local: Option<&Transform>,
	global: Option<&GlobalTransform>,
	fuse: f32,
) {
	let Some(stats) = carriers
		.get(user_entity)
		.ok()
		.and_then(|carrier| bags.get(carrier.bag).ok())
		.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
	else {
		return;
	};
	let spec = carriers
		.get(user_entity)
		.ok()
		.and_then(|carrier| bags.get(carrier.bag).ok())
		.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_spec))
		.unwrap_or_default();
	let origin = held
		.get(user.held)
		.ok()
		.map(|pose| pose.translation)
		.or_else(|| global.map(GlobalTransform::translation))
		.or_else(|| local.map(|transform| transform.translation))
		.unwrap_or(Vec3::ZERO);
	let inherit = velocities.get(user_entity).map(|velocity| velocity.0).unwrap_or(Vec3::ZERO);
	start_primary_recharge(carriers, bags, user_entity);
	drops.write(DropLiveGrenade {
		origin,
		velocity: inherit * stats.inherit_velocity,
		spec,
		stats,
		effect: GrenadeEffect::from_stats(&stats),
		fuse,
		source: user_entity,
	});
}

pub(super) fn start_primary_recharge(
	carriers: &Query<&InventoryUser>,
	bags: &mut Query<&mut Inventory>,
	user_entity: Entity,
) {
	if let Ok(carrier) = carriers.get(user_entity) {
		if let Ok(mut bag) = bags.get_mut(carrier.bag) {
			if let Some(InventoryItem::Grenade { recharge, stats, .. }) = bag.primary_weapon_mut() {
				recharge.start(stats.recharge);
			}
		}
	}
}
