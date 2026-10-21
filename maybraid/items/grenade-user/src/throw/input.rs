//! Addressed use → cook start.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::{GrenadeStats, Inventory, InventoryItem};
use firearm_user::WeaponSwap;
use maybraid_controllers_core::InputRouter;
use maybraid_equipment::{EquippedItemUse, delivered_item_use};

use super::{GrenadeThrow, GrenadeUser, ThrowJingleDue};

pub fn apply_throw_intents(
	router: Option<Res<InputRouter>>,
	mut uses: MessageReader<EquippedItemUse>,
	bags: Query<&Inventory>,
	carriers: Query<&InventoryUser>,
	mut commands: Commands,
	mut users: Query<(Entity, &mut GrenadeThrow, Has<WeaponSwap>), With<GrenadeUser>>,
) {
	let router = router.as_deref();
	let uses: Vec<_> = uses
		.read()
		.copied()
		.filter(|use_item| delivered_item_use(router, *use_item))
		.collect();
	for (entity, mut throw, swapping) in &mut users {
		let Some(use_item) = uses
			.iter()
			.rev()
			.find(|use_item| use_item.addresses(entity) && use_item.handler().is_grenade())
		else {
			// Lifecycle events own cancel / freeze. Missing use is not a release.
			continue;
		};
		throw.apply_use(use_item.held());
		if !use_item.held() {
			continue;
		}
		let ready = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(Inventory::primary_weapon)
			.is_some_and(|item| match item {
				InventoryItem::Grenade { recharge, .. } => recharge.ready(),
				_ => false,
			});
		let fuse = carriers
			.get(entity)
			.ok()
			.and_then(|carrier| bags.get(carrier.bag).ok())
			.and_then(|bag| bag.primary_weapon().and_then(InventoryItem::grenade_stats))
			.map(|stats| stats.fuse)
			.unwrap_or_else(|| GrenadeStats::standard().fuse);
		if throw.try_start_cook(ready, swapping, fuse) {
			commands.entity(entity).try_insert(ThrowJingleDue);
		}
	}
}
