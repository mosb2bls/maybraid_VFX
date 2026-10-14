//! Helpers for integration tests that drive the real HCSG worker through a
//! [`bevy::app::App`]. Production code should not call these.

use std::time::Duration;

use bevy::ecs::entity_disabling::Disabled;
use bevy::prelude::*;

use crate::gen::{Id, Version};

use super::{HcsgDemand, HcsgNode, HcsgValue, RetiredHost};

/// [`settle_presentation`] waited longer than [`SettlePresentation::idle_timeout`].
#[derive(Debug, Clone, Copy, thiserror::Error)]
#[error("HCSG worker did not go idle before the timeout ({timeout:?})")]
pub struct SettleTimedOut {
	pub timeout: Duration,
}

/// How long to wait for the worker and how many frames to run after it idles.
#[derive(Debug, Clone, Copy)]
pub struct SettlePresentation {
	/// Passed to [`HcsgDemand::wait_idle`].
	pub idle_timeout: Duration,
	/// Subscribe (or resubscribe), then wait for idle; repeated when discovery
	/// needs a second pass.
	pub idle_rounds: u32,
	/// Extra `App::update` frames after the worker idles so presentation can
	/// spawn hosts and chunk fulfill can run.
	pub reconcile_frames: u32,
}

impl SettlePresentation {
	/// Lod `hcsg` plugin tests: two idle passes and one reconcile frame.
	pub const PLUGIN_TESTS: Self =
		Self { idle_timeout: Duration::from_secs(10), idle_rounds: 2, reconcile_frames: 1 };

	/// Layer integration tests that fulfill LOD scenes (Richmond, Chico, …).
	pub fn layer_integration(idle_timeout: Duration) -> Self {
		Self { idle_timeout, idle_rounds: 2, reconcile_frames: 8 }
	}

	/// Durham terrain/water presentation (fewer fulfill frames).
	pub fn durham_integration(idle_timeout: Duration) -> Self {
		Self { idle_timeout, idle_rounds: 2, reconcile_frames: 4 }
	}

	/// Geneva language overlay (extra idle round for gated bounds).
	pub fn geneva_integration(idle_timeout: Duration) -> Self {
		Self { idle_timeout, idle_rounds: 3, reconcile_frames: 8 }
	}
}

/// Subscribe (or notice a dropped subscription and resubscribe), let the worker
/// finish, then run enough frames for hosts to reconcile.
pub fn settle_presentation(app: &mut App, opts: SettlePresentation) -> Result<(), SettleTimedOut> {
	for _ in 0..opts.idle_rounds {
		app.update();
		let demand = app.world().resource::<HcsgDemand>().clone();
		if !demand.wait_idle(opts.idle_timeout) {
			return Err(SettleTimedOut { timeout: opts.idle_timeout });
		}
	}
	for _ in 0..opts.reconcile_frames {
		app.update();
	}
	Ok(())
}

/// Leftover [`RetiredHost`]s still waiting on the `Last` prefix.
pub fn leftover_retired_hosts(app: &mut App) -> usize {
	app.world_mut()
		.query_filtered::<Entity, (With<RetiredHost>, Allow<Disabled>)>()
		.iter(app.world())
		.count()
}

/// Extra `App::update` frames until the retired-host `Last` budget is empty.
///
/// Leave retires every layer's hosts in one wave. [`settle_presentation`] only
/// waits for presented (non-[`RetiredHost`]) hosts, so a neighborhood cascade
/// can still have leftovers after reconcile — the cull-root cap is two per
/// frame, and other layers consume that prefix first. Returns how many are
/// still waiting after `max_frames`.
pub fn drain_retired_hosts(app: &mut App, max_frames: u32) -> usize {
	for _ in 0..max_frames {
		if leftover_retired_hosts(app) == 0 {
			return 0;
		}
		app.update();
	}
	leftover_retired_hosts(app)
}

/// Sorted `(id, version)` for every [`HcsgNode`] of `T` in the world.
pub fn hcsg_hosts<T: HcsgValue>(app: &mut App) -> Vec<(Id, Version)> {
	let mut hosts: Vec<_> = app
		.world_mut()
		.query_filtered::<&HcsgNode<T>, Without<RetiredHost>>()
		.iter(app.world())
		.map(|node| (node.id, node.version))
		.collect();
	hosts.sort();
	hosts
}

/// Every `before` id is presented at a strictly newer version.
///
/// After a session restart, presentation keeps the previous session until the
/// worker publishes replacements. A missing id is still mid-replace.
pub fn hosts_replaced(before: &[(Id, Version)], after: &[(Id, Version)]) -> bool {
	let after: std::collections::BTreeMap<Id, Version> = after.iter().copied().collect();
	!before.is_empty()
		&& before.iter().all(|(id, old)| after.get(id).is_some_and(|new| *new > *old))
}

/// Extra frames until [`hosts_replaced`] holds.
///
/// [`settle_presentation`] uses a fixed reconcile budget. A busy store or a
/// still-running publish pass can leave one host on the previous session after
/// those frames — this waits on the version condition instead.
pub fn wait_replaced_hosts<T: HcsgValue>(
	app: &mut App,
	before: &[(Id, Version)],
	idle_timeout: Duration,
	max_frames: u32,
) -> Result<Vec<(Id, Version)>, SettleTimedOut> {
	for _ in 0..max_frames {
		let after = hcsg_hosts::<T>(app);
		if hosts_replaced(before, &after) {
			return Ok(after);
		}
		app.update();
		let demand = app.world().resource::<HcsgDemand>().clone();
		if !demand.wait_idle(idle_timeout) {
			return Err(SettleTimedOut { timeout: idle_timeout });
		}
	}
	Ok(hcsg_hosts::<T>(app))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn hosts_replaced_requires_every_id_at_a_newer_version() -> anyhow::Result<()> {
		let before = [(Id::Universal, Version(3))];
		assert!(!hosts_replaced(&before, &[]));
		assert!(!hosts_replaced(&before, &[(Id::Universal, Version(3))]));
		assert!(hosts_replaced(&before, &[(Id::Universal, Version(4))]));
		Ok(())
	}
}
