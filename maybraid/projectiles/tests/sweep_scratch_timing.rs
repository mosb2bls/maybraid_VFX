//! Release timing for `ProjectileSystems::Sweep`.
//!
//! ```bash
//! nix develop --command bash -lc 'cargo test -p projectiles --release sweep_scratch_timing -- --ignored --nocapture'
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant};

use avian3d::prelude::*;
use bevy::mesh::MeshPlugin;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use lod_avian::PhysicsInteractionLayer;
use projectiles::{
	Flight, ProjectileContact, ProjectileSystems, ProjectilesPlugin, MAX_SWEEP_METERS,
};

struct CountingAlloc;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);

unsafe impl GlobalAlloc for CountingAlloc {
	unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
		ALLOCS.fetch_add(1, Ordering::Relaxed);
		BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
		unsafe { System.alloc(layout) }
	}

	unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
		ALLOCS.fetch_add(1, Ordering::Relaxed);
		BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
		unsafe { System.alloc_zeroed(layout) }
	}

	unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
		ALLOCS.fetch_add(1, Ordering::Relaxed);
		BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
		unsafe { System.realloc(ptr, layout, new_size) }
	}

	unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
		unsafe { System.dealloc(ptr, layout) }
	}
}

#[global_allocator]
static GLOBAL: CountingAlloc = CountingAlloc;

const DT: f32 = 1.0 / 60.0;
const WARMUP_FRAMES: u32 = 16;
const MEASURE_FRAMES: u32 = 48;
const REPEATS: u32 = 5;
const PROJECTILES: usize = 32;

#[derive(Clone, Copy)]
struct Workload {
	name: &'static str,
	walls: usize,
	overlap: bool,
	travel: f32,
}

#[derive(Resource, Default)]
struct SweepClock(Option<Instant>);

#[derive(Resource, Default)]
struct SweepTime(Duration);

#[derive(Resource, Default)]
struct FrameCounts {
	flights: u64,
	segments: u64,
	contacts: u64,
}

fn take_allocs() -> (u64, u64) {
	(ALLOCS.swap(0, Ordering::Relaxed), BYTES.swap(0, Ordering::Relaxed))
}

fn mark_sweep_start(mut clock: ResMut<SweepClock>) {
	clock.0 = Some(Instant::now());
}

fn mark_sweep_end(clock: Res<SweepClock>, mut sweep: ResMut<SweepTime>) {
	if let Some(start) = clock.0 {
		sweep.0 += start.elapsed();
	}
}

fn count_contacts(mut contacts: MessageReader<ProjectileContact>, mut counts: ResMut<FrameCounts>) {
	counts.contacts += contacts.read().len() as u64;
}

fn advance_projectiles(mut flights: Query<&mut Transform, With<Flight>>) {
	for mut transform in &mut flights {
		transform.translation.x += 3.0;
	}
}

fn expected_segments(flight: &Flight, pos: Vec3) -> u32 {
	let ds = pos.distance(flight.last);
	if ds <= 1e-5 {
		return 0;
	}
	let range = (flight.max_range - flight.path).max(0.0);
	let age_fraction =
		if DT <= 1e-8 { 1.0 } else { ((flight.max_age - flight.age) / DT).clamp(0.0, 1.0) };
	let allowed = ds.min(range).min(ds * age_fraction);
	if allowed <= 1e-5 {
		return 0;
	}
	((allowed / MAX_SWEEP_METERS).ceil() as u32).clamp(1, 24)
}

fn spawn_wall(world: &mut World, x: f32) {
	world.spawn((
		RigidBody::Static,
		Collider::cuboid(0.15, 2.0, 2.0),
		Transform::from_xyz(x, 0.0, 0.0),
		PhysicsInteractionLayer::fixed_layers(),
	));
}

fn spawn_projectile(world: &mut World, origin: Vec3, _travel: f32) {
	world.spawn((
		Transform::from_translation(origin + Vec3::X * 3.0),
		RigidBody::Dynamic,
		Collider::capsule(0.04, 0.3),
		Sensor,
		PhysicsInteractionLayer::projectile_layers(),
		LinearVelocity(Vec3::ZERO),
		GravityScale(0.0),
		Flight::spawn(origin, 80.0, 8.0, 4.0),
	));
}

fn live_flight_count(world: &mut World) -> usize {
	let mut query = world.query::<&Flight>();
	query.iter(world).count()
}

fn replenish(app: &mut App, travel: f32) {
	let live = live_flight_count(app.world_mut());
	if live >= PROJECTILES {
		return;
	}
	for index in live..PROJECTILES {
		let z = (index as f32 - PROJECTILES as f32 * 0.5) * 0.25;
		spawn_projectile(app.world_mut(), Vec3::new(-2.0, 0.0, z), travel);
	}
}

