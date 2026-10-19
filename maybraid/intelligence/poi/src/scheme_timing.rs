//! Release timing for distant xz-tile POI lookup.
//!
//! Catalog entries are the [`GridTiles`] pins for an `n ×` one-origin-per-tile
//! layout, so the old global-list scan, session-root [`Catalog`], and
//! [`GridTiles`] resolve the same probes. Discovery still differs: scan and
//! [`Catalog`] walk the catalog (`O(n)`), [`GridTiles`] enumerates tiles in the
//! query (`O(tiles)`).
//!
//! ```bash
//! CXX=g++ cargo test -p poi-intelligence xz_tile_scheme_timing --release -- --ignored --nocapture
//! ```

use std::collections::HashSet;
use std::hint::black_box;
use std::time::{Duration, Instant};

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};

use crate::{
	collect_matching_in_xz_tile, xz_tile, xz_tile_region, Catalog, GridTiles, PoiCatalog, PoiEntry,
	PoiId, PoiInterests, PoiOrigin, PoiSet, GRID_KIND,
};

const TILE_SIZE: f32 = 256.0;
const POI_COUNTS: [usize; 4] = [128, 512, 2048, 8192];
const TILE_SPANS: [i32; 3] = [1, 2, 4];
const SPAN_SWEEP_N: usize = 2048;
const PROBES: usize = 64;
const WARMUP: usize = 8;
const WARM_ITERS: usize = 32;
const COLD_ITERS: usize = 4;

const RERUN: &str =
	"CXX=g++ cargo test -p poi-intelligence xz_tile_scheme_timing --release -- --ignored --nocapture";

#[derive(Clone, Copy, Debug)]
struct Query {
	origin: IVec2,
	span: i32,
}

impl Query {
	fn tiles(self) -> i32 {
		self.span * self.span
	}
}

struct Stats {
	median: Duration,
	p95: Duration,
	total: Duration,
}

fn percentile(sorted: &[Duration], p: f64) -> Duration {
	if sorted.is_empty() {
		return Duration::ZERO;
	}
	let idx = ((sorted.len() as f64 - 1.0) * p).round() as usize;
	sorted[idx.min(sorted.len() - 1)]
}

fn stats(mut samples: Vec<Duration>) -> Stats {
	samples.sort_unstable();
	Stats {
		median: percentile(&samples, 0.50),
		p95: percentile(&samples, 0.95),
		total: samples.iter().copied().sum(),
	}
}

fn us(d: Duration) -> f64 {
	d.as_secs_f64() * 1_000_000.0
}

fn fmt_us(d: Duration) -> String {
	format!("{:.2}", us(d))
}

fn fmt_total(d: Duration) -> String {
	let ms = d.as_secs_f64() * 1_000.0;
	if ms >= 1_000.0 {
		format!("{:.2} s", d.as_secs_f64())
	} else {
		format!("{:.2} ms", ms)
	}
}

fn tile_grid(count: usize) -> Vec<IVec2> {
	let width = (count as f64).sqrt().ceil() as i32;
	(0..count)
		.map(|index| {
			let index = index as i32;
			IVec2::new(index.rem_euclid(width), index / width)
		})
		.collect()
}

fn queries_for_span(tiles: &[IVec2], span: i32, limit: usize) -> Vec<Query> {
	if span <= 1 {
		return tiles.iter().take(limit).map(|&origin| Query { origin, span: 1 }).collect();
	}
	let present: HashSet<IVec2> = tiles.iter().copied().collect();
	let mut queries = Vec::new();
	for &origin in tiles {
		if queries.len() >= limit {
			break;
		}
		let covered =
			(0..span).all(|z| (0..span).all(|x| present.contains(&(origin + IVec2::new(x, z)))));
		if covered {
			queries.push(Query { origin, span });
		}
	}
	queries
}

fn grid_catalog(tiles: &[IVec2]) -> Vec<PoiEntry> {
	let storage = HcsgStorage::default();
	let mut cx = GenerationContext::new(&storage);
	let interests = PoiInterests::one(GRID_KIND);
	let mut entries = Vec::with_capacity(tiles.len());
	let mut scratch = Vec::new();
	for &tile in tiles {
		collect_matching_in_xz_tile::<GridTiles>(
			&mut cx,
			tile,
			TILE_SIZE,
			0.0,
			&interests,
			&mut scratch,
		);
		entries.extend_from_slice(&scratch);
	}
	entries.sort_unstable_by_key(|entry| entry.id);
	entries
}

fn seed_catalog(entries: &[PoiEntry]) -> HcsgStorage {
	let storage = HcsgStorage::default();
	storage.seed(PoiCatalog { entries: entries.to_vec() }, universal_bounds());
	storage
}

fn ids_of(entries: &[PoiEntry]) -> Vec<PoiId> {
	entries.iter().map(|entry| entry.id).collect()
}

