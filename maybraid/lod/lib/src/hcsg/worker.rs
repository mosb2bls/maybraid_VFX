//! [`HcsgWorker`]: the thread that fills [`HcsgDemand`] subscriptions.

use std::panic::{self, AssertUnwindSafe};
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Instant;

use bevy::log::error;
use bevy::math::Vec3A;
use bevy::prelude::Resource;

use crate::gen::Id;

use super::context::GenerationContext;
use super::demand::{
	quantum_cost, HcsgDemand, Job, QuantumProgress, WorkerWait, QUANTUM_IDS, QUANTUM_TIME,
};
use super::storage::HcsgStorage;

/// Fills unfinished subscriptions in weighted quanta: discovers each
/// subscription's ids once, nearest the focus first, then generates until
/// the quantum's id or time budget is spent. Resume keeps the discovered
/// list and cursor.
///
/// Between jobs, after the live subscription set changes, the worker sweeps
/// stored values by reach. Dropping the worker shuts the thread down and
/// joins it.
#[derive(Resource)]
pub(crate) struct HcsgWorker {
	demand: HcsgDemand,
	thread: Option<JoinHandle<()>>,
}

impl HcsgWorker {
	pub(crate) fn spawn(storage: HcsgStorage, demand: HcsgDemand) -> std::io::Result<Self> {
		let thread = thread::Builder::new().name("hcsg-worker".into()).spawn({
			let demand = demand.clone();
			move || run(&storage, &demand)
		})?;
		Ok(Self { demand, thread: Some(thread) })
	}
}

impl Drop for HcsgWorker {
	fn drop(&mut self) {
		self.demand.shutdown();
		if let Some(thread) = self.thread.take() {
			let _ = thread.join();
		}
	}
}

fn run(storage: &HcsgStorage, demand: &HcsgDemand) {
	loop {
		match demand.next_work() {
			WorkerWait::Shutdown => return,
			WorkerWait::Sweep(regions_by_type) => storage.retain_reached(&regions_by_type),
			WorkerWait::Job(job) => {
				let id = job.id;
				let reach = job.reach.clone();
				let progress =
					panic::catch_unwind(AssertUnwindSafe(|| fill_quantum(storage, job)))
						.unwrap_or_else(|_| {
							error!(
								"hcsg worker: generation panicked; subscription {id:?} left partial"
							);
							QuantumProgress { done: true, ..QuantumProgress::default() }
						});
				demand.finish_quantum(id, progress, Some(reach));
			}
		}
	}
}

fn fill_quantum(storage: &HcsgStorage, job: Job) -> QuantumProgress {
	let stale = || job.cancelled.load(Ordering::Acquire);
	let started = Instant::now();
	let mut cx = GenerationContext::with_stale(storage, &stale);
	let first_discover = job.discovered.is_none();
	let ids = match job.discovered {
		Some(ids) => ids,
		None => {
			let mut ids: Vec<Id> =
				job.regions.iter().flat_map(|region| (job.discover)(&mut cx, *region)).collect();
			ids.sort();
			ids.dedup();
			if let Some(focus) = job.focus {
				let focus = Vec3A::from(focus);
				let distance = |id: &Id| {
					id.origin_cell_bounds().map_or(f32::INFINITY, |cell| {
						((cell.min + cell.max) * 0.5).distance_squared(focus)
					})
				};
				ids.sort_by(|a, b| distance(a).total_cmp(&distance(b)));
			}
			ids
		}
	};
	if stale() {
		return progress(
			&mut cx,
			Some(ids),
			job.cursor,
			quantum_cost(1, started.elapsed()),
			false,
			Vec::new(),
		);
	}
	if first_discover && started.elapsed() >= QUANTUM_TIME {
		let done = ids.is_empty();
		return progress(
			&mut cx,
			Some(ids),
			job.cursor,
			quantum_cost(1, started.elapsed()),
			done,
			Vec::new(),
		);
	}

	let mut cursor = job.cursor;
	let mut cost = 0u32;
	let mut published = Vec::new();
	while cursor < ids.len() {
		if stale() {
			break;
		}
		if cost > 0 && ((cost as usize) >= QUANTUM_IDS || started.elapsed() >= QUANTUM_TIME) {
			break;
		}
		if (job.generate)(&mut cx, ids[cursor]) {
			published.push(ids[cursor]);
		}
		cursor += 1;
		cost += 1;
	}
	let done = cursor >= ids.len() && !stale();
	progress(
		&mut cx,
		Some(ids),
		cursor,
		quantum_cost(cost.max(1), started.elapsed()),
		done,
		published,
	)
}

fn progress(
	cx: &mut GenerationContext,
	discovered: Option<Vec<Id>>,
	cursor: usize,
	cost: f64,
	done: bool,
	published: Vec<Id>,
) -> QuantumProgress {
	QuantumProgress { discovered, cursor, cost, done, reached: cx.take_reached(), published }
}
