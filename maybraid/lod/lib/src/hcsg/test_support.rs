//! Helpers for integration tests that drive the real HCSG worker through a
//! [`bevy::app::App`]. Production code should not call these.

use std::time::Duration;

use bevy::prelude::*;

use crate::gen::{Id, Version};

use super::{HcsgDemand, HcsgNode, HcsgValue};

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
	pub const PLUGIN_TESTS: Self = Self {
		idle_timeout: Duration::from_secs(10),
		idle_rounds: 2,
		reconcile_frames: 1,
	};

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
pub fn settle_presentation(
	app: &mut App,
	opts: SettlePresentation,
) -> Result<(), SettleTimedOut> {
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

/// Sorted `(id, version)` for every [`HcsgNode`] of `T` in the world.
pub fn hcsg_hosts<T: HcsgValue>(app: &mut App) -> Vec<(Id, Version)> {
	let mut hosts: Vec<_> = app
		.world_mut()
		.query::<&HcsgNode<T>>()
		.iter(app.world())
		.map(|node| (node.id, node.version))
		.collect();
	hosts.sort();
	hosts
}
