//! Eviction sweeps and debug rebuild tracking.

use std::any::TypeId;
use std::collections::HashMap;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;

use crate::gen::Id;

use super::super::node_store::expand_region;
#[cfg(debug_assertions)]
use super::store::lock_mutex;
use super::store::{read, ErasedStore};
use super::{HcsgStorage, HcsgValue};

pub(super) fn record_evictions(storage: &HcsgStorage, removed: Vec<(TypeId, Id)>) {
	if removed.is_empty() {
		return;
	}
	#[cfg(debug_assertions)]
	lock_mutex(&storage.0.evicted).extend(removed);
	#[cfg(not(debug_assertions))]
	let _ = (storage, removed);
}

pub(super) fn forget_evictions_of(storage: &HcsgStorage, type_id: TypeId) {
	#[cfg(debug_assertions)]
	lock_mutex(&storage.0.evicted).retain(|(stored, _)| *stored != type_id);
	#[cfg(not(debug_assertions))]
	let _ = (storage, type_id);
}

impl HcsgStorage {
	/// Removes every `T` whose bounds overlap none of `regions`. `Id::Universal`
	/// is kept. Removal bumps the membership revision as [`Self::remove`] does.
	pub fn retain_overlapping<T: HcsgValue>(&self, regions: &[Aabb3d]) {
		let Some(store) = self.store::<T>() else {
			return;
		};
		let stamp = self.stamp();
		let removed = store.evict_outside(regions, stamp);
		if removed.is_empty() {
			return;
		}
		record_evictions(self, removed.into_iter().map(|id| (TypeId::of::<T>(), id)).collect());
	}

	/// Worker sweep: each stored type is kept in the union of `regions_by_type`
	/// for that type, expanded by the type's retention margin. A type no live
	/// subscription reaches is cleared, except `Id::Universal` entries.
	pub(in crate::hcsg) fn retain_reached(&self, regions_by_type: &HashMap<TypeId, Vec<Aabb3d>>) {
		if regions_by_type.is_empty() {
			self.clear_evictions();
		}
		let stores: Vec<(TypeId, Arc<dyn ErasedStore>)> = read(&self.0.stores)
			.iter()
			.map(|(type_id, store)| (*type_id, store.clone()))
			.collect();
		for (type_id, store) in stores {
			let margin = self.retention_margin_of(type_id);
			let expanded: Vec<Aabb3d> = regions_by_type
				.get(&type_id)
				.into_iter()
				.flatten()
				.map(|region| expand_region(*region, margin))
				.collect();
			let stamp = self.stamp();
			let removed = store.evict_outside(&expanded, stamp);
			if removed.is_empty() {
				continue;
			}
			record_evictions(self, removed.into_iter().map(|id| (type_id, id)).collect());
		}
	}

	/// True if this key was swept; removes it so the set cannot grow forever.
	pub(in crate::hcsg) fn take_evicted(&self, type_id: TypeId, id: Id) -> bool {
		#[cfg(debug_assertions)]
		{
			lock_mutex(&self.0.evicted).remove(&(type_id, id))
		}
		#[cfg(not(debug_assertions))]
		{
			let _ = (type_id, id);
			false
		}
	}

	pub(in crate::hcsg) fn record_rebuild(&self, nested: bool) {
		if nested {
			self.0.nested_rebuilds.fetch_add(1, Ordering::Relaxed);
		} else {
			self.0.top_rebuilds.fetch_add(1, Ordering::Relaxed);
		}
	}

	/// Drops every recorded eviction. Called when no live subscription remains
	/// (epoch or last unsubscribe) so a new session does not count as rebuilds.
	pub fn clear_evictions(&self) {
		#[cfg(debug_assertions)]
		lock_mutex(&self.0.evicted).clear();
	}
}
