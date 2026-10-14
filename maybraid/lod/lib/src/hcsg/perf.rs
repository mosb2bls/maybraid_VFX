//! Release-only timing harnesses for HCSG core hot paths. Run with:
//!
//! ```bash
//! cd maybraid/lod/lib && CXX=g++ cargo test -p lod hcsg::perf --release -- --ignored --nocapture
//! ```

use std::time::Instant;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};

use std::sync::Arc;

use crate::gen::tests::test_utils::cell;
use crate::gen::Id;

use super::node_store::{NodeStore, StoredEntry};
use super::storage::HcsgStorage;
use crate::gen::Version;

fn span(x: f32, width: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
}

fn fill_store(entries: usize) -> NodeStore<u32> {
	let mut store = NodeStore::new(DVec3::splat(64.0));
	for i in 0..entries {
		let id = Id::from_cell(cell(i as f32 * 64.0));
		store.put(
			id,
			StoredEntry { value: i as u32, bounds: cell(i as f32 * 64.0), version: Version(1) },
			i as u64 + 1,
		);
	}
	store
}

/// Sweeps that drop most of a large typed store (worker retention path).
#[test]
#[ignore]
fn perf_eviction_sweep_large_store() {
	const ENTRIES: usize = 20_000;
	const ROUNDS: usize = 30;
	let keep = vec![span(64.0 * 800.0, 64.0 * 32.0)];

	let started = Instant::now();
	for round in 0..ROUNDS {
		let mut store = fill_store(ENTRIES);
		let removed = store.evict_outside(&keep, round as u64 + 1);
		assert!(removed.len() > ENTRIES * 9 / 10);
	}
	let elapsed = started.elapsed();
	eprintln!(
		"perf_eviction_sweep_large_store: {} rounds × {} entries → {:?} ({:.1?} per sweep)",
		ROUNDS,
		ENTRIES,
		elapsed,
		elapsed / ROUNDS as u32
	);
}

/// Eviction only on a full store (refill removed ids each round). Matches the
/// worker sweep hot path more closely than rebuilding the store every round.
#[test]
#[ignore]
fn perf_eviction_sweep_evict_only() {
	const ENTRIES: usize = 20_000;
	const ROUNDS: usize = 100;
	let keep = vec![span(64.0 * 800.0, 64.0 * 32.0)];
	let mut store = fill_store(ENTRIES);

	let started = Instant::now();
	for round in 0..ROUNDS {
		let removed = store.evict_outside(&keep, round as u64 + 1);
		assert!(removed.len() > ENTRIES * 9 / 10);
		for id in removed {
			let bounds = id.origin_cell_bounds().unwrap_or(cell(0.0));
			store.put(id, StoredEntry { value: 0, bounds, version: Version(1) }, round as u64 + 2);
		}
	}
	let elapsed = started.elapsed();
	eprintln!(
		"perf_eviction_sweep_evict_only: {} rounds × {} entries → {:?} ({:.2?} per evict+refill)",
		ROUNDS,
		ENTRIES,
		elapsed,
		elapsed / ROUNDS as u32
	);
}

/// Measures [`NodeStore::evict_outside`] alone on a dense store with a small
/// retention window (typical worker sweep after a camera move).
#[test]
#[ignore]
fn perf_eviction_sweep_evict_alone() {
	const ENTRIES: usize = 20_000;
	const ROUNDS: usize = 200;
	let keep = vec![span(64.0 * 800.0, 64.0 * 32.0)];

	let mut evict_total = std::time::Duration::ZERO;
	for round in 0..ROUNDS {
		let mut store = fill_store(ENTRIES);
		let started = Instant::now();
		let removed = store.evict_outside(&keep, round as u64 + 1);
		evict_total += started.elapsed();
		assert!(removed.len() > ENTRIES * 9 / 10);
	}
	eprintln!(
		"perf_eviction_sweep_evict_alone: {} rounds × {} entries → {:?} ({:.2?} per evict)",
		ROUNDS,
		ENTRIES,
		evict_total,
		evict_total / ROUNDS as u32
	);
}

/// Spatial overlapping queries over a dense store (generation discovery helpers).
#[test]
#[ignore]
fn perf_overlapping_queries() {
	const ENTRIES: usize = 20_000;
	const QUERIES: usize = 2_000;
	let store = fill_store(ENTRIES);
	let query = span(64.0 * 400.0, 64.0 * 8.0);

	let started = Instant::now();
	let mut hits = 0usize;
	for _ in 0..QUERIES {
		hits += store.overlapping(query).len();
	}
	let elapsed = started.elapsed();
	assert!(hits > 0);
	eprintln!(
		"perf_overlapping_queries: {} queries on {} entries → {:?} ({:.2?} per query)",
		QUERIES,
		ENTRIES,
		elapsed,
		elapsed / QUERIES as u32
	);
}

/// Presentation spawns one host per published id; each [`HcsgStorage::try_entry`]
/// re-resolves the typed store through the registry lock.
#[test]
#[ignore]
fn perf_presentation_try_entries() {
	const STORE_ENTRIES: usize = 8_000;
	const BATCH: usize = 32;
	const ROUNDS: usize = 50_000;

	let storage = HcsgStorage::default();
	let mut batch_ids = Vec::with_capacity(BATCH);
	for i in 0..STORE_ENTRIES {
		let id = Id::from_cell(cell(i as f32 * 64.0));
		if i < BATCH {
			batch_ids.push(id);
		}
		storage.publish(id, Arc::new(i as u32), cell(i as f32 * 64.0));
	}

	let per_entry_started = Instant::now();
	for _ in 0..ROUNDS {
		for id in &batch_ids {
			let _ = storage.try_entry::<u32>(*id).unwrap();
		}
	}
	let per_entry = per_entry_started.elapsed();

	let batch_started = Instant::now();
	for _ in 0..ROUNDS {
		let _ = storage.try_entries::<u32>(&batch_ids).unwrap();
	}
	let batch = batch_started.elapsed();

	let per_entry_calls = ROUNDS * BATCH;
	eprintln!(
		"perf_presentation_try_entries: {} rounds × {} ids ({per_entry_calls} lookups)",
		ROUNDS,
		BATCH,
	);
	eprintln!(
		"  try_entry loop: {:?} ({:.1?} per lookup)",
		per_entry,
		per_entry / per_entry_calls as u32
	);
	eprintln!(
		"  try_entries batch: {:?} ({:.1?} per lookup)",
		batch,
		batch / per_entry_calls as u32
	);
}
