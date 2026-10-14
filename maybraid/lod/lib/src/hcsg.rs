//! Thread-shared HCSG runtime ([#1005](https://github.com/ramate-io/maybraid/issues/1005),
//! [#1007](https://github.com/ramate-io/maybraid/issues/1007)): generation runs on a
//! worker, the frame only subscribes bounds and reads published values. The design
//! is `hcsg/README.md`, next to this module.
//!
//! - [`HcsgStorage`]: a cloneable handle over one locked node store of `Arc<T>`
//!   per type. Locks cover lookup and publication only.
//! - [`GenerationContext`]: hands schemes owned `Arc` dependencies and generates
//!   what is missing.
//! - [`HcsgDemand`]: one subscription per bounds source, and the epoch that
//!   sessions advance.
//! - Worker thread: fills subscriptions (started by [`runtime`] / [`spawn_worker`] in tests).
//! - [`HcsgRegions<C>`]: the boxes channel `C` wants values in, sent by
//!   [`HcsgBounds`] producers.
//! - [`GenerationPlugin`]: keeps `T` warm within `C`'s regions, fire and forget.
//! - [`PresentationPlugin`]: keeps one [`HcsgNode<T>`] host per published value
//!   within `C`'s regions.
//! - [`HcsgNode<T>`]: forwards the LOD scene traits to `T`.

mod bounds;
mod context;
mod demand;
mod generation;
mod node;
mod node_store;
mod presentation;
mod runtime;
mod session;
mod storage;
mod test_support;
mod worker;

#[cfg(test)]
mod perf;
#[cfg(test)]
mod system_tests;
#[cfg(test)]
mod tests;

pub use bounds::{
	viewer_focus, Gated, HcsgBounds, HcsgBoundsPlugin, HcsgClass, HcsgGate, HcsgRegions,
	LodViewers, ViewerHcsgBounds,
};
pub use context::{GenerationContext, GenerationScheme};
pub use demand::{HcsgDemand, Outstanding, Published, SubscriptionId};
pub use generation::GenerationPlugin;
pub use node::HcsgNode;
pub use node_store::StoredEntry;
pub use presentation::{PresentationPlugin, RetiredHost};
pub use runtime::HcsgSystems;
pub use session::{
	register_session_seed, request_hcsg_session_restart, HcsgRestartRequest, HcsgSessionPlugin,
	HcsgSessionRestarted, HcsgSessionSeed,
};
pub use storage::{Busy, HcsgStorage, HcsgValue};
pub use test_support::{
	drain_retired_hosts, hcsg_hosts, hosts_replaced, leftover_retired_hosts, settle_presentation,
	wait_replaced_hosts, SettlePresentation, SettleTimedOut,
};

/// Keeps a test [`worker::HcsgWorker`] alive until dropped.
/// Allow unused since this a guard object.
#[allow(unused)]
pub struct TestWorkerGuard(worker::HcsgWorker);

/// Starts the shared generation worker. Production apps rely on
/// [`PresentationPlugin`] / [`GenerationPlugin`] to install one worker; call
/// this only in tests that subscribe [`HcsgDemand`] without those plugins.
pub fn spawn_worker(storage: HcsgStorage, demand: HcsgDemand) -> std::io::Result<TestWorkerGuard> {
	worker::HcsgWorker::spawn(storage, demand).map(TestWorkerGuard)
}

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

/// Bounds for universal entries: everything a region query could ask about.
pub fn universal_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::splat(-1.0e9), Vec3::splat(1.0e9))
}

/// Declares a scheme's spatial index scale and eviction margin (defaults match).
#[macro_export]
macro_rules! hcsg_index_scale {
	($scale:expr) => {
		const INDEX_SCALE: bevy::math::DVec3 = $scale;
		const RETENTION_MARGIN: bevy::math::DVec3 = $scale;
	};
}

/// Declares `T` a seeded session root at `Id::Universal`. It is never built,
/// only [`HcsgStorage::seed`].
#[macro_export]
macro_rules! seeded_root {
	($T:ty) => {
		impl $crate::hcsg::GenerationScheme for $T {
			fn original_ids_for(
				_cx: &mut $crate::hcsg::GenerationContext,
				_region: bevy::math::bounding::Aabb3d,
			) -> Vec<$crate::gen::OriginalId> {
				vec![$crate::gen::OriginalId::universal()]
			}

			fn build_with_id(
				_cx: &mut $crate::hcsg::GenerationContext,
				_id: $crate::gen::Id,
			) -> Option<(Self, bevy::math::bounding::Aabb3d)> {
				None
			}
		}
	};
}
