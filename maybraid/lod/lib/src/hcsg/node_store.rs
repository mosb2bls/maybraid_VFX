//! Typed HCSG storage: one homogeneous [`NodeStore`] per generated type.

use std::collections::{HashMap, HashSet};

use bevy::math::bounding::{Aabb3d, BoundingVolume, IntersectsVolume};
use bevy::math::{DVec3, Vec3};

use crate::gen::{Id, Version};

/// Base cell size of a store's spatial index when none is configured.
pub const DEFAULT_BASE_SCALE: DVec3 = DVec3::splat(64.0);

/// One stored value with its authoritative bounds and storage version.
#[derive(Debug, Clone)]
pub struct StoredEntry<T> {
	pub value: T,
	pub bounds: Aabb3d,
	pub version: Version,
}

/// Values of one type, their bounds, and a spatial index over those bounds.
///
/// Every mutation goes through [`HcsgStorage`], which keeps `entries`, the
/// spatial index, and the membership revision in step. `Id::Universal`
/// entries are reachable by id only; they are never spatially indexed.
pub struct NodeStore<T> {
	entries: HashMap<Id, StoredEntry<T>>,
	/// `None` when gimme rejects the base scale; queries then walk `entries`.
	spatial: Option<gimme_core::SpatialIndex<Id>>,
	/// Ids gimme would not index (non-finite bounds). Every query walks them.
	unindexed: HashSet<Id>,
	base_scale: DVec3,
	membership_revision: u64,
}

impl<T> NodeStore<T> {
	pub(super) fn new(base_scale: DVec3) -> Self {
		Self {
			entries: HashMap::new(),
			spatial: gimme_core::SpatialIndex::new(base_scale).ok(),
			unindexed: HashSet::new(),
			base_scale,
			membership_revision: 0,
		}
	}

	pub(super) fn rebuild_index(&mut self, base_scale: DVec3) {
		self.base_scale = base_scale;
		self.spatial = gimme_core::SpatialIndex::new(base_scale).ok();
		self.unindexed.clear();
		let entries: Vec<(Id, Aabb3d)> =
			self.entries.iter().map(|(id, entry)| (*id, entry.bounds)).collect();
		for (id, bounds) in entries {
			self.index(id, bounds);
		}
	}

	fn index(&mut self, id: Id, bounds: Aabb3d) {
		if id == Id::Universal {
			return;
		}
		let indexed =
			self.spatial.as_mut().is_some_and(|spatial| spatial.insert(id, bounds).is_ok());
		if indexed {
			self.unindexed.remove(&id);
		} else {
			if let Some(spatial) = self.spatial.as_mut() {
				spatial.remove(id);
			}
			self.unindexed.insert(id);
		}
	}

	pub fn entry(&self, id: Id) -> Option<&StoredEntry<T>> {
		self.entries.get(&id)
	}

	pub fn value(&self, id: Id) -> Option<&T> {
		self.entries.get(&id).map(|entry| &entry.value)
	}

	pub(crate) fn contains(&self, id: Id) -> bool {
		self.entries.contains_key(&id)
	}

	pub fn len(&self) -> usize {
		self.entries.len()
	}

	/// Monotonic stamp bumped by every insert, removal, and clear.
	pub fn membership_revision(&self) -> u64 {
		self.membership_revision
	}

	/// Ids whose stored bounds intersect `region`, in id order.
	pub fn overlapping(&self, region: Aabb3d) -> Vec<Id> {
		let mut ids: Vec<Id> = match &self.spatial {
			Some(spatial) if !self.region_is_sparse(region) => {
				let ids = spatial.query(region);
				if self.unindexed.is_empty() {
					ids
				} else {
					let mut ids = ids;
					ids.extend(self.unindexed.iter().copied().filter(|id| {
						self.entries.get(id).is_some_and(|entry| entry.bounds.intersects(&region))
					}));
					ids.sort();
					ids
				}
			}
			_ => {
				let mut ids = self
					.entries
					.keys()
					.copied()
					.filter(|id| *id != Id::Universal)
					.filter(|id| {
						self.entries.get(id).is_some_and(|entry| entry.bounds.intersects(&region))
					})
					.collect::<Vec<_>>();
				ids.sort();
				ids
			}
		};
		if self
			.entries
			.get(&Id::Universal)
			.is_some_and(|entry| entry.bounds.intersects(&region))
		{
			ids.push(Id::Universal);
		}
		ids
	}

