//! Release micro-benchmark for column-walk exclusion handling.
//!
//! Run:
//! ```bash
//! nix develop --command cargo test -p ground-avian --release column_skip_filter_timing -- --ignored --nocapture
//! ```

use std::hint::black_box;
use std::time::Instant;

use avian3d::prelude::SpatialQueryFilter;
use bevy::ecs::entity::Entity;
use lod_avian::PhysicsInteractionLayer;

const ITERATIONS: usize = 200_000;
const MAX_COLUMN_HITS: usize = 8;

fn sample_entities(count: usize) -> Vec<Entity> {
	(0..count)
		.map(|index| Entity::from_raw_u32(index as u32 + 1).expect("test entity"))
		.collect()
}

fn fixed_filter(exclude: impl IntoIterator<Item = Entity>) -> SpatialQueryFilter {
	SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed).with_excluded_entities(exclude)
}

/// Prior `hit_down`: heap `Vec` plus a fresh filter rebuild on every column step.
fn legacy_column_filter(exclude: &[Entity], column_hits: &[Entity]) -> usize {
	let mut skipped = exclude.to_vec();
	let mut last_len = skipped.len();
	for entity in column_hits {
		skipped.push(*entity);
		let filter = fixed_filter(skipped.iter().copied());
		last_len = filter.excluded_entities.len();
	}
	last_len
}

/// Reuse one filter and insert each column hit into its exclusion set.
fn reused_column_filter(exclude: &[Entity], column_hits: &[Entity]) -> usize {
	let mut filter = fixed_filter(exclude.iter().copied());
	for entity in column_hits {
		filter.excluded_entities.insert(*entity);
	}
	filter.excluded_entities.len()
}

fn median_nanos(samples: &mut [u128]) -> f64 {
	samples.sort_unstable();
	let mid = samples.len() / 2;
	if samples.len().is_multiple_of(2) {
		(samples[mid - 1] + samples[mid]) as f64 / 2.0
	} else {
		samples[mid] as f64
	}
}

fn bench<F>(mut f: F) -> f64
where
	F: FnMut(),
{
	let mut samples = [0u128; 5];
	for sample in &mut samples {
		let start = Instant::now();
		for _ in 0..ITERATIONS {
			f();
		}
		*sample = start.elapsed().as_nanos();
	}
	median_nanos(&mut samples) / ITERATIONS as f64
}

#[test]
#[ignore = "release micro-benchmark; run with --release --ignored --nocapture"]
fn column_skip_filter_timing() {
	let exclude = sample_entities(4);
	let column_hits = sample_entities(3);

	let legacy_ns = bench(|| {
		let len = legacy_column_filter(black_box(&exclude), black_box(&column_hits));
		black_box(len);
	});
	let reused_ns = bench(|| {
		let len = reused_column_filter(black_box(&exclude), black_box(&column_hits));
		black_box(len);
	});

	eprintln!(
		"column skip filter ({} iterations, 4 exclude + {} hits): legacy {:.1} ns/iter, reused {:.1} ns/iter",
		ITERATIONS,
		MAX_COLUMN_HITS.min(column_hits.len()),
		legacy_ns,
		reused_ns
	);
}
