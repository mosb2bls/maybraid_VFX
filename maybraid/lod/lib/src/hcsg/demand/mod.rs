//! [`HcsgDemand`]: what the worker should fill, one subscription per system.

mod outstanding;
mod registry;
mod retention;
mod schedule;
mod state;

use std::sync::{Arc, MutexGuard, PoisonError, TryLockError};
use std::time::{Duration, Instant};

use bevy::prelude::Resource;

use state::DemandShared;

use super::storage::Busy;

pub use outstanding::Outstanding;
pub use registry::Published;
pub use state::SubscriptionId;

pub(crate) use retention::WorkerWait;
pub(crate) use schedule::{quantum_cost, Job, QuantumProgress, QUANTUM_IDS, QUANTUM_TIME};

/// One subscription per generation or presentation system, filled in
/// weighted quanta by the [`super::HcsgWorker`].
///
/// A subscription's regions never change: new regions replace the subscription,
/// which cancels whatever the worker was still doing for the old one. A
/// replacement inherits the predecessor's stride `pass` (floored at the current
/// minimum among remaining live work) and reach, so resubscribing cannot jump
/// the queue, bank credit, or drop dependencies between fills.
#[derive(Resource, Clone, Default)]
pub struct HcsgDemand(Arc<DemandShared>);

impl HcsgDemand {
	fn lock(&self) -> MutexGuard<'_, state::DemandState> {
		self.0.state.lock().unwrap_or_else(PoisonError::into_inner)
	}

	fn try_lock(&self) -> Result<MutexGuard<'_, state::DemandState>, Busy> {
		match self.0.state.try_lock() {
			Ok(state) => Ok(state),
			Err(TryLockError::Poisoned(poisoned)) => Ok(poisoned.into_inner()),
			Err(TryLockError::WouldBlock) => Err(Busy),
		}
	}

	#[cfg(test)]
	pub(crate) fn epoch(&self) -> u64 {
		self.lock().epoch
	}

	/// Blocks until no subscription has outstanding work and no sweep is
	/// pending, or `timeout` passes. `false` on timeout. Needs a running
	/// [`super::HcsgWorker`].
	pub fn wait_idle(&self, timeout: Duration) -> bool {
		let deadline = Instant::now() + timeout;
		let mut state = self.lock();
		loop {
			let busy = state.working
				|| state.sweep_needed
				|| state.subscriptions.values().any(|s| !s.done);
			if !busy {
				return true;
			}
			let Some(left) = deadline.checked_duration_since(Instant::now()) else {
				return false;
			};
			state = self.0.idle.wait_timeout(state, left).unwrap_or_else(PoisonError::into_inner).0;
		}
	}

	pub(in crate::hcsg) fn shutdown(&self) {
		self.lock().shutdown = true;
		self.0.wake.notify_all();
	}
}