	/// Whether walking the store beats enumerating `region`'s finest buckets.
	fn region_is_sparse(&self, region: Aabb3d) -> bool {
		let scale = self.base_scale;
		let extent = Vec3::from(region.max - region.min).as_dvec3().max(DVec3::ZERO);
		let buckets = (extent / scale).ceil().max(DVec3::ONE);
		buckets.x * buckets.y * buckets.z > 4.0 * self.entries.len().max(1) as f64
	}

	pub(super) fn put(&mut self, id: Id, entry: StoredEntry<T>, revision: u64) {
		self.index(id, entry.bounds);
		self.entries.insert(id, entry);
		self.membership_revision = revision;
	}

	pub(super) fn remove(&mut self, id: Id, revision: u64) -> Option<StoredEntry<T>> {
		let entry = self.entries.remove(&id)?;
		if let Some(spatial) = self.spatial.as_mut() {
			spatial.remove(id);
		}
		self.unindexed.remove(&id);
		self.membership_revision = revision;
		Some(entry)
	}

	/// Ids that would be dropped by [`Self::retain_overlapping`], without mutating.
	pub(super) fn ids_outside(&self, regions: &[Aabb3d]) -> Vec<Id> {
		let keep = KeepRegions::new(regions);
		self.entries
			.iter()
			.filter(|(id, entry)| **id != Id::Universal && !keep.overlaps(entry.bounds))
			.map(|(id, _)| *id)
			.collect()
	}

	/// Removes entries outside `regions` and bumps the membership revision.
	pub(super) fn evict_outside(&mut self, regions: &[Aabb3d], revision: u64) -> Vec<Id> {
		if regions.is_empty() {
			return self.clear_non_universal(revision);
		}
		let keep = KeepRegions::new(regions);
		let mut removed = Vec::new();
		let before = self.entries.len();
		self.entries.retain(|id, entry| {
			let retain = *id == Id::Universal || keep.overlaps(entry.bounds);
			if !retain {
				removed.push(*id);
			}
			retain
		});
		if removed.is_empty() {
			return removed;
		}
		let bulk = self.spatial.is_some() && removed.len() * 2 > before;
		if bulk {
			self.rebuild_index(self.base_scale);
		} else {
			if let Some(spatial) = self.spatial.as_mut() {
				for id in &removed {
					spatial.remove(*id);
				}
			}
			for id in &removed {
				self.unindexed.remove(id);
			}
		}
		self.membership_revision = revision;
		removed
	}

	/// Removes `ids` and bumps the membership revision. No-op when `ids` is empty.
	pub(super) fn remove_ids(&mut self, ids: &[Id], revision: u64) {
		if ids.is_empty() {
			return;
		}
		// Dropping most of the store is cheaper as one index rebuild than thousands
		// of per-id spatial removes (worker retention sweeps).
		let bulk = self.spatial.is_some() && ids.len() * 2 > self.entries.len();
		if bulk {
			for id in ids {
				self.entries.remove(id);
			}
			self.rebuild_index(self.base_scale);
		} else {
			for id in ids {
				self.entries.remove(id);
				if let Some(spatial) = self.spatial.as_mut() {
					spatial.remove(*id);
				}
				self.unindexed.remove(id);
			}
		}
		self.membership_revision = revision;
	}

	/// Keeps entries that overlap any of `regions`, and every `Id::Universal`
	/// entry regardless of bounds. Returns the ids that were removed.
	#[cfg(test)]
	pub(super) fn retain_overlapping(&mut self, regions: &[Aabb3d], revision: u64) -> Vec<Id> {
		let removed = self.ids_outside(regions);
		self.remove_ids(&removed, revision);
		removed
	}

	/// Drops every entry, keeping the base scale.
	pub(super) fn reset(&mut self, revision: u64) {
		*self = Self::new(self.base_scale);
		self.membership_revision = revision;
	}

