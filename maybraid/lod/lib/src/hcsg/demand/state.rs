//! Shared demand state: subscriptions and worker synchronization.

use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

use crate::gen::{Id, OriginalId};

use super::super::bounds::HcsgClass;
use super::super::context::{GenerationContext, GenerationScheme};

/// Handle a generation or presentation system keeps for its one subscription.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SubscriptionId(u64);

impl SubscriptionId {
	pub(super) fn new(raw: u64) -> Self {
		Self(raw)
	}
}

pub(super) type Discover = fn(&mut GenerationContext, Aabb3d) -> Vec<Id>;
pub(super) type Generate = fn(&mut GenerationContext, Id) -> bool;

pub(super) fn discover<T: GenerationScheme>(cx: &mut GenerationContext, region: Aabb3d) -> Vec<Id> {
	T::original_ids_for(cx, region).into_iter().map(|OriginalId(id)| id).collect()
}

pub(super) fn generate<T: GenerationScheme>(cx: &mut GenerationContext, id: Id) -> bool {
	cx.get_or_generate::<T>(id).is_some()
}

pub(super) struct Subscription {
	pub regions: Vec<Aabb3d>,
	pub focus: Option<Vec3>,
	pub class: HcsgClass,
	pub pass: f64,
	pub discover: Discover,
	pub generate: Generate,
	/// Deduped, focus-sorted ids, once discovery has run.
	pub discovered: Option<Vec<Id>>,
	pub discovered_len: Option<usize>,
	/// Next index in [`Self::discovered`] to generate.
	pub cursor: usize,
	/// Every discovered id whose value is available, in the order it landed.
	pub published: Vec<Id>,
	pub done: bool,
	/// Set when the subscription is replaced, removed, or its epoch ends.
	pub cancelled: Arc<AtomicBool>,
	/// This scheme's type plus every type its fills have read. Shared with
	/// replacements so a mid-quantum finish still lands on the live chain.
	pub reach: Arc<Mutex<HashSet<TypeId>>>,
}

impl Subscription {
	pub(super) fn cancel(&self) {
		self.cancelled.store(true, Ordering::Release);
	}
}

#[derive(Default)]
pub(super) struct DemandState {
	pub subscriptions: HashMap<SubscriptionId, Subscription>,
	pub epoch: u64,
	pub working: bool,
	pub shutdown: bool,
	/// The live subscription set changed since the last sweep.
	pub sweep_needed: bool,
}

pub(super) struct DemandShared {
	pub next_id: AtomicU64,
	pub picks: AtomicU64,
	pub sweeps: AtomicU64,
	pub state: Mutex<DemandState>,
	/// Notified when there is work, or on shutdown.
	pub wake: Condvar,
	/// Notified when the worker finishes a quantum or runs out of work.
	pub idle: Condvar,
}

impl Default for DemandShared {
	fn default() -> Self {
		Self {
			next_id: AtomicU64::new(0),
			picks: AtomicU64::new(0),
			sweeps: AtomicU64::new(0),
			state: Mutex::new(DemandState::default()),
			wake: Condvar::new(),
			idle: Condvar::new(),
		}
	}
}

pub(super) fn lock_mutex<T>(lock: &Mutex<T>) -> MutexGuard<'_, T> {
	lock.lock().unwrap_or_else(PoisonError::into_inner)
}
