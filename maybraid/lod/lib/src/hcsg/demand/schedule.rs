//! Stride scheduler: weighted quanta and job picking.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

use crate::gen::Id;

use super::state::SubscriptionId;
use super::state::{lock_mutex, DemandState, Discover, Generate};
use super::HcsgDemand;

/// Ids the worker generates in one quantum before returning to the scheduler.
pub const QUANTUM_IDS: usize = 32;
/// Wall time after which a quantum yields, even if fewer ids have run.
pub const QUANTUM_TIME: Duration = Duration::from_millis(30);

/// One subscription the worker is filling for a quantum.
pub(crate) struct Job {
	pub id: SubscriptionId,
	pub regions: Vec<Aabb3d>,
	pub focus: Option<Vec3>,
	pub discover: Discover,
	pub generate: Generate,
	pub cancelled: Arc<AtomicBool>,
	pub discovered: Option<Vec<Id>>,
	pub cursor: usize,
	pub reach: Arc<Mutex<HashSet<TypeId>>>,
}

/// What the worker writes back after one quantum.
#[derive(Default)]
pub(crate) struct QuantumProgress {
	pub discovered: Option<Vec<Id>>,
	pub cursor: usize,
	pub cost: f64,
	pub done: bool,
	pub reached: HashSet<TypeId>,
	/// Ids whose values landed during this quantum, in generation order.
	pub published: Vec<Id>,
}

/// Charge the larger of the id count and the time fraction of a quantum, so
/// an expensive class that hits the 30 ms cap after a few ids pays a full
/// quantum, not a handful of ids.
pub(crate) fn quantum_cost(ids: u32, elapsed: Duration) -> f64 {
	let by_ids = f64::from(ids);
	let by_time = (elapsed.as_secs_f64() / QUANTUM_TIME.as_secs_f64()) * QUANTUM_IDS as f64;
	by_ids.max(by_time)
}

pub(super) fn min_pass(state: &DemandState) -> f64 {
	state
		.subscriptions
		.values()
		.filter(|subscription| !subscription.done)
		.map(|subscription| subscription.pass)
		.min_by(|a, b| a.total_cmp(b))
		.unwrap_or(0.0)
}

pub(super) fn pick_job(state: &mut DemandState) -> Option<Job> {
	let id = state
		.subscriptions
		.iter()
		.filter(|(_, subscription)| !subscription.done)
		.min_by(|(id_a, a), (id_b, b)| a.pass.total_cmp(&b.pass).then(id_b.cmp(id_a)))
		.map(|(id, _)| *id)?;
	let subscription = state.subscriptions.get_mut(&id)?;
	Some(Job {
		id,
		regions: subscription.regions.clone(),
		focus: subscription.focus,
		discover: subscription.discover,
		generate: subscription.generate,
		cancelled: Arc::clone(&subscription.cancelled),
		discovered: subscription.discovered.take(),
		cursor: subscription.cursor,
		reach: Arc::clone(&subscription.reach),
	})
}

impl HcsgDemand {
	pub(in crate::hcsg) fn finish_quantum(
		&self,
		id: SubscriptionId,
		progress: QuantumProgress,
		reach: Option<Arc<Mutex<HashSet<TypeId>>>>,
	) {
		let mut state = self.lock();
		state.working = false;
		let reach = reach.or_else(|| {
			state.subscriptions.get(&id).map(|subscription| Arc::clone(&subscription.reach))
		});
		if let Some(reach) = reach {
			lock_mutex(&reach).extend(progress.reached);
		}
		if let Some(subscription) = state.subscriptions.get_mut(&id) {
			if let Some(ids) = &progress.discovered {
				subscription.discovered_len = Some(ids.len());
			}
			subscription.discovered = progress.discovered;
			subscription.cursor = progress.cursor;
			if !progress.published.is_empty() {
				subscription.published.extend(progress.published);
			}
			if progress.done {
				subscription.done = true;
				if subscription.discovered_len.is_none() {
					subscription.discovered_len = Some(0);
				}
			} else if progress.cost > 0.0 {
				subscription.pass += progress.cost / f64::from(subscription.class.weight());
			}
		}
		self.0.idle.notify_all();
		self.0.wake.notify_all();
	}

	/// How many times the worker has taken a quantum. Tests use this to check
	/// that a lone subscription yields once per quantum, not once per id.
	pub fn scheduler_picks(&self) -> u64 {
		self.0.picks.load(Ordering::Relaxed)
	}

	/// How many times the worker has swept. Tests use this to check that a
	/// sweep runs only when the live subscription set changes.
	pub fn sweep_count(&self) -> u64 {
		self.0.sweeps.load(Ordering::Relaxed)
	}

	/// Non-blocking pick for scheduler tests. The caller must
	/// [`Self::finish_quantum`].
	#[cfg(test)]
	pub(crate) fn try_pick(&self) -> Option<Job> {
		let mut state = self.lock();
		let job = pick_job(&mut state)?;
		state.working = true;
		Some(job)
	}

	#[cfg(test)]
	pub(crate) fn pass_of(&self, id: SubscriptionId) -> Option<f64> {
		self.lock().subscriptions.get(&id).map(|subscription| subscription.pass)
	}
}