fn setup(workload: Workload) -> App {
	let mut app = App::new();
	app.add_plugins((
		MinimalPlugins,
		TransformPlugin,
		PhysicsPlugins::default(),
		AssetPlugin::default(),
		MeshPlugin,
		ProjectilesPlugin,
	))
	.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_secs_f32(DT)))
	.init_resource::<SweepClock>()
	.init_resource::<SweepTime>()
	.init_resource::<FrameCounts>()
	.add_systems(
		PostUpdate,
		(
			advance_projectiles.before(ProjectileSystems::Sweep),
			count_segments.after(advance_projectiles).before(ProjectileSystems::Sweep),
			mark_sweep_start.after(count_segments).before(ProjectileSystems::Sweep),
			mark_sweep_end.after(ProjectileSystems::Sweep),
			count_contacts.after(ProjectileSystems::Sweep),
		),
	);
	app.finish();
	for index in 0..workload.walls {
		let x =
			if workload.overlap { 1.5 + index as f32 * 0.08 } else { 1.5 + index as f32 * 1.25 };
		spawn_wall(app.world_mut(), x);
	}
	app.update();
	replenish(&mut app, workload.travel);
	app
}

fn count_segments(flights: Query<(&Flight, &Transform)>, mut counts: ResMut<FrameCounts>) {
	for (flight, transform) in &flights {
		counts.flights += 1;
		counts.segments += u64::from(expected_segments(flight, transform.translation));
	}
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

fn run_workload(workload: Workload) {
	let mut frame_ns = [0u128; REPEATS as usize];
	let mut sweep_ns = [0u128; REPEATS as usize];
	let mut allocs = [0u64; REPEATS as usize];
	let mut bytes = [0u64; REPEATS as usize];
	let mut flights = 0u64;
	let mut segments = 0u64;
	let mut contacts = 0u64;

	for repeat in 0..REPEATS {
		let mut app = setup(workload);
		for _ in 0..WARMUP_FRAMES {
			replenish(&mut app, workload.travel);
			app.update();
		}
		app.world_mut().resource_mut::<SweepTime>().0 = Duration::ZERO;
		app.world_mut().resource_mut::<FrameCounts>().flights = 0;
		app.world_mut().resource_mut::<FrameCounts>().segments = 0;
		app.world_mut().resource_mut::<FrameCounts>().contacts = 0;
		take_allocs();
		let started = Instant::now();
		for _ in 0..MEASURE_FRAMES {
			replenish(&mut app, workload.travel);
			app.update();
		}
		frame_ns[repeat as usize] = started.elapsed().as_nanos();
		sweep_ns[repeat as usize] = app.world().resource::<SweepTime>().0.as_nanos();
		let (count, size) = take_allocs();
		allocs[repeat as usize] = count;
		bytes[repeat as usize] = size;
		let counts = app.world().resource::<FrameCounts>();
		flights += counts.flights;
		segments += counts.segments;
		contacts += counts.contacts;
	}

	let frame_med = median_nanos(&mut frame_ns) / MEASURE_FRAMES as f64;
	let sweep_med = median_nanos(&mut sweep_ns) / MEASURE_FRAMES as f64;
	let frame_min = frame_ns.iter().copied().min().unwrap_or(0) as f64 / MEASURE_FRAMES as f64;
	let frame_max = frame_ns.iter().copied().max().unwrap_or(0) as f64 / MEASURE_FRAMES as f64;
	let alloc_med = {
		let mut samples = allocs;
		samples.sort_unstable();
		samples[samples.len() / 2]
	};
	let bytes_med = {
		let mut samples = bytes;
		samples.sort_unstable();
		samples[samples.len() / 2]
	};

	eprintln!(
		"sweep_scratch_timing {name}: flights {flights} segments {segments} contacts {contacts} | sweep {sweep_us:.2} µs/frame (med) | frame {frame_us:.2} µs/frame (med; min {frame_min:.2} max {frame_max:.2}) | allocs {alloc_med} / {bytes_med} B per {frames} frames after warmup",
		name = workload.name,
		sweep_us = sweep_med / 1000.0,
		frame_us = frame_med / 1000.0,
		frame_min = frame_min / 1000.0,
		frame_max = frame_max / 1000.0,
		frames = MEASURE_FRAMES,
	);
}

#[test]
#[ignore = "release micro-benchmark; run with --release --ignored --nocapture"]
fn sweep_scratch_timing() {
	if !cfg!(debug_assertions) {
		// release path
	} else {
		eprintln!("warning: run with --release; debug timings are not comparable");
	}
	for workload in [
		Workload { name: "empty-space", walls: 0, overlap: false, travel: 6.0 },
		Workload { name: "sparse-contacts", walls: 4, overlap: false, travel: 6.0 },
		Workload { name: "dense-overlap", walls: 24, overlap: true, travel: 6.0 },
		Workload { name: "multi-segment", walls: 12, overlap: false, travel: 8.0 },
	] {
		run_workload(workload);
	}
}
