//! Release timing for the live [`WorldPoiTiles`] distant-tile path.
//!
//! Seeds [`chico::ForestSelection`] (LushJungle) so [`GrovePois`] resolve on a
//! real grove grid. Urban / place families stay empty without Durham / Richmond
//! roots — `WorldPoiTiles` still walks them, matching the production call.
//!
//! ```bash
//! CXX=g++ cargo test -p maybraid-world xz_tile_world_timing --release -- --ignored --nocapture
//! ```

use std::hint::black_box;
use std::time::{Duration, Instant};

use bevy::math::IVec2;
use chico::{ForestSelection, LayeringKind};
use durham::Durham;
use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};
use mob_characters::{LOCAL_POI, SALOON_POI, URBAN_POI, VEGETATION_POI};
use poi_intelligence::{
	collect_matching_in_xz_tile, DistantTilePois, PoiEntry, PoiId, PoiInterests,
};
use richmond::Richmond;
use terrain_layer_model::OnTerrain;
use urbanization_layer_model::Urbanization;

use super::{GrovePois, PlacePois, UrbanPois, WorldPoiTiles};

const TILE_SIZE: f32 = 256.0;
const PROBES: i32 = 8;
const WARMUP: usize = 4;
const WARM_ITERS: usize = 16;
const COLD_ITERS: usize = 3;

const RERUN: &str =
	"CXX=g++ cargo test -p maybraid-world xz_tile_world_timing --release -- --ignored --nocapture";

type Ground = OnTerrain<Durham>;
type Urban = Urbanization<Richmond<Ground>>;

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

fn fmt_us(d: Duration) -> String {
	format!("{:.2}", d.as_secs_f64() * 1_000_000.0)
}

fn fmt_total(d: Duration) -> String {
	let ms = d.as_secs_f64() * 1_000.0;
	if ms >= 1_000.0 {
		format!("{:.2} s", d.as_secs_f64())
	} else {
		format!("{:.2} ms", ms)
	}
}

fn world_interests() -> PoiInterests {
	PoiInterests::one(VEGETATION_POI)
		.with_weight(URBAN_POI, 1.0)
		.with_weight(LOCAL_POI, 1.0)
		.with_weight(SALOON_POI, 1.0)
}

fn seed_forest() -> HcsgStorage {
	let storage = HcsgStorage::default();
	storage.seed(
		ForestSelection { layering: Some(LayeringKind::LushJungle), ..ForestSelection::default() },
		universal_bounds(),
	);
	storage
}

fn probe_tiles() -> Vec<IVec2> {
	let mut tiles = Vec::new();
	for z in 0..PROBES {
		for x in 0..PROBES {
			tiles.push(IVec2::new(x, z));
		}
	}
	tiles
}

fn ids_of(entries: &[PoiEntry]) -> Vec<PoiId> {
	let mut ids: Vec<PoiId> = entries.iter().map(|entry| entry.id).collect();
	ids.sort_unstable();
	ids
}

fn collect_world(
	storage: &HcsgStorage,
	tile: IVec2,
	interests: &PoiInterests,
	out: &mut Vec<PoiEntry>,
) {
	WorldPoiTiles.collect_matching_in_xz_tile(storage, tile, TILE_SIZE, 0.0, interests, out);
}

fn collect_families(storage: &HcsgStorage, tile: IVec2, interests: &PoiInterests) -> Vec<PoiEntry> {
	let mut cx = GenerationContext::new(storage);
	let mut out = Vec::new();
	let mut scratch = Vec::new();
	collect_matching_in_xz_tile::<GrovePois>(
		&mut cx,
		tile,
		TILE_SIZE,
		0.0,
		interests,
		&mut scratch,
	);
	out.extend(scratch.iter().copied());
	collect_matching_in_xz_tile::<UrbanPois<Ground>>(
		&mut cx,
		tile,
		TILE_SIZE,
		0.0,
		interests,
		&mut scratch,
	);
	out.extend(scratch.iter().copied());
	collect_matching_in_xz_tile::<PlacePois<Urban>>(
		&mut cx,
		tile,
		TILE_SIZE,
		0.0,
		interests,
		&mut scratch,
	);
	out.extend(scratch.iter().copied());
	out.sort_unstable_by_key(|poi| poi.id);
	out.dedup_by_key(|poi| poi.id);
	out
}

fn collect_grove(
	cx: &mut GenerationContext,
	tile: IVec2,
	interests: &PoiInterests,
	out: &mut Vec<PoiEntry>,
) {
	collect_matching_in_xz_tile::<GrovePois>(cx, tile, TILE_SIZE, 0.0, interests, out);
}

fn assert_world_paths_agree(tiles: &[IVec2], interests: &PoiInterests) {
	let storage = seed_forest();
	let mut world_out = Vec::new();
	let mut hits = 0usize;
	for &tile in tiles {
		collect_world(&storage, tile, interests, &mut world_out);
		let families = collect_families(&storage, tile, interests);
		assert_eq!(
			ids_of(&world_out),
			ids_of(&families),
			"WorldPoiTiles vs Grove+Urban+Place at {tile:?}"
		);
		hits += world_out.len();
	}
	assert!(hits > 0, "seeded LushJungle region produced no tile POIs");
}

