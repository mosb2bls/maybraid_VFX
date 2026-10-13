use std::any::TypeId;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};

use crate::gen::tests::test_utils::cell;
use crate::gen::{Id, OriginalId};

use super::bounds::HcsgClass;
use super::demand::{quantum_cost, QuantumProgress, SubscriptionId, QUANTUM_IDS, QUANTUM_TIME};
use super::{GenerationContext, GenerationScheme, HcsgDemand, HcsgStorage, Outstanding};
use crate::hcsg::worker::HcsgWorker;

fn subscribe<T: GenerationScheme>(
	demand: &HcsgDemand,
	previous: Option<SubscriptionId>,
	regions: Vec<Aabb3d>,
	focus: Option<Vec3>,
) -> SubscriptionId {
	demand.subscribe::<T>(previous, regions, focus, HcsgClass::Near)
}

fn subscribe_class<T: GenerationScheme>(
	demand: &HcsgDemand,
	previous: Option<SubscriptionId>,
	regions: Vec<Aabb3d>,
	class: HcsgClass,
) -> SubscriptionId {
	demand.subscribe::<T>(previous, regions, None, class)
}

fn yield_quantum(
	demand: &HcsgDemand,
	id: SubscriptionId,
	discovered: Option<Vec<Id>>,
	cursor: usize,
	cost: u32,
	done: bool,
) {
	demand.finish_quantum(
		id,
		QuantumProgress {
			discovered,
			cursor,
			cost: f64::from(cost),
			done,
			..QuantumProgress::default()
		},
		None,
	);
}

const IDLE: Duration = Duration::from_secs(10);

/// Session root every [`Ground`] reads.
struct Root {
	seed: u32,
}

#[derive(Debug, PartialEq)]
struct Ground {
	cell: Aabb3d,
	seed: u32,
}

/// Depends on the [`Ground`] at the same id.
struct Cover {
	ground: Arc<Ground>,
}

/// Independent of [`Ground`]; a large channel that must not pin Ground.
struct FarField;

/// Depends on itself.
struct Cycle;

struct Panicky;

fn span(x: f32, width: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
}

fn cells_in(region: Aabb3d) -> Vec<OriginalId> {
	let start = region.min.x.floor() as i32;
	let end = region.max.x.ceil() as i32;
	(start..end).map(|x| OriginalId::new(Id::from_cell(cell(x as f32)))).collect()
}

impl GenerationScheme for Ground {
	const INDEX_SCALE: DVec3 = DVec3::ONE;

	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let root = cx.get::<Root>(Id::Universal)?;
		let cell = id.origin_cell_bounds()?;
		Some((Self { cell, seed: root.seed }, cell))
	}
}

impl GenerationScheme for Cover {
	const INDEX_SCALE: DVec3 = DVec3::ONE;

	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let ground = cx.get_or_generate::<Ground>(id)?;
		let bounds = ground.cell;
		Some((Self { ground }, bounds))
	}
}

impl GenerationScheme for FarField {
	const INDEX_SCALE: DVec3 = DVec3::ONE;

	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(_: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		Some((Self, cell))
	}
}

impl GenerationScheme for Cycle {
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		cx.get_or_generate::<Cycle>(id)?;
		Some((Self, cell(0.0)))
	}
}

impl GenerationScheme for Panicky {
	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(_: &mut GenerationContext, _: Id) -> Option<(Self, Aabb3d)> {
		panic!("scheme failure");
	}
}

fn seeded() -> HcsgStorage {
	let storage = HcsgStorage::default();
	storage.configure::<Ground>(DVec3::ONE);
	storage.configure::<Cover>(DVec3::ONE);
	storage.configure::<FarField>(DVec3::ONE);
	storage.seed(Root { seed: 7 }, span(-1_000.0, 2_000.0));
	storage
}

