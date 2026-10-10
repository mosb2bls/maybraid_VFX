//! Subscription registry: subscribe, read published ids, and advance epochs.

use std::any::TypeId;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;

use crate::gen::Id;

use super::super::bounds::HcsgClass;
use super::super::context::GenerationScheme;
use super::super::storage::Busy;
use super::schedule::min_pass;
use super::state::{discover, generate, lock_mutex, Subscription, SubscriptionId};
use super::HcsgDemand;

/// One read of a subscription's published ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Published {
	/// Ids published from the read's cursor on.
	pub ids: Vec<Id>,
	/// No more ids will be published: `ids` reaches the end.
	pub done: bool,
}

impl HcsgDemand {
	/// Replaces `previous` (if any) with a subscription to `T` over `regions`,
	/// under one lock, and wakes the worker.
	///
	/// A replacement inherits `previous`'s stride `pass`, floored at the
	/// current minimum among remaining live work, and its reach. A new
	/// subscription starts at that minimum, with reach `{T}`.
	pub fn subscribe<T: GenerationScheme>(
		&self,
		previous: Option<SubscriptionId>,
		regions: Vec<Aabb3d>,
		focus: Option<Vec3>,
		class: HcsgClass,
	) -> SubscriptionId {
		let id = SubscriptionId::new(self.0.next_id.fetch_add(1, Ordering::Relaxed));
		let mut state = self.lock();
		let inherited =
			previous
				.and_then(|previous| state.subscriptions.remove(&previous))
				.map(|previous| {
					previous.cancel();
					(previous.pass, Arc::clone(&previous.reach))
				});
		let floor = min_pass(&state);
		let (pass, reach) = match inherited {
			Some((pass, reach)) => (pass.max(floor), reach),
			None => (floor, Arc::new(std::sync::Mutex::new(HashSet::new()))),
		};
		lock_mutex(&reach).insert(TypeId::of::<T>());
		state.subscriptions.insert(
			id,
			Subscription {
				regions,
				focus,
				class,
				pass,
				discover: discover::<T>,
				generate: generate::<T>,
				discovered: None,
				discovered_len: None,
				cursor: 0,
				published: Vec::new(),
				done: false,
				cancelled: Arc::new(AtomicBool::new(false)),
				reach,
			},
		);
		state.sweep_needed = true;
		self.0.wake.notify_all();
		id
	}

	pub(crate) fn unsubscribe(&self, id: SubscriptionId) {
		let mut state = self.lock();
		if let Some(subscription) = state.subscriptions.remove(&id) {
			subscription.cancel();
			state.sweep_needed = true;
			self.0.wake.notify_all();
		}
		self.0.idle.notify_all();
	}

	/// Ids published for `id` from `cursor` on.
	///
	/// `Ok(None)` when the subscription no longer exists (replaced, removed,
	/// or dropped by [`Self::advance_epoch`]): subscribe again.
	#[cfg(test)]
	pub(crate) fn try_read_published(
		&self,
		id: SubscriptionId,
		cursor: usize,
	) -> Result<Option<Vec<Id>>, Busy> {
		Ok(self.try_read(id, cursor)?.map(|published| published.ids))
	}

	/// Like [`Self::try_read_published`], read together with whether the
	/// worker has finished the subscription.
	pub fn try_read(&self, id: SubscriptionId, cursor: usize) -> Result<Option<Published>, Busy> {
		let state = self.try_lock()?;
		Ok(state.subscriptions.get(&id).map(|subscription| {
			let published = &subscription.published;
			Published {
				ids: published[cursor.min(published.len())..].to_vec(),
				done: subscription.done,
			}
		}))
	}

	/// Whether `id` still exists; `false` once replaced, removed, or dropped by
	/// [`Self::advance_epoch`].
	pub fn try_is_live(&self, id: SubscriptionId) -> Result<bool, Busy> {
		Ok(self.try_lock()?.subscriptions.contains_key(&id))
	}

	/// Ends the session: cancels and drops every subscription. Values the
	/// worker is still generating are not published.
	pub(crate) fn advance_epoch(&self) -> u64 {
		let mut state = self.lock();
		for (_, subscription) in state.subscriptions.drain() {
			subscription.cancel();
		}
		state.epoch += 1;
		state.sweep_needed = true;
		self.0.wake.notify_all();
		self.0.idle.notify_all();
		state.epoch
	}
}
