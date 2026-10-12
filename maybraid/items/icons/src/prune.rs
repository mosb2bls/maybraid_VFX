//! Drop icon-cache PNGs that no saved inventory still needs.

use bevy::prelude::*;
use character_inventory_user;
use character_items::Inventory;
use character_persist::SaveRoot;
use screen_capture::{CaptureCache, CaptureCacheKey, CaptureCacheRoot, CaptureLighting};
use std::collections::HashSet;
use std::fs;

use crate::{item_icon_requests, CaptureRevision};

/// Impulse: prune `.maybraid/icon-cache` against player save inventories.
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestPruneItemIconCache;

pub fn kept_icon_file_names<'a>(
	inventories: impl IntoIterator<Item = &'a Inventory>,
) -> HashSet<String> {
	let mut keep = HashSet::new();
	for inventory in inventories {
		for item in &inventory.items {
			for request in
				item_icon_requests(item, CaptureLighting::studio(), CaptureRevision::CURRENT)
			{
				keep.insert(CaptureCacheKey::from_request(&request).file_name());
			}
		}
	}
	keep
}

pub fn prune_icon_cache(cache: &CaptureCache, keep: &HashSet<String>) -> usize {
	let Ok(entries) = fs::read_dir(cache.root()) else {
		return 0;
	};
	let mut removed = 0;
	for entry in entries.flatten() {
		let path = entry.path();
		if path.extension().and_then(|ext| ext.to_str()) != Some("png") {
			continue;
		}
		let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
			continue;
		};
		if keep.contains(name) {
			continue;
		}
		if fs::remove_file(&path).is_ok() {
			removed += 1;
		}
	}
	removed
}

/// Keep only PNGs for items still in saved character inventories.
pub fn prune_icon_cache_to_saved_inventories(save_root: &SaveRoot, cache: &CaptureCache) -> usize {
	let Ok(ids) = save_root.list_ids() else {
		return 0;
	};
	let inventories: Vec<Inventory> = ids
		.into_iter()
		.filter_map(|id| character_inventory_user::load(save_root, id).ok())
		.collect();
	let keep = kept_icon_file_names(inventories.iter());
	prune_icon_cache(cache, &keep)
}

pub fn prune_icon_cache_on_request(
	mut commands: Commands,
	requests: Query<Entity, With<RequestPruneItemIconCache>>,
	save_root: Option<Res<SaveRoot>>,
	cache: Option<Res<CaptureCacheRoot>>,
) {
	if requests.is_empty() {
		return;
	}
	for entity in &requests {
		commands.entity(entity).try_despawn();
	}
	let (Some(save_root), Some(cache)) = (save_root, cache) else {
		return;
	};
	let removed = prune_icon_cache_to_saved_inventories(&save_root, &cache.cache);
	if removed > 0 {
		info!("pruned {removed} icon-cache file(s) not in saved inventories");
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use character_items::{FirearmMesh, InventoryItem};
	use character_persist::CharacterId;
	use screen_capture::CaptureCache;
	use std::fs;

	#[test]
	fn prune_drops_pngs_that_no_save_still_owns() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let save = SaveRoot::at(dir.path().join("saves"));
		save.ensure_dirs()?;
		let id = CharacterId::new();
		fs::write(save.character_path(id), "{}")?;
		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let inventory = Inventory::with_starter_outfit(vec![item.clone()]);
		character_inventory_user::save(&save, id, &inventory)?;

		let cache = CaptureCache::new(dir.path().join("icons"));
		fs::create_dir_all(cache.root())?;
		let keep = kept_icon_file_names(std::iter::once(&inventory));
		assert!(!keep.is_empty());
		let keep_name = keep.iter().next().cloned().ok_or_else(|| anyhow::anyhow!("keep"))?;
		fs::write(cache.root().join(&keep_name), b"keep")?;
		fs::write(cache.root().join("orphan_shaded_256x256_deadbeef.png"), b"drop")?;

		assert_eq!(prune_icon_cache_to_saved_inventories(&save, &cache), 1);
		assert!(cache.root().join(&keep_name).is_file());
		assert!(!cache.root().join("orphan_shaded_256x256_deadbeef.png").is_file());
		Ok(())
	}

	#[test]
	fn empty_roster_clears_the_icon_cache() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let save = SaveRoot::at(dir.path().join("saves"));
		save.ensure_dirs()?;
		let cache = CaptureCache::new(dir.path().join("icons"));
		fs::create_dir_all(cache.root())?;
		fs::write(cache.root().join("stale_silhouette_256x256_abcd.png"), b"drop")?;
		assert_eq!(prune_icon_cache_to_saved_inventories(&save, &cache), 1);
		Ok(())
	}
}