#[test]
fn publish_shares_values_and_bumps_versions() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(2.0));
	let first = storage.publish(id, Arc::new(Ground { cell: cell(2.0), seed: 1 }), cell(2.0));
	let value = Arc::new(Ground { cell: cell(2.0), seed: 2 });
	let second = storage.publish(id, Arc::clone(&value), cell(2.0));
	assert!(second > first);
	assert!(storage.get::<Ground>(id).is_some_and(|stored| Arc::ptr_eq(&stored, &value)));
	assert_eq!(storage.try_entry::<Ground>(id).ok().flatten().map(|e| e.version), Some(second));
	assert_eq!(storage.overlapping::<Ground>(span(0.0, 5.0)), vec![id]);
	assert!(storage.overlapping::<Cover>(span(0.0, 5.0)).is_empty());

	let revision = storage.membership_revision::<Ground>();
	assert!(storage.remove::<Ground>(id).is_some());
	assert!(storage.membership_revision::<Ground>() > revision);
	assert!(storage
		.try_overlapping::<Ground>(span(0.0, 5.0))
		.is_ok_and(|ids| ids.is_empty()));
}

#[test]
fn scheme_registers_index_scale_on_first_publish() {
	let storage = HcsgStorage::default();
	storage.seed(Root { seed: 0 }, span(-1.0, 2.0));
	let id = Id::from_cell(cell(0.0));
	assert!(GenerationContext::new(&storage).get_or_generate::<Ground>(id).is_some());
	assert_eq!(storage.index_scale_of::<Ground>(), DVec3::ONE);
}

struct WideIndex;

impl GenerationScheme for WideIndex {
	const INDEX_SCALE: DVec3 = DVec3::new(100.0, 1.0, 1.0);
	const RETENTION_MARGIN: DVec3 = DVec3::ONE;

	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cells_in(region)
	}

	fn build_with_id(_: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let cell = id.origin_cell_bounds()?;
		Some((Self, cell))
	}
}

#[test]
fn retention_margin_not_index_scale_controls_eviction_hysteresis() {
	let storage = HcsgStorage::default();
	let near = Id::from_cell(cell(0.0));
	let far = Id::from_cell(cell(10.0));
	storage.register_scheme::<WideIndex>();
	storage.publish(near, Arc::new(WideIndex), cell(0.0));
	storage.publish(far, Arc::new(WideIndex), cell(10.0));
	let mut regions = HashMap::new();
	regions.insert(TypeId::of::<WideIndex>(), vec![span(10.0, 1.0)]);
	storage.retain_reached(&regions);
	assert!(!storage.contains::<WideIndex>(near), "margin 1 must not keep cell 0");
	assert!(storage.contains::<WideIndex>(far));
}

#[test]
fn configure_rebuilds_the_index_without_losing_values() {
	let storage = HcsgStorage::default();
	for x in 0..50 {
		let id = Id::from_cell(cell(x as f32));
		storage.publish(id, Arc::new(Ground { cell: cell(x as f32), seed: 0 }), cell(x as f32));
	}
	storage.configure::<Ground>(DVec3::splat(4.0));
	assert_eq!(storage.overlapping::<Ground>(span(10.2, 2.0)).len(), 3);
	storage.clear::<Ground>();
	assert!(storage.overlapping::<Ground>(span(0.0, 50.0)).is_empty());
}

#[test]
fn retain_overlapping_removes_entries_outside_every_region() {
	let storage = HcsgStorage::default();
	let inside = Id::from_cell(cell(2.0));
	let outside = Id::from_cell(cell(0.0));
	storage.publish(inside, Arc::new(Ground { cell: cell(2.0), seed: 1 }), cell(2.0));
	storage.publish(outside, Arc::new(Ground { cell: cell(0.0), seed: 1 }), cell(0.0));
	let revision = storage.membership_revision::<Ground>();
	storage.retain_overlapping::<Ground>(&[span(1.5, 1.0)]);
	assert!(storage.contains::<Ground>(inside));
	assert!(!storage.contains::<Ground>(outside));
	assert!(storage.membership_revision::<Ground>() > revision);
}