fn scan_query(entries: &[PoiEntry], query: Query, interests: &PoiInterests) -> Vec<PoiEntry> {
	let mut out: Vec<PoiEntry> = entries
		.iter()
		.copied()
		.filter(|entry| {
			if !interests.contains(entry.kind) {
				return false;
			}
			let tile = xz_tile(entry.position, TILE_SIZE);
			tile.x >= query.origin.x
				&& tile.x < query.origin.x + query.span
				&& tile.y >= query.origin.y
				&& tile.y < query.origin.y + query.span
		})
		.collect();
	out.sort_unstable_by_key(|entry| entry.id);
	out
}

fn collect_query<S: PoiOrigin>(
	cx: &mut GenerationContext,
	query: Query,
	interests: &PoiInterests,
	out: &mut Vec<PoiEntry>,
) {
	if query.span <= 1 {
		collect_matching_in_xz_tile::<S>(cx, query.origin, TILE_SIZE, 0.0, interests, out);
		return;
	}
	out.clear();
	let Some(min_region) = xz_tile_region(query.origin, TILE_SIZE, 0.0) else {
		return;
	};
	let last = query.origin + IVec2::splat(query.span - 1);
	let Some(max_region) = xz_tile_region(last, TILE_SIZE, 0.0) else {
		return;
	};
	let region = Aabb3d::from_min_max(
		Vec3::new(min_region.min.x, min_region.min.y, min_region.min.z),
		Vec3::new(max_region.max.x, max_region.max.y, max_region.max.z),
	);
	for set in cx.get_or_generate_in::<PoiSet<S>>(region) {
		for poi in &set.pois {
			if !interests.contains(poi.kind) {
				continue;
			}
			let tile = xz_tile(poi.position, TILE_SIZE);
			if tile.x >= query.origin.x
				&& tile.x < query.origin.x + query.span
				&& tile.y >= query.origin.y
				&& tile.y < query.origin.y + query.span
			{
				out.push(*poi);
			}
		}
	}
	out.sort_unstable_by_key(|poi| poi.id);
}

fn assert_paths_agree(entries: &[PoiEntry], queries: &[Query], interests: &PoiInterests) {
	let catalog = seed_catalog(entries);
	let mut catalog_cx = GenerationContext::new(&catalog);
	let grid = HcsgStorage::default();
	let mut grid_cx = GenerationContext::new(&grid);
	let mut catalog_out = Vec::new();
	let mut grid_out = Vec::new();
	for &query in queries {
		let scan = scan_query(entries, query, interests);
		collect_query::<Catalog>(&mut catalog_cx, query, interests, &mut catalog_out);
		collect_query::<GridTiles>(&mut grid_cx, query, interests, &mut grid_out);
		let scan_ids = ids_of(&scan);
		assert_eq!(scan_ids, ids_of(&catalog_out), "scan vs Catalog at {query:?}");
		assert_eq!(scan_ids, ids_of(&grid_out), "scan vs GridTiles at {query:?}");
		assert_eq!(scan.len(), query.tiles() as usize, "expected one pin per tile at {query:?}");
	}
}

fn time_scan(entries: &[PoiEntry], queries: &[Query], interests: &PoiInterests) -> Stats {
	for _ in 0..WARMUP {
		for &query in queries {
			black_box(scan_query(entries, query, interests).len());
		}
	}
	let mut samples = Vec::with_capacity(WARM_ITERS * queries.len());
	for _ in 0..WARM_ITERS {
		for &query in queries {
			let started = Instant::now();
			let hits = scan_query(entries, query, interests);
			samples.push(started.elapsed());
			black_box(hits.len());
		}
	}
	stats(samples)
}

fn time_scheme_cold<S: PoiOrigin>(
	seed: impl Fn() -> HcsgStorage,
	queries: &[Query],
	interests: &PoiInterests,
) -> Stats {
	let mut samples = Vec::with_capacity(COLD_ITERS * queries.len());
	let mut scratch = Vec::new();
	for _ in 0..COLD_ITERS {
		let storage = seed();
		let mut cx = GenerationContext::new(&storage);
		for &query in queries {
			let started = Instant::now();
			collect_query::<S>(&mut cx, query, interests, &mut scratch);
			samples.push(started.elapsed());
			black_box(scratch.len());
		}
	}
	stats(samples)
}

fn time_scheme_warm<S: PoiOrigin>(
	storage: &HcsgStorage,
	queries: &[Query],
	interests: &PoiInterests,
) -> Stats {
	let mut cx = GenerationContext::new(storage);
	let mut scratch = Vec::new();
	for &query in queries {
		collect_query::<S>(&mut cx, query, interests, &mut scratch);
	}
	for _ in 0..WARMUP {
		for &query in queries {
			collect_query::<S>(&mut cx, query, interests, &mut scratch);
			black_box(scratch.len());
		}
	}
	let mut samples = Vec::with_capacity(WARM_ITERS * queries.len());
	for _ in 0..WARM_ITERS {
		for &query in queries {
			let started = Instant::now();
			collect_query::<S>(&mut cx, query, interests, &mut scratch);
			samples.push(started.elapsed());
			black_box(scratch.len());
		}
	}
	stats(samples)
}

