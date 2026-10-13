//! Blocking reads for the worker and non-blocking reads for the frame.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;

use crate::gen::Id;

use super::super::node_store::StoredEntry;
use super::store::{read, try_read};
use super::{Busy, HcsgStorage, HcsgValue};

impl HcsgStorage {
	fn blocking_get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		let store = self.store::<T>()?;
		let nodes = read(&store.nodes);
		nodes.value(id).cloned()
	}

	fn blocking_entry<T: HcsgValue>(&self, id: Id) -> Option<StoredEntry<Arc<T>>> {
		let store = self.store::<T>()?;
		let nodes = read(&store.nodes);
		nodes.entry(id).cloned()
	}

	/// Stored ids of `T` whose bounds intersect `region`, in id order.
	fn blocking_overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id> {
		self.store::<T>()
			.map(|store| read(&store.nodes).overlapping(region))
			.unwrap_or_default()
	}

	#[cfg(not(any(test, feature = "test-support")))]
	pub(crate) fn get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		self.blocking_get::<T>(id)
	}

	#[cfg(any(test, feature = "test-support"))]
	pub fn get<T: HcsgValue>(&self, id: Id) -> Option<Arc<T>> {
		self.blocking_get::<T>(id)
	}

	#[cfg(not(any(test, feature = "test-support")))]
	pub(crate) fn entry<T: HcsgValue>(&self, id: Id) -> Option<StoredEntry<Arc<T>>> {
		self.blocking_entry::<T>(id)
	}

	#[cfg(any(test, feature = "test-support"))]
	pub fn entry<T: HcsgValue>(&self, id: Id) -> Option<StoredEntry<Arc<T>>> {
		self.blocking_entry::<T>(id)
	}

	#[cfg(any(test, feature = "test-support"))]
	pub fn contains<T: HcsgValue>(&self, id: Id) -> bool {
		self.store::<T>().is_some_and(|store| read(&store.nodes).contains(id))
	}

	#[cfg(not(any(test, feature = "test-support")))]
	pub(crate) fn overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id> {
		self.blocking_overlapping::<T>(region)
	}

	#[cfg(any(test, feature = "test-support"))]
	pub fn overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Vec<Id> {
		self.blocking_overlapping::<T>(region)
	}

	/// Bumped by every publish, removal and clear of `T`.
	#[cfg(any(test, feature = "test-support"))]
	pub fn membership_revision<T: HcsgValue>(&self) -> u64 {
		self.store::<T>().map_or(0, |store| read(&store.nodes).membership_revision())
	}

	/// [`Self::entry`] for the frame: never waits on a lock.
	pub fn try_entry<T: HcsgValue>(&self, id: Id) -> Result<Option<StoredEntry<Arc<T>>>, Busy> {
		let Some(store) = self.try_store::<T>()? else {
			return Ok(None);
		};
		let entry = try_read(&store.nodes)?.entry(id).cloned();
		Ok(entry)
	}

	/// [`Self::overlapping`] for the frame: never waits on a lock.
	pub fn try_overlapping<T: HcsgValue>(&self, region: Aabb3d) -> Result<Vec<Id>, Busy> {
		let Some(store) = self.try_store::<T>()? else {
			return Ok(Vec::new());
		};
		let ids = try_read(&store.nodes)?.overlapping(region);
		Ok(ids)
	}

	/// [`Self::membership_revision`] for the frame: never waits on a lock.
	pub fn try_membership_revision<T: HcsgValue>(&self) -> Result<u64, Busy> {
		let Some(store) = self.try_store::<T>()? else {
			return Ok(0);
		};
		let revision = try_read(&store.nodes)?.membership_revision();
		Ok(revision)
	}
}