#[test]
fn retain_overlapping_keeps_entries_that_touch_a_region() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(1.0));
	storage.publish(id, Arc::new(Ground { cell: cell(1.0), seed: 1 }), cell(1.0));
	let revision = storage.membership_revision::<Ground>();
	storage.retain_overlapping::<Ground>(&[span(1.2, 0.3)]);
	assert!(storage.contains::<Ground>(id));
	assert_eq!(storage.membership_revision::<Ground>(), revision);
}

#[test]
fn retain_overlapping_keeps_universal_whatever_its_bounds() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	storage.publish(
		Id::Universal,
		Arc::new(Ground { cell: span(-1_000.0, 2_000.0), seed: 1 }),
		span(-1_000.0, 2_000.0),
	);
	storage.publish(id, Arc::new(Ground { cell: cell(0.0), seed: 1 }), cell(0.0));
	storage.retain_overlapping::<Ground>(&[span(50.0, 1.0)]);
	assert!(storage.contains::<Ground>(Id::Universal), "Universal is exempt by key");
	assert!(!storage.contains::<Ground>(id));
	storage.retain_overlapping::<Ground>(&[]);
	assert!(storage.contains::<Ground>(Id::Universal));
}

#[test]
fn context_generates_dependencies_once() {
	let storage = seeded();
	let id = Id::from_cell(cell(1.0));
	let mut cx = GenerationContext::new(&storage);
	let cover = cx.get_or_generate::<Cover>(id);
	assert_eq!(cover.as_ref().map(|cover| cover.ground.seed), Some(7));
	assert!(storage.contains::<Ground>(id));

	let again = cx.get_or_generate::<Cover>(id);
	assert!(matches!((cover, again), (Some(a), Some(b)) if Arc::ptr_eq(&a, &b)));
}

#[test]
fn context_returns_none_without_roots() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(1.0));
	assert!(GenerationContext::new(&storage).get_or_generate::<Cover>(id).is_none());
	assert!(!storage.contains::<Ground>(id));
}

#[test]
fn context_breaks_cycles() {
	let storage = HcsgStorage::default();
	let id = Id::from_cell(cell(0.0));
	assert!(GenerationContext::new(&storage).get_or_generate::<Cycle>(id).is_none());
	assert!(!storage.contains::<Cycle>(id));
}

#[test]
fn stale_context_publishes_nothing() {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	let stale = || true;
	let mut cx = GenerationContext::with_stale(&storage, &stale);
	assert!(cx.get_or_generate::<Cover>(id).is_none());
	assert!(!storage.contains::<Ground>(id));
}

#[test]
fn a_value_that_goes_stale_mid_build_is_not_published() {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	let checks = std::sync::atomic::AtomicUsize::new(0);
	let stale = || checks.fetch_add(1, std::sync::atomic::Ordering::Relaxed) > 0;
	let mut cx = GenerationContext::with_stale(&storage, &stale);
	assert!(cx.get_or_generate::<Ground>(id).is_none());
	assert!(!storage.contains::<Ground>(id));
}

#[test]
fn worker_fills_subscription_nearest_first() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let subscription =
		subscribe::<Cover>(&demand, None, vec![span(0.0, 4.0)], Some(Vec3::new(3.5, 0.5, 0.5)));
	assert!(demand.wait_idle(IDLE));

	let published = demand.try_read_published(subscription, 0).ok().flatten().unwrap_or_default();
	let expected: Vec<Id> = [3.0, 2.0, 1.0, 0.0].map(|x| Id::from_cell(cell(x))).into();
	assert_eq!(published, expected);
	assert!(published
		.iter()
		.all(|&id| storage.contains::<Cover>(id) && storage.contains::<Ground>(id)));

	let tail = demand.try_read_published(subscription, 2).ok().flatten();
	assert_eq!(tail, Some(expected[2..].to_vec()));
	assert_eq!(demand.try_read_published(subscription, 9).ok().flatten(), Some(Vec::new()));
	Ok(())
}