fn print_scan_row(n: usize, tiles: i32, measured: &Stats) {
	eprintln!(
		"| {n} | {tiles} | scan | — | — | {} | {} | {} |",
		fmt_us(measured.median),
		fmt_us(measured.p95),
		fmt_total(measured.total),
	);
}

fn print_scheme_row(n: usize, tiles: i32, path: &str, cold: &Stats, warm: &Stats) {
	eprintln!(
		"| {n} | {tiles} | {path} | {} | {} | {} | {} | {} |",
		fmt_us(cold.median),
		fmt_us(cold.p95),
		fmt_us(warm.median),
		fmt_us(warm.p95),
		fmt_total(warm.total),
	);
}

/// Fast identity check so CI catches a path split without running the bench.
#[test]
fn xz_tile_scheme_paths_agree() {
	let interests = PoiInterests::one(GRID_KIND);
	let tiles = tile_grid(32);
	let entries = grid_catalog(&tiles);
	assert_eq!(entries.len(), 32);
	for span in [1, 2] {
		let queries = queries_for_span(&tiles, span, 8);
		assert!(!queries.is_empty(), "no queries for span {span}");
		assert_paths_agree(&entries, &queries, &interests);
	}
}

#[test]
#[ignore]
fn xz_tile_scheme_timing() {
	if cfg!(debug_assertions) {
		eprintln!("warning: run with --release; debug timings are not comparable");
	}
	eprintln!("{RERUN}");
	eprintln!(
		"tile={TILE_SIZE} m, warmup={WARMUP}, warm iters={WARM_ITERS}, cold iters={COLD_ITERS}, probes={PROBES}"
	);
	eprintln!();
	eprintln!("### Single-tile lookup vs catalog size");
	eprintln!();
	eprintln!(
		"| n | tiles | path | cold med µs | cold p95 µs | warm med µs | warm p95 µs | warm total |"
	);
	eprintln!("| ---: | ---: | --- | ---: | ---: | ---: | ---: | ---: |");

	let interests = PoiInterests::one(GRID_KIND);
	for n in POI_COUNTS {
		let tiles = tile_grid(n);
		let entries = grid_catalog(&tiles);
		let queries = queries_for_span(&tiles, 1, PROBES);
		assert_eq!(queries.len(), PROBES.min(n));
		assert_paths_agree(&entries, &queries, &interests);

		let scan = time_scan(&entries, &queries, &interests);
		let catalog_cold =
			time_scheme_cold::<Catalog>(|| seed_catalog(&entries), &queries, &interests);
		let catalog_store = seed_catalog(&entries);
		let catalog_warm = time_scheme_warm::<Catalog>(&catalog_store, &queries, &interests);
		let grid_cold = time_scheme_cold::<GridTiles>(HcsgStorage::default, &queries, &interests);
		let grid_store = HcsgStorage::default();
		let grid_warm = time_scheme_warm::<GridTiles>(&grid_store, &queries, &interests);

		print_scan_row(n, 1, &scan);
		print_scheme_row(n, 1, "Catalog", &catalog_cold, &catalog_warm);
		print_scheme_row(n, 1, "GridTiles", &grid_cold, &grid_warm);
	}

	eprintln!();
	eprintln!("### Tile-span lookup (n = {SPAN_SWEEP_N})");
	eprintln!();
	eprintln!(
		"| n | tiles | path | cold med µs | cold p95 µs | warm med µs | warm p95 µs | warm total |"
	);
	eprintln!("| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |");

	let tiles = tile_grid(SPAN_SWEEP_N);
	let entries = grid_catalog(&tiles);
	for span in TILE_SPANS {
		let queries = queries_for_span(&tiles, span, PROBES);
		assert!(!queries.is_empty(), "no queries for span {span}");
		assert_paths_agree(&entries, &queries, &interests);

		let scan = time_scan(&entries, &queries, &interests);
		let catalog_cold =
			time_scheme_cold::<Catalog>(|| seed_catalog(&entries), &queries, &interests);
		let catalog_store = seed_catalog(&entries);
		let catalog_warm = time_scheme_warm::<Catalog>(&catalog_store, &queries, &interests);
		let grid_cold = time_scheme_cold::<GridTiles>(HcsgStorage::default, &queries, &interests);
		let grid_store = HcsgStorage::default();
		let grid_warm = time_scheme_warm::<GridTiles>(&grid_store, &queries, &interests);

		print_scan_row(SPAN_SWEEP_N, span * span, &scan);
		print_scheme_row(SPAN_SWEEP_N, span * span, "Catalog", &catalog_cold, &catalog_warm);
		print_scheme_row(SPAN_SWEEP_N, span * span, "GridTiles", &grid_cold, &grid_warm);
	}
}
