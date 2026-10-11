//! World first-encounter: request icons when [`WorldPlayerLoadout`] changes.

use bevy::prelude::*;
use item_icons::{CaptureClient, ItemIconDiscovery, ItemIconSystems, request_inventory_icons};

use crate::weapon::WorldPlayerLoadout;

pub use item_icons::{
	ITEM_ICON_RESOLUTION, InventoryItemSubject, ItemIconCache,
	ItemIconPlugin as ItemIconsCorePlugin, discover_icon_cache_dir, item_icon_requests,
};

/// World wrapper: core icon plugin plus loadout discovery.
pub struct ItemIconPlugin;

impl Plugin for ItemIconPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<ItemIconsCorePlugin>() {
			app.add_plugins(ItemIconsCorePlugin);
		}
		app.add_systems(Update, discover_loadout_items.in_set(ItemIconSystems::Discover));
	}
}

fn discover_loadout_items(
	loadout: Option<Res<WorldPlayerLoadout>>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut client: CaptureClient,
) {
	let Some(loadout) = loadout else {
		return;
	};
	if !loadout.is_changed() && discovery.requested_len() > 0 {
		return;
	}
	request_inventory_icons(&loadout.inventory, &mut discovery, &mut client);
}