#[test]
fn worker_republishes_existing_values() -> anyhow::Result<()> {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	let existing = GenerationContext::new(&storage).get_or_generate::<Ground>(id);
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let subscription = subscribe::<Ground>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(subscription, 0).ok().flatten(), Some(vec![id]));
	assert!(
		matches!((existing, storage.get::<Ground>(id)), (Some(a), Some(b)) if Arc::ptr_eq(&a, &b))
	);
	Ok(())
}

#[test]
fn replacing_a_subscription_drops_the_previous() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let first = subscribe::<Ground>(&demand, None, vec![span(0.0, 2.0)], None);
	let second = subscribe::<Ground>(&demand, Some(first), vec![span(10.0, 2.0)], None);
	assert_eq!(demand.try_read_published(first, 0), Ok(None));
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(second, 0).ok().flatten().map(|ids| ids.len()), Some(2));

	demand.unsubscribe(second);
	assert_eq!(demand.try_read_published(second, 0), Ok(None));
	Ok(())
}

#[test]
fn advancing_the_epoch_drops_every_subscription() {
	let demand = HcsgDemand::default();
	let ground = subscribe::<Ground>(&demand, None, vec![span(0.0, 2.0)], None);
	let cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 2.0)], None);
	assert_eq!(demand.advance_epoch(), 1);
	assert_eq!(demand.epoch(), 1);
	assert_eq!(demand.try_read_published(ground, 0), Ok(None));
	assert_eq!(demand.try_read_published(cover, 0), Ok(None));
}

#[test]
fn region_pulls_skip_what_does_not_exist() {
	let storage = HcsgStorage::default();
	storage.seed(Root { seed: 3 }, span(-1_000.0, 2_000.0));
	let mut cx = GenerationContext::new(&storage);
	let grounds = cx.get_or_generate_in::<Ground>(span(0.2, 2.6));
	let cells: Vec<Aabb3d> = grounds.iter().map(|ground| ground.cell).collect();
	assert_eq!(cells, vec![cell(0.0), cell(1.0), cell(2.0)]);
	assert!(cx.get_or_generate_in::<Cycle>(span(0.0, 2.0)).is_empty());
}

fn cover_seeds(storage: &HcsgStorage) -> std::collections::BTreeMap<Id, u32> {
	storage
		.overlapping::<Cover>(span(-1.0, 20.0))
		.into_iter()
		.filter_map(|id| {
			let cover = storage.get::<Cover>(id)?;
			Some((id, cover.ground.seed))
		})
		.collect()
}

fn worker_cover_state(regions: Vec<Aabb3d>) -> anyhow::Result<std::collections::BTreeMap<Id, u32>> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let subscription = subscribe::<Cover>(&demand, None, regions, None);
	anyhow::ensure!(demand.wait_idle(IDLE));
	anyhow::ensure!(
		demand.try_read_published(subscription, 0).ok().flatten().is_some(),
		"subscription finished"
	);
	Ok(cover_seeds(&storage))
}

#[test]
fn generation_results_do_not_depend_on_demand_order() -> anyhow::Result<()> {
	let one_region = worker_cover_state(vec![span(0.0, 4.0)])?;
	let split_regions =
		worker_cover_state(vec![span(2.0, 1.0), span(0.0, 1.0), span(1.0, 1.0), span(3.0, 1.0)])?;
	anyhow::ensure!(one_region == split_regions, "region order changed stored values");
	Ok(())
}

#[test]
fn a_panicking_scheme_does_not_stop_the_worker() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;

	let panicky = subscribe::<Panicky>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(panicky, 0), Ok(Some(Vec::new())));

	let ground = subscribe::<Ground>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.try_read_published(ground, 0).ok().flatten().map(|ids| ids.len()), Some(1));
	Ok(())
}

#[test]
fn stride_schedule_is_fair_by_class_weight() {
	let demand = HcsgDemand::default();
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Ambient);
	let mut near_quanta = 0u32;
	let mut ambient_quanta = 0u32;
	for _ in 0..36 {
		let job = demand.try_pick().expect("unfinished work");
		if job.id == near {
			near_quanta += 1;
		} else {
			assert_eq!(job.id, ambient);
			ambient_quanta += 1;
		}
		yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, false);
	}
	assert_eq!((near_quanta, ambient_quanta), (32, 4), "weight 8 and weight 1 run 8:1");
}

