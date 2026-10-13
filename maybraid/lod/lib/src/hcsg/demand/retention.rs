//! Retention planning and worker sweeps when the live set changes.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::PoisonError;

use super::schedule::{pick_job, Job};
use super::state::{lock_mutex, DemandState};
use super::HcsgDemand;

/// What the worker should do next: sweep, fill a quantum, or stop.
pub(crate) enum WorkerWait {
	Job(Job),
	/// Regions to retain, keyed by the types live subscriptions reach.
	Sweep(HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>>),
	Shutdown,
}

pub(super) fn retention_plan(
	state: &DemandState,
) -> HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>> {
	let mut regions_by_type: HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>> = HashMap::new();
	for subscription in state.subscriptions.values() {
		for type_id in lock_mutex(&subscription.reach).iter() {
			regions_by_type
				.entry(*type_id)
				.or_default()
				.extend(subscription.regions.iter().copied());
		}
	}
	regions_by_type
}

impl HcsgDemand {
	/// The next sweep or unfinished subscription. Blocks until there is one.
	/// Sweep is taken first whenever the live set has changed.
	pub(in crate::hcsg) fn next_work(&self) -> WorkerWait {
		let mut state = self.lock();
		loop {
			if state.shutdown {
				return WorkerWait::Shutdown;
			}
			if state.sweep_needed {
				state.sweep_needed = false;
				self.0.sweeps.fetch_add(1, Ordering::Relaxed);
				return WorkerWait::Sweep(retention_plan(&state));
			}
			if let Some(job) = pick_job(&mut state) {
				state.working = true;
				self.0.picks.fetch_add(1, Ordering::Relaxed);
				return WorkerWait::Job(job);
			}
			self.0.idle.notify_all();
			state = self.0.wake.wait(state).unwrap_or_else(PoisonError::into_inner);
		}
	}

	/// Applies a pending sweep plan without a worker. Tests use this after a
	/// mid-quantum replacement.
	#[cfg(test)]
	pub(crate) fn try_sweep(&self) -> Option<HashMap<TypeId, Vec<bevy::math::bounding::Aabb3d>>> {
		let mut state = self.lock();
		if !state.sweep_needed {
			return None;
		}
		state.sweep_needed = false;
		self.0.sweeps.fetch_add(1, Ordering::Relaxed);
		Some(retention_plan(&state))
	}
}
