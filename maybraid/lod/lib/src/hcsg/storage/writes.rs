//! Publication, removal, and session clears.

use std::any::TypeId;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;

use crate::gen::{Id, Version};

use super::super::node_store::StoredEntry;
use super::eviction::forget_evictions_of;
use super::store::{read, write, ErasedStore};
use super::{HcsgStorage, HcsgValue};

impl HcsgStorage {
	/// Publishes `value` and its spatial entry together under a fresh version.
	pub fn publish<T: HcsgValue>(&self, id: Id, value: Arc<T>, bounds: Aabb3d) -> Version {
		let stamp = self.stamp();
		let version = Version(stamp);
		if let Some(store) = self.store_or_create::<T>() {
			write(&store.nodes).put(id, StoredEntry { value, bounds, version }, stamp);
		}
		version
	}

	/// [`Self::publish`], unless `stale` holds once `T`'s store is locked.
	///
	/// A session reset cancels its jobs before clearing, so a value built for
	/// the old session either lands before the clear or is dropped here.
	pub fn publish_unless<T: HcsgValue>(
		&self,
		id: Id,
		value: Arc<T>,
		bounds: Aabb3d,
		stale: impl Fn() -> bool,
	) -> Option<Version> {
		let stamp = self.stamp();
		let store = self.store_or_create::<T>()?;
		let mut nodes = write(&store.nodes);
		if stale() {
			return None;
		}
		nodes.put(id, StoredEntry { value, bounds, version: Version(stamp) }, stamp);
		Some(Version(stamp))
	}

	/// Stores a session root at `Id::Universal`.
	pub fn seed<T: HcsgValue>(&self, value: T, bounds: Aabb3d) -> Version {
		self.publish(Id::Universal, Arc::new(value), bounds)
	}

	pub fn remove<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		let stamp = self.stamp();
		let store = self.store::<T>()?;
		let removed = write(&store.nodes).remove(id, stamp).map(|entry| entry.value);
		removed
	}

	/// Drops every value of `T`.
	pub fn clear<T: HcsgValue>(&self) {
		let stamp = self.stamp();
		let store = read(&self.0.stores).get(&TypeId::of::<T>()).cloned();
		if let Some(store) = store {
			store.reset(stamp);
		}
		forget_evictions_of(self, TypeId::of::<T>());
	}

	/// Drops every value in every store, roots included, and resets eviction
	/// diagnostics. A session restart reseeds the roots afterward.
	pub fn clear_derived(&self) {
		self.clear_evictions();
		self.0.top_rebuilds.store(0, Ordering::Relaxed);
		self.0.nested_rebuilds.store(0, Ordering::Relaxed);
		let stores: Vec<Arc<dyn ErasedStore>> = read(&self.0.stores).values().cloned().collect();
		for store in stores {
			store.reset(self.stamp());
		}
	}
}