#[test]
fn a_weight_1_subscription_completes_while_higher_classes_resubscribe() {
	let demand = HcsgDemand::default();
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Ambient);
	let mut near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let mut ambient_cost = 0u32;
	for _ in 0..64 {
		let job = demand.try_pick().expect("unfinished work");
		if job.id == ambient {
			ambient_cost += QUANTUM_IDS as u32;
			let done = ambient_cost >= 3 * QUANTUM_IDS as u32;
			yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, done);
			if done {
				assert!(demand.try_read(ambient, 0).unwrap().unwrap().done);
				return;
			}
		} else {
			yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, false);
			near = subscribe_class::<Ground>(
				&demand,
				Some(near),
				vec![span(0.0, 1.0)],
				HcsgClass::Near,
			);
		}
	}
	panic!("ambient work starved");
}

#[test]
fn replacing_a_subscription_inherits_pass() {
	let demand = HcsgDemand::default();
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Ambient);
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let first = demand.try_pick().expect("near is newer at the same pass");
	assert_eq!(first.id, near);
	yield_quantum(&demand, first.id, None, 0, QUANTUM_IDS as u32, false);
	let inherited = demand.pass_of(near).expect("near still live");
	assert!(inherited > 0.0);
	let replaced =
		subscribe_class::<Ground>(&demand, Some(near), vec![span(1.0, 1.0)], HcsgClass::Near);
	assert_eq!(demand.pass_of(replaced), Some(inherited));
	let next = demand.try_pick().expect("ambient is behind");
	assert_eq!(next.id, ambient, "resubscribing must not jump the queue");
	yield_quantum(&demand, next.id, None, 0, 1, true);
}

#[test]
fn a_switched_out_subscription_resumes_without_rediscovering() {
	let demand = HcsgDemand::default();
	let first = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 8.0)], HcsgClass::Ambient);
	let job = demand.try_pick().expect("only work");
	assert!(job.discovered.is_none());
	let ids: Vec<Id> = (0..8).map(|x| Id::from_cell(cell(x as f32))).collect();
	yield_quantum(&demand, job.id, Some(ids.clone()), 3, 3, false);
	let _other = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	let other = demand.try_pick().expect("near work");
	assert_ne!(other.id, first);
	yield_quantum(&demand, other.id, None, 0, 1, true);
	let resume = demand.try_pick().expect("first still unfinished");
	assert_eq!(resume.id, first);
	assert_eq!(resume.cursor, 3);
	assert_eq!(resume.discovered.as_deref(), Some(ids.as_slice()));
	yield_quantum(&demand, resume.id, resume.discovered, resume.cursor, 1, true);
}

#[test]
fn a_finished_subscription_resumes_at_the_current_minimum() {
	let demand = HcsgDemand::default();
	let far = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Far);
	let job = demand.try_pick().expect("only work");
	assert_eq!(job.id, far);
	yield_quantum(&demand, job.id, None, 0, 1, true);
	assert!(demand.try_read(far, 0).unwrap().unwrap().done);

	let near = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	for _ in 0..8 {
		let job = demand.try_pick().expect("near is the only unfinished work");
		assert_eq!(job.id, near);
		yield_quantum(&demand, job.id, None, 0, QUANTUM_IDS as u32, false);
	}
	let near_pass = demand.pass_of(near).expect("near still live");
	assert!(near_pass > 0.0);

	let far = subscribe_class::<Ground>(&demand, Some(far), vec![span(1.0, 1.0)], HcsgClass::Far);
	assert_eq!(
		demand.pass_of(far),
		Some(near_pass),
		"a finished subscription must not resume below the live minimum"
	);
}

#[test]
fn quantum_cost_charges_the_larger_of_ids_and_time() {
	assert_eq!(quantum_cost(32, Duration::from_millis(1)), 32.0);
	assert_eq!(quantum_cost(3, QUANTUM_TIME), QUANTUM_IDS as f64);
	assert_eq!(quantum_cost(3, QUANTUM_TIME / 2), QUANTUM_IDS as f64 / 2.0);
}

