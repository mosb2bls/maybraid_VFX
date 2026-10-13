//! Resolve the currently shown [`LodLevelRoot`] under a host.

use bevy::prelude::*;
use lod::{lod_root_is_shown, LodLevelRoot, LodLevelRoots};

/// Shown [`LodLevelRoot`] under `host`, if any.
pub fn shown_level_root(
	host: Entity,
	children_q: &Query<&Children>,
	level_roots_bags: &Query<(), With<LodLevelRoots>>,
	root_keys: &Query<&LodLevelRoot>,
	visibilities: &Query<&Visibility>,
) -> Option<Entity> {
	let host_kids = children_q.get(host).ok()?;
	for kid in host_kids.iter() {
		if !level_roots_bags.contains(kid) {
			continue;
		}
		let Ok(root_kids) = children_q.get(kid) else {
			continue;
		};
		for root_e in root_kids.iter() {
			if root_keys.get(root_e).is_err() {
				continue;
			}
			let Ok(vis) = visibilities.get(root_e) else {
				continue;
			};
			if lod_root_is_shown(*vis) {
				return Some(root_e);
			}
		}
	}
	None
}
