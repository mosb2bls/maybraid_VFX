//! World first-encounter: request icons when [`WorldPlayerLoadout`] changes.

use bevy::prelude::*;
use character_inventory_user::InventoryUser;
use character_items::Inventory;
use item_icons::{
	request_inventory_icons, request_self_lubikan_icon, CaptureClient, ItemCaptureSubjects,
	ItemIconDiscovery, ItemIconSystems, PoseCaptureSubjects,
};
use tanz_user::request_inventory_tanz_icons;

use crate::weapon::WorldPlayerLoadout;

pub use item_icons::{
	discover_icon_cache_dir, item_icon_requests, InventoryItemSubject, ItemIconCache,
	ItemIconPlugin as ItemIconsCorePlugin, ITEM_ICON_RESOLUTION,
};

/// World wrapper: core icon plugin plus loadout discovery.
pub struct ItemIconPlugin;

impl Plugin for ItemIconPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ItemIconsCorePlugin>() {
			app.add_plugins(ItemIconsCorePlugin);
		}
		app.add_systems(
			Update,
			(discover_loadout_items, discover_tanz_icons).in_set(ItemIconSystems::Discover),
		);
	}
}

fn discover_loadout_items(
	loadout: Option<Res<WorldPlayerLoadout>>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut cache: ResMut<ItemIconCache>,
	mut client: CaptureClient,
) {
	let Some(loadout) = loadout else {
		return;
	};
	if !loadout.is_changed() && discovery.requested_len() > 0 {
		return;
	}
	request_self_lubikan_icon(&mut discovery, &mut client, &mut cache);
	request_inventory_icons(
		&loadout.inventory,
		&mut discovery,
		&mut subjects,
		&mut client,
		&mut cache,
	);
}

fn discover_tanz_icons(
	loadout: Option<Res<WorldPlayerLoadout>>,
	users: Query<&InventoryUser>,
	bags: Query<&Inventory>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut poses: ResMut<PoseCaptureSubjects>,
	mut cache: ResMut<ItemIconCache>,
	mut client: CaptureClient,
) {
	if let Some(loadout) = loadout.as_deref() {
		request_inventory_tanz_icons(
			&loadout.inventory,
			&mut discovery,
			&mut poses,
			&mut client,
			&mut cache,
		);
	}
	for user in &users {
		let Ok(bag) = bags.get(user.bag) else {
			continue;
		};
		request_inventory_tanz_icons(bag, &mut discovery, &mut poses, &mut client, &mut cache);
	}
}