#[test]
fn a_lone_subscription_fills_one_quantum_at_a_time() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let n = 100u32;
	let subscription = subscribe::<Ground>(&demand, None, vec![span(0.0, n as f32)], None);
	assert!(demand.wait_idle(IDLE));
	let published = demand.try_read_published(subscription, 0).ok().flatten().unwrap_or_default();
	let expected: Vec<Id> = (0..n).map(|x| Id::from_cell(cell(x as f32))).collect();
	assert_eq!(published, expected);
	let picks = demand.scheduler_picks();
	let quanta = u64::from(n.div_ceil(QUANTUM_IDS as u32));
	assert!(picks >= quanta, "a lone subscription must yield each quantum, got {picks} picks");
	assert!(picks < u64::from(n), "must not return to the scheduler once per id, got {picks}");
	Ok(())
}

#[test]
fn a_small_channel_evicts_its_dependencies_while_a_large_unrelated_channel_stays(
) -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let _far = subscribe::<FarField>(&demand, None, vec![span(0.0, 40.0)], None);
	let cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 2.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert!(storage.contains::<Ground>(Id::from_cell(cell(0.0))));
	assert!(storage.contains::<FarField>(Id::from_cell(cell(0.0))));

	let _cover = subscribe::<Cover>(&demand, Some(cover), vec![span(20.0, 2.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert!(
		!storage.contains::<Ground>(Id::from_cell(cell(0.0))),
		"Ground is only reached by the small Cover channel"
	);
	assert!(
		storage.contains::<FarField>(Id::from_cell(cell(0.0))),
		"an unrelated large channel must not pin Ground, and is itself retained"
	);
	assert!(storage.contains::<Ground>(Id::from_cell(cell(20.0))));
	Ok(())
}

#[test]
fn a_replaced_subscription_inherits_its_predecessors_reach() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 2.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert!(storage.contains::<Ground>(Id::from_cell(cell(0.0))));

	// New Cover region still overlaps cell 0 once expanded by base scale 1.
	// Without inherited reach, Ground would be cleared (only Cover is known
	// until the next fill) and cell 0 would not be rebuilt.
	let _cover = subscribe::<Cover>(&demand, Some(cover), vec![span(1.0, 2.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert!(
		storage.contains::<Ground>(Id::from_cell(cell(0.0))),
		"inherited reach keeps Ground in the predecessor's expanded region"
	);
	Ok(())
}

#[test]
fn a_replaced_mid_quantum_keeps_types_that_quantum_reached() {
	let storage = seeded();
	let id = Id::from_cell(cell(0.0));
	storage.publish(id, Arc::new(Ground { cell: cell(0.0), seed: 7 }), cell(0.0));
	let demand = HcsgDemand::default();
	let cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 2.0)], None);
	let job = demand.try_pick().expect("cover work");
	let _cover = subscribe::<Cover>(&demand, Some(cover), vec![span(0.0, 2.0)], None);
	demand.finish_quantum(
		job.id,
		QuantumProgress {
			discovered: Some(vec![id]),
			cursor: 1,
			cost: 1.0,
			done: true,
			reached: HashSet::from([TypeId::of::<Ground>()]),
			..QuantumProgress::default()
		},
		Some(job.reach),
	);
	let plan = demand.try_sweep().expect("replacement requests a sweep");
	storage.retain_reached(&plan);
	assert!(
		storage.contains::<Ground>(id),
		"Ground first read in the replaced quantum must survive the sweep"
	);
}

#[test]
fn no_sweep_runs_when_the_live_subscription_set_has_not_changed() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let _cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 2.0)], None);
	assert!(demand.wait_idle(IDLE));
	let sweeps = demand.sweep_count();
	assert!(sweeps >= 1);
	assert!(demand.wait_idle(IDLE));
	assert_eq!(demand.sweep_count(), sweeps, "idle live set must not sweep again");
	Ok(())
}

