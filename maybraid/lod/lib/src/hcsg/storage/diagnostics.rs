//! Store-size and rebuild diagnostics (`MAYBRAID_HCSG_DIAG` reads these).

use std::sync::atomic::Ordering;

use super::store::read;
use super::HcsgStorage;

impl HcsgStorage {
	/// `(type name, len)` for every store, sorted by name. Diagnostics.
	pub fn store_sizes(&self) -> Vec<(&'static str, usize)> {
		let mut sizes: Vec<_> = read(&self.0.stores)
			.values()
			.map(|store| (store.type_name(), store.len()))
			.collect();
		sizes.sort_by_key(|(name, _)| *name);
		sizes
	}

	/// Top-level and nested rebuilds of values this store previously evicted.
	pub fn rebuilds_after_eviction(&self) -> (u64, u64) {
		(
			self.0.top_rebuilds.load(Ordering::Relaxed),
			self.0.nested_rebuilds.load(Ordering::Relaxed),
		)
	}
}
