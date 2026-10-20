//! Small helpers shared by begin / drain / complete.

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;

use crate::scene::host::LodSceneHost;

use super::types::{LodLazyPending, LodSceneHostStreamed};

/// Whether `entity` is a [`LodSceneHost`], or wraps one as a direct child.
///
/// Returns `Some(streamed)` when a next-level host is found.
fn next_level_host_streamed(
	entity: Entity,
	children_q: &Query<&Children, Allow<Disabled>>,
	hosts: &Query<(), (With<LodSceneHost>, Allow<Disabled>)>,
	streamed_hosts: &Query<(), (With<LodSceneHostStreamed>, Allow<Disabled>)>,
) -> Option<bool> {
	if hosts.contains(entity) {
		return Some(streamed_hosts.contains(entity));
	}
	let Ok(kids) = children_q.get(entity) else {
		return None;
	};
	for kid in kids.iter() {
		if hosts.contains(kid) {
			return Some(streamed_hosts.contains(kid));
		}
	}
	None
}

/// Count next-level nested hosts under a level root and how many are streamed.
///
/// Returns `(required, streamed)`. `required == 0` when there are no nested hosts
/// (mesh-only roots complete immediately).
pub(super) fn count_nested_hosts(
	root: Entity,
	children_q: &Query<&Children, Allow<Disabled>>,
	hosts: &Query<(), (With<LodSceneHost>, Allow<Disabled>)>,
	streamed_hosts: &Query<(), (With<LodSceneHostStreamed>, Allow<Disabled>)>,
) -> (usize, usize) {
	let Ok(children) = children_q.get(root) else {
		return (0, 0);
	};

	let mut required = 0usize;
	let mut streamed = 0usize;
	for child in children.iter() {
		let Some(is_streamed) = next_level_host_streamed(child, children_q, hosts, streamed_hosts)
		else {
			continue;
		};
		required += 1;
		if is_streamed {
			streamed += 1;
		}
	}
	(required, streamed)
}

/// True when `root` or any descendant still holds [`LodLazyPending`].
pub(super) fn subtree_has_lod_lazy_pending(
	root: Entity,
	children_q: &Query<&Children, Allow<Disabled>>,
	lazy: &Query<(), (With<LodLazyPending>, Allow<Disabled>)>,
) -> bool {
	if lazy.contains(root) {
		return true;
	}
	let mut stack: Vec<Entity> = match children_q.get(root) {
		Ok(children) => children.iter().collect(),
		Err(_) => return false,
	};
	while let Some(entity) = stack.pop() {
		if lazy.contains(entity) {
			return true;
		}
		if let Ok(kids) = children_q.get(entity) {
			stack.extend(kids.iter());
		}
	}
	false
}