#[test]
fn an_evicted_value_regenerates_identically() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let first = subscribe::<Ground>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	let id = Id::from_cell(cell(0.0));
	let before = storage.get::<Ground>(id).expect("generated");
	assert_eq!(before.seed, 7);

	let _away = subscribe::<Ground>(&demand, Some(first), vec![span(20.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert!(!storage.contains::<Ground>(id));

	let _back = subscribe::<Ground>(&demand, None, vec![span(0.0, 1.0)], None);
	assert!(demand.wait_idle(IDLE));
	let after = storage.get::<Ground>(id).expect("regenerated");
	assert_eq!(after.seed, before.seed);
	assert_eq!(after.cell, before.cell);
	Ok(())
}

#[test]
fn a_scripted_walk_plateaus_store_sizes() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage.clone(), demand.clone())?;
	let mut cover = subscribe::<Cover>(&demand, None, vec![span(0.0, 4.0)], None);
	assert!(demand.wait_idle(IDLE));
	let mut max_ground = 0usize;
	let mut max_cover = 0usize;
	for start in (0..40).step_by(4) {
		cover = subscribe::<Cover>(&demand, Some(cover), vec![span(start as f32, 4.0)], None);
		assert!(demand.wait_idle(IDLE));
		max_ground = max_ground.max(storage.len::<Ground>());
		max_cover = max_cover.max(storage.len::<Cover>());
	}
	assert!(
		max_ground <= 8 && max_cover <= 8,
		"walk visited 40 cells; stores must plateau, got ground={max_ground} cover={max_cover} sizes={:?}",
		storage.store_sizes()
	);
	let start = Id::from_cell(cell(0.0));
	assert!(
		!storage.contains::<Cover>(start) && !storage.contains::<Ground>(start),
		"walking away must evict the start Cover and its Ground"
	);
	let _cover = subscribe::<Cover>(&demand, Some(cover), vec![span(0.0, 4.0)], None);
	assert!(demand.wait_idle(IDLE));
	assert!(
		storage.contains::<Cover>(start) && storage.contains::<Ground>(start),
		"returning to the start rebuilds Cover and nested Ground"
	);
	Ok(())
}

#[test]
fn outstanding_counts_an_undiscovered_subscription() {
	let demand = HcsgDemand::default();
	let _near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 4.0)], HcsgClass::Near);
	let outstanding = demand.try_outstanding(&[HcsgClass::Near]).unwrap();
	assert_eq!(outstanding.undiscovered, 1);
	assert_eq!(outstanding.remaining, 0);
}

#[test]
fn outstanding_counts_fall_as_ids_are_generated() {
	let demand = HcsgDemand::default();
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 8.0)], HcsgClass::Near);
	let job = demand.try_pick().expect("near work");
	let ids: Vec<Id> = (0..8).map(|x| Id::from_cell(cell(x as f32))).collect();
	yield_quantum(&demand, job.id, Some(ids), 3, 3, false);
	let outstanding = demand.try_outstanding(&[HcsgClass::Near]).unwrap();
	assert_eq!(outstanding.undiscovered, 0);
	assert_eq!(outstanding.remaining, 5);
	let _ = near;
}

#[test]
fn outstanding_counts_are_released_on_replace_and_unsubscribe() {
	let demand = HcsgDemand::default();
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 8.0)], HcsgClass::Near);
	let job = demand.try_pick().expect("near work");
	let ids: Vec<Id> = (0..8).map(|x| Id::from_cell(cell(x as f32))).collect();
	yield_quantum(&demand, job.id, Some(ids), 3, 3, false);
	assert_eq!(demand.try_outstanding(&[HcsgClass::Near]).unwrap().remaining, 5);

	let replaced =
		subscribe_class::<Ground>(&demand, Some(near), vec![span(0.0, 2.0)], HcsgClass::Near);
	assert_eq!(
		demand.try_outstanding(&[HcsgClass::Near]).unwrap(),
		Outstanding { undiscovered: 1, remaining: 0 },
		"replacement drops the predecessor's remaining ids"
	);
	demand.unsubscribe(replaced);
	assert_eq!(demand.try_outstanding(&[HcsgClass::Near]).unwrap(), Outstanding::default());
}