	/// Worker sweep when no live subscription reaches this type: drop every
	/// entry except [`Id::Universal`] without rebuilding the spatial index from
	/// thousands of per-id removes. Under release builds, does not list removed
	/// ids (see [`ErasedStore::clear_non_universal`]).
	pub(super) fn clear_non_universal(&mut self, revision: u64) -> Vec<Id> {
		#[cfg(debug_assertions)]
		let removed: Vec<Id> =
			self.entries.keys().filter(|id| **id != Id::Universal).copied().collect();
		#[cfg(debug_assertions)]
		if removed.is_empty() {
			return removed;
		}
		#[cfg(not(debug_assertions))]
		{
			let len = self.entries.len();
			if len == 0 || (len == 1 && self.entries.contains_key(&Id::Universal)) {
				return Vec::new();
			}
		}
		let universal = self.entries.remove(&Id::Universal);
		let base_scale = self.base_scale;
		*self = Self::new(base_scale);
		self.membership_revision = revision;
		if let Some(entry) = universal {
			self.index(Id::Universal, entry.bounds);
			self.entries.insert(Id::Universal, entry);
		}
		#[cfg(debug_assertions)]
		{
			return removed;
		}
		#[cfg(not(debug_assertions))]
		Vec::new()
	}
}

fn regions_envelope(regions: &[Aabb3d]) -> Aabb3d {
	let mut envelope = regions[0];
	for region in regions.iter().skip(1) {
		envelope = envelope.merge(region);
	}
	envelope
}

/// Precomputes the bounding envelope for multi-region retention checks.
struct KeepRegions<'a> {
	regions: &'a [Aabb3d],
	envelope: Option<Aabb3d>,
}

impl<'a> KeepRegions<'a> {
	fn new(regions: &'a [Aabb3d]) -> Self {
		let envelope = if regions.len() > 1 { Some(regions_envelope(regions)) } else { None };
		Self { regions, envelope }
	}

	fn overlaps(&self, bounds: Aabb3d) -> bool {
		match self.regions {
			[] => false,
			[region] => bounds.intersects(region),
			_ => {
				let envelope = self.envelope.expect("envelope set for multiple regions");
				bounds.intersects(&envelope)
					&& self.regions.iter().any(|region| bounds.intersects(region))
			}
		}
	}
}

/// One bucket of hysteresis around a live subscription region.
pub(super) fn expand_region(region: Aabb3d, scale: DVec3) -> Aabb3d {
	let pad = Vec3::new(scale.x as f32, scale.y as f32, scale.z as f32);
	Aabb3d::from_min_max(Vec3::from(region.min) - pad, Vec3::from(region.max) + pad)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::gen::tests::test_utils::cell;
	use crate::gen::{Id, Version};

	fn store_with(entries: &[(Id, Aabb3d)]) -> NodeStore<u32> {
		let mut store = NodeStore::new(DVec3::ONE);
		for (i, &(id, bounds)) in entries.iter().enumerate() {
			store.put(
				id,
				StoredEntry { value: i as u32, bounds, version: Version(i as u64 + 1) },
				i as u64 + 1,
			);
		}
		store
	}

	fn span(x: f32, width: f32) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
	}

	#[test]
	fn retain_overlapping_removes_entries_outside_every_region() {
		let id0 = Id::from_cell(cell(0.0));
		let id2 = Id::from_cell(cell(2.0));
		let mut store = store_with(&[(id0, cell(0.0)), (id2, cell(2.0))]);
		let removed = store.retain_overlapping(&[span(1.5, 1.0)], 9);
		assert_eq!(removed, vec![id0]);
		assert!(!store.contains(id0));
		assert!(store.contains(id2));
		assert_eq!(store.membership_revision(), 9);
	}

	#[test]
	fn retain_overlapping_keeps_entries_that_touch_a_region() {
		let id = Id::from_cell(cell(1.0));
		let mut store = store_with(&[(id, cell(1.0))]);
		let revision = store.membership_revision();
		let removed = store.retain_overlapping(&[span(1.2, 0.3)], 10);
		assert!(removed.is_empty());
		assert!(store.contains(id));
		assert_eq!(store.membership_revision(), revision);
	}

	#[test]
	fn retain_overlapping_keeps_universal_whatever_its_bounds() {
		let id = Id::from_cell(cell(0.0));
		let mut store = store_with(&[(Id::Universal, span(-1_000.0, 2_000.0)), (id, cell(0.0))]);
		let removed = store.retain_overlapping(&[span(50.0, 1.0)], 11);
		assert_eq!(removed, vec![id]);
		assert!(store.contains(Id::Universal));
		assert_eq!(store.membership_revision(), 11);
	}
}