fn time_world_cold(tiles: &[IVec2], interests: &PoiInterests) -> Stats {
	let mut samples = Vec::with_capacity(COLD_ITERS * tiles.len());
	let mut scratch = Vec::new();
	for _ in 0..COLD_ITERS {
		let storage = seed_forest();
		for &tile in tiles {
			let started = Instant::now();
			collect_world(&storage, tile, interests, &mut scratch);
			samples.push(started.elapsed());
			black_box(scratch.len());
		}
	}
	stats(samples)
}

fn time_world_warm(tiles: &[IVec2], interests: &PoiInterests) -> Stats {
	let storage = seed_forest();
	let mut scratch = Vec::new();
	for &tile in tiles {
		collect_world(&storage, tile, interests, &mut scratch);
	}
	for _ in 0..WARMUP {
		for &tile in tiles {
			collect_world(&storage, tile, interests, &mut scratch);
			black_box(scratch.len());
		}
	}
	let mut samples = Vec::with_capacity(WARM_ITERS * tiles.len());
	for _ in 0..WARM_ITERS {
		for &tile in tiles {
			let started = Instant::now();
			collect_world(&storage, tile, interests, &mut scratch);
			samples.push(started.elapsed());
			black_box(scratch.len());
		}
	}
	stats(samples)
}

fn time_grove_cold(tiles: &[IVec2], interests: &PoiInterests) -> Stats {
	let mut samples = Vec::with_capacity(COLD_ITERS * tiles.len());
	let mut scratch = Vec::new();
	for _ in 0..COLD_ITERS {
		let storage = seed_forest();
		let mut cx = GenerationContext::new(&storage);
		for &tile in tiles {
			let started = Instant::now();
			collect_grove(&mut cx, tile, interests, &mut scratch);
			samples.push(started.elapsed());
			black_box(scratch.len());
		}
	}
	stats(samples)
}

fn time_grove_warm(tiles: &[IVec2], interests: &PoiInterests) -> Stats {
	let storage = seed_forest();
	let mut cx = GenerationContext::new(&storage);
	let mut scratch = Vec::new();
	for &tile in tiles {
		collect_grove(&mut cx, tile, interests, &mut scratch);
	}
	for _ in 0..WARMUP {
		for &tile in tiles {
			collect_grove(&mut cx, tile, interests, &mut scratch);
			black_box(scratch.len());
		}
	}
	let mut samples = Vec::with_capacity(WARM_ITERS * tiles.len());
	for _ in 0..WARM_ITERS {
		for &tile in tiles {
			let started = Instant::now();
			collect_grove(&mut cx, tile, interests, &mut scratch);
			samples.push(started.elapsed());
			black_box(scratch.len());
		}
	}
	stats(samples)
}

fn print_scheme_row(path: &str, cold: &Stats, warm: &Stats) {
	eprintln!(
		"| {path} | {} | {} | {} | {} | {} |",
		fmt_us(cold.median),
		fmt_us(cold.p95),
		fmt_us(warm.median),
		fmt_us(warm.p95),
		fmt_total(warm.total),
	);
}

#[test]
fn xz_tile_world_paths_agree() {
	let tiles = vec![IVec2::ZERO, IVec2::new(1, 0), IVec2::new(0, 1), IVec2::new(1, 1)];
	assert_world_paths_agree(&tiles, &world_interests());
}

#[test]
#[ignore]
fn xz_tile_world_timing() {
	if cfg!(debug_assertions) {
		eprintln!("warning: run with --release; debug timings are not comparable");
	}
	let interests = world_interests();
	let tiles = probe_tiles();
	assert_world_paths_agree(&tiles, &interests);

	let world_cold = time_world_cold(&tiles, &interests);
	let world_warm = time_world_warm(&tiles, &interests);
	let grove_cold = time_grove_cold(&tiles, &interests);
	let grove_warm = time_grove_warm(&tiles, &interests);

	let storage = seed_forest();
	let mut scratch = Vec::new();
	let mut hits = 0usize;
	for &tile in &tiles {
		collect_world(&storage, tile, &interests, &mut scratch);
		hits += scratch.len();
	}

	eprintln!("{RERUN}");
	eprintln!(
		"seeded LushJungle, tile={TILE_SIZE} m, {} probes ({PROBES}×{PROBES}), warmup={WARMUP}, warm iters={WARM_ITERS}, cold iters={COLD_ITERS}, hits={hits}",
		tiles.len(),
	);
	eprintln!();
	eprintln!("| path | cold med µs | cold p95 µs | warm med µs | warm p95 µs | warm total |");
	eprintln!("| --- | ---: | ---: | ---: | ---: | ---: |");
	print_scheme_row("WorldPoiTiles", &world_cold, &world_warm);
	print_scheme_row("PoiSet<GrovePois>", &grove_cold, &grove_warm);
}