#[test]
fn outstanding_counts_filter_by_class() {
	let demand = HcsgDemand::default();
	let ambient = subscribe_class::<Cover>(&demand, None, vec![span(0.0, 8.0)], HcsgClass::Ambient);
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 4.0)], HcsgClass::Near);
	let job = demand.try_pick().expect("near is newer at pass 0");
	assert_eq!(job.id, near);
	let ids: Vec<Id> = (0..4).map(|x| Id::from_cell(cell(x as f32))).collect();
	yield_quantum(&demand, job.id, Some(ids), 1, 1, false);
	assert_eq!(
		demand.try_outstanding(&[HcsgClass::Near]).unwrap(),
		Outstanding { undiscovered: 0, remaining: 3 }
	);
	assert_eq!(
		demand.try_outstanding(&[HcsgClass::Ambient]).unwrap(),
		Outstanding { undiscovered: 1, remaining: 0 }
	);
	assert_eq!(demand.try_outstanding(&[HcsgClass::Far]).unwrap(), Outstanding::default());
	let _ = ambient;
}

#[test]
fn outstanding_skips_a_subscription_that_finishes_without_ids() {
	let demand = HcsgDemand::default();
	let near = subscribe_class::<Ground>(&demand, None, vec![span(0.0, 4.0)], HcsgClass::Near);
	let job = demand.try_pick().expect("near work");
	yield_quantum(&demand, job.id, None, 0, 1, true);
	assert_eq!(
		demand.try_outstanding(&[HcsgClass::Near]).unwrap(),
		Outstanding::default(),
		"done without discovery must not stay undiscovered"
	);
	let _ = near;
}

/// Regression guard for worker sweep cost: eviction must stay linear in store
/// size with a single write lock per typed store.
#[test]
fn retention_sweep_on_a_large_store_stays_bounded() {
	use std::time::{Duration, Instant};

	const ENTRIES: usize = 4_000;
	const ROUNDS: usize = 25;
	let mut store = super::node_store::NodeStore::new(DVec3::ONE);
	for i in 0..ENTRIES {
		let id = Id::from_cell(cell(i as f32));
		store.put(
			id,
			super::node_store::StoredEntry {
				value: i as u32,
				bounds: cell(i as f32),
				version: crate::gen::Version(i as u64 + 1),
			},
			i as u64 + 1,
		);
	}
	let keep = vec![span(1_000.0, 16.0)];
	let started = Instant::now();
	for round in 0..ROUNDS {
		for i in 0..ENTRIES {
			let id = Id::from_cell(cell(i as f32));
			store.put(
				id,
				super::node_store::StoredEntry {
					value: i as u32,
					bounds: cell(i as f32),
					version: crate::gen::Version((round * ENTRIES + i) as u64 + 1),
				},
				(round * ENTRIES + i) as u64 + 1,
			);
		}
		let removed = store.evict_outside(&keep, (round * ENTRIES + ENTRIES) as u64 + 1);
		assert!(
			removed.len() > ENTRIES / 2,
			"most entries should fall outside the retention window"
		);
	}
	let elapsed = started.elapsed();
	assert!(elapsed < Duration::from_secs(3), "25 sweeps over 4k entries took {:?}", elapsed);
}

#[test]
fn outstanding_does_not_count_a_panicking_subscription() -> anyhow::Result<()> {
	let storage = seeded();
	let demand = HcsgDemand::default();
	let _worker = HcsgWorker::spawn(storage, demand.clone())?;
	let _near = subscribe_class::<Panicky>(&demand, None, vec![span(0.0, 1.0)], HcsgClass::Near);
	anyhow::ensure!(demand.wait_idle(IDLE));
	assert_eq!(
		demand.try_outstanding(&[HcsgClass::Near]).unwrap(),
		Outstanding::default(),
		"a panic during first discovery finishes the subscription"
	);
	Ok(())
}
