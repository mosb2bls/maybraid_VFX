//! App wiring shared by every generation and presentation plugin.

use bevy::log::{error, info};
use bevy::prelude::{App, IntoScheduleConfigs, Last, Local, Res, SystemSet, Time, Update};

use crate::scene::LodRefreshSystems;

use super::demand::HcsgDemand;
use super::presentation::despawn_retired_hosts;
use super::session::{configure_session_before_hcsg_systems, ensure_session};
use super::storage::HcsgStorage;
use super::worker::HcsgWorker;

/// Set to any value to log per-type store sizes and rebuild-after-eviction
/// counts every few seconds.
const DIAG_ENV: &str = "MAYBRAID_HCSG_DIAG";
const DIAG_INTERVAL_SECS: f32 = 5.0;

/// Generation requests and presentation host reconciliation. Runs before the
/// LOD refresh chain, so scene systems see this frame's hosts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub struct HcsgSystems;

/// Storage, demand and one worker, once per app.
pub(super) fn ensure_runtime(app: &mut App) {
	if app.world().contains_resource::<HcsgWorker>() {
		return;
	}
	app.configure_sets(Update, HcsgSystems.before(LodRefreshSystems::Track))
		.add_systems(Last, despawn_retired_hosts);
	if std::env::var_os(DIAG_ENV).is_some() {
		app.add_systems(Update, log_store_diagnostics);
	}
	let storage = app.world_mut().get_resource_or_init::<HcsgStorage>().clone();
	let demand = app.world_mut().get_resource_or_init::<HcsgDemand>().clone();
	match HcsgWorker::spawn(storage, demand) {
		Ok(worker) => {
			app.insert_resource(worker);
		}
		Err(err) => error!("hcsg: failed to spawn the generation worker: {err}"),
	}
	ensure_session(app);
	configure_session_before_hcsg_systems(app);
}

fn log_store_diagnostics(storage: Res<HcsgStorage>, time: Res<Time>, mut last: Local<f32>) {
	*last += time.delta_secs();
	if *last < DIAG_INTERVAL_SECS {
		return;
	}
	*last = 0.0;
	let (top, nested) = storage.rebuilds_after_eviction();
	info!(
		target: "hcsg",
		"stores={:?} rebuilds_after_eviction=({top}, {nested})",
		storage.store_sizes()
	);
}
