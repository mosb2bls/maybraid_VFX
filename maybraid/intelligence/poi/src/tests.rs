use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;

use crate::{
	choose_poi, drive_poi_goals, refresh_poi_goals, GlobalPoi, KnownPoi, LocalPoi, Poi,
	PoiDiscoverLimits, PoiGoal, PoiGoalState, PoiGoalStatus, PoiId, PoiIntelligencePlugin,
	PoiIntelligenceUser, PoiInterest, PoiInterests, PoiKind, PoiKnowledge, PoiLearningPolicy,
	PoiObservation, PoiRecord, PoiRegistry, PoiSource, PoiVisitPolicy, PoiVisitState,
	AGENT_SEPARATION, DEFAULT_NEARBY_RADIUS,
};
use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};
use movement_intelligence::MovementIntelligence;
use routing_intelligence::{RoutingIntelligenceUser, RoutingSettings};

const CAMP: PoiKind = PoiKind::new("test/camp");
const WATER: PoiKind = PoiKind::new("test/water");

fn observation(user: Entity, id: u64, source: PoiSource, at: f32) -> PoiObservation {
	PoiObservation {
		user,
		id: PoiId(id),
		entity: None,
		kind: CAMP,
		position: Vec3::X * at,
		arrival_radius: 1.0,
		salience: 1.0,
		confidence: 0.8,
		source,
	}
}

#[test]
fn registry_separates_local_and_sparse_global_sets() -> anyhow::Result<()> {
	let mut world = World::new();
	let near = world.spawn_empty().id();
	let far = world.spawn_empty().id();
	let water = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(near, Poi::new(PoiId(1), CAMP), Vec3::X * 10.0, true, false)?;
	registry.upsert(far, Poi::new(PoiId(2), CAMP), Vec3::X * 1_000.0, false, true)?;
	registry.upsert(water, Poi::new(PoiId(3), WATER), Vec3::X * 8.0, true, true)?;

	let interests = PoiInterests::new([PoiInterest::new(CAMP, 1.0)]);
	let local = registry.local_matching(Vec3::ZERO, 200.0, &interests);
	let global = registry.global_matching(&interests);
	assert_eq!(local.iter().map(|record| record.id).collect::<Vec<_>>(), [PoiId(1)]);
	assert_eq!(global.iter().map(|record| record.id).collect::<Vec<_>>(), [PoiId(2)]);
	Ok(())
}

#[test]
fn weighted_nearby_choice_avoids_the_previous_poi() -> anyhow::Result<()> {
	let mut world = World::new();
	let first = world.spawn_empty().id();
	let second = world.spawn_empty().id();
	let far = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(first, Poi::new(PoiId(1), CAMP), Vec3::X * 20.0, true, false)?;
	registry.upsert(second, Poi::new(PoiId(2), CAMP), Vec3::Z * 24.0, false, true)?;
	registry.upsert(far, Poi::new(PoiId(3), CAMP), Vec3::X * 400.0, false, true)?;
	let interests = PoiInterests::one(CAMP);

	let selected =
		registry.choose_nearby(Vec3::ZERO, DEFAULT_NEARBY_RADIUS, &interests, &[PoiId(1)], 42);

	assert_eq!(selected.map(|record| record.id), Some(PoiId(2)));
	Ok(())
}

#[test]
fn nearby_in_lists_local_and_global_outside_the_hole() -> anyhow::Result<()> {
	let mut world = World::new();
	let near = world.spawn_empty().id();
	let mid = world.spawn_empty().id();
	let global = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(near, Poi::new(PoiId(1), CAMP), Vec3::X * 10.0, true, false)?;
	registry.upsert(mid, Poi::new(PoiId(2), CAMP), Vec3::X * 80.0, true, false)?;
	registry.upsert(global, Poi::new(PoiId(3), CAMP), Vec3::Z * 90.0, false, true)?;
	let interests = PoiInterests::one(CAMP);
	let listed = registry.nearby_in(
		Vec3::ZERO,
		crate::NearbyQuery::nearest_beyond(DEFAULT_NEARBY_RADIUS, 60.0),
		&interests,
		&[PoiId(2)],
	);
	assert_eq!(listed.iter().map(|record| record.id).collect::<Vec<_>>(), [PoiId(3)]);
	Ok(())
}

#[test]
fn registry_tracks_an_entity_when_its_poi_id_changes() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(entity, Poi::new(PoiId(1), CAMP), Vec3::ZERO, true, false)?;
	registry.upsert(entity, Poi::new(PoiId(2), CAMP), Vec3::X, true, false)?;
	assert!(registry.get(PoiId(1)).is_none());
	assert!(registry.get(PoiId(2)).is_some());
	registry.remove_entity(entity);
	assert!(registry.is_empty());
	Ok(())
}

#[test]
fn registry_rejects_duplicate_stable_ids() -> anyhow::Result<()> {
	let mut world = World::new();
	let first = world.spawn_empty().id();
	let second = world.spawn_empty().id();
	let mut registry = PoiRegistry::default();
	registry.upsert(first, Poi::new(PoiId(1), CAMP), Vec3::ZERO, true, false)?;
	registry.upsert(second, Poi::new(PoiId(2), CAMP), Vec3::X, true, false)?;
	assert!(registry.upsert(second, Poi::new(PoiId(1), CAMP), Vec3::X, true, false).is_err());
	assert!(registry.get(PoiId(1)).is_some_and(|record| record.entity == first));
	assert!(registry.get(PoiId(2)).is_none());
	assert!(registry.remove_entity(second).is_none());
	Ok(())
}

#[test]
fn knowledge_unions_sources_and_expires_only_non_durable_entries() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let mut knowledge = PoiKnowledge::default();
	knowledge.observe(observation(user, 1, PoiSource::LOCAL_SCAN, 1.0), 1.0);
	knowledge.observe(observation(user, 1, PoiSource::SHARED, 2.0), 2.0);
	knowledge.observe(observation(user, 2, PoiSource::GLOBAL_SCAN, 1.0), 1.0);
	assert!(knowledge
		.get(PoiId(1))
		.is_some_and(|known| known.sources.contains(PoiSource::LOCAL_SCAN | PoiSource::SHARED)));

	let policy = PoiLearningPolicy {
		retention_secs: 5.0,
		durable_sources: PoiSource::GLOBAL_SCAN,
		..default()
	};
	knowledge.maintain(10.0, policy);
	assert!(knowledge.get(PoiId(1)).is_none());
	assert!(knowledge.get(PoiId(2)).is_some());
	Ok(())
}

#[test]
fn removing_one_source_preserves_other_membership() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let mut knowledge = PoiKnowledge::default();
	knowledge.observe(observation(user, 1, PoiSource::LOCAL_SCAN, 1.0), 1.0);
	knowledge.observe(observation(user, 1, PoiSource::EXTERNAL, 1.0), 1.0);
	assert!(knowledge.remove_source(PoiId(1), PoiSource::LOCAL_SCAN));
	assert!(knowledge.get(PoiId(1)).is_some());
	assert!(knowledge.remove_source(PoiId(1), PoiSource::EXTERNAL));
	assert!(knowledge.get(PoiId(1)).is_none());
	Ok(())
}

#[test]
fn knowledge_rejects_non_finite_external_findings() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world.spawn_empty().id();
	let mut knowledge = PoiKnowledge::default();
	let mut invalid = observation(user, 1, PoiSource::EXTERNAL, 0.0);
	invalid.arrival_radius = f32::INFINITY;
	assert!(knowledge.observe(invalid, 0.0).is_none());
	assert!(knowledge.is_empty());
	Ok(())
}

#[test]
fn cycle_roster_has_an_explicit_stable_order() -> anyhow::Result<()> {
	let mut visits = PoiVisitState::default();
	assert!(visits.add_to_cycle(PoiId(4), 2));
	assert!(visits.add_to_cycle(PoiId(7), 2));
	assert!(!visits.add_to_cycle(PoiId(9), 2));
	assert_eq!(visits.next_cycle(false, |_| true), Some(PoiId(4)));
	assert_eq!(visits.next_cycle(false, |_| true), Some(PoiId(7)));
	assert_eq!(visits.next_cycle(false, |_| true), Some(PoiId(4)));
	Ok(())
}

#[test]
fn weighted_policy_prefers_novelty_then_keeps_circulating() -> anyhow::Result<()> {
	let poi = |id, x| KnownPoi {
		id: PoiId(id),
		entity: None,
		kind: CAMP,
		position: Vec3::X * x,
		arrival_radius: 1.0,
		salience: 1.0,
		confidence: 1.0,
		sources: PoiSource::LOCAL_SCAN,
		first_observed_at: 0.0,
		last_observed_at: 0.0,
	};
	let camp = poi(4, 0.0);
	let other = poi(7, 8.0);
	let policy = PoiVisitPolicy::Weighted {
		novelty_weight: 2.0,
		revisit_cooldown_secs: 10.0,
		repeat_weight: 1.0,
	};
	let mut visits = PoiVisitState::default();
	visits.complete(camp.id, 5.0);
	assert_eq!(choose_poi(&mut visits, policy, &[camp], 6.0, |_| 1.0), Some(camp.id));
	assert_eq!(choose_poi(&mut visits, policy, &[camp, other], 6.0, |_| 1.0), Some(other.id));
	visits.complete(other.id, 6.0);
	assert_eq!(choose_poi(&mut visits, policy, &[camp, other], 7.0, |_| 1.0), Some(camp.id));
	Ok(())
}

#[test]
fn goal_state_advances_generation_when_replaced() -> anyhow::Result<()> {
	let mut state = PoiGoalState::new(PoiId(1));
	state.status = PoiGoalStatus::Completed;
	assert_eq!(state.begin(PoiId(2)), 2);
	assert_eq!(state.target, PoiId(2));
	assert_eq!(state.status, PoiGoalStatus::Active);
	Ok(())
}

#[test]
fn linger_completes_only_after_dwelling() -> anyhow::Result<()> {
	let mut goal = PoiGoal::new(1, PoiId(4), None, CAMP, Vec3::ZERO, 1.0, 0.0, 2.0);
	assert!(!goal.linger_ready(true, 0.5));
	assert_eq!(goal.arrived_at(), Some(0.5));
	assert!(!goal.linger_ready(true, 2.4));
	assert!(goal.linger_ready(true, 2.5));
	Ok(())
}

#[test]
fn leaving_the_disk_resets_linger() -> anyhow::Result<()> {
	let mut goal = PoiGoal::new(1, PoiId(4), None, CAMP, Vec3::ZERO, 1.0, 0.0, 2.0);
	assert!(!goal.linger_ready(true, 1.0));
	assert!(!goal.linger_ready(false, 2.0));
	assert!(goal.arrived_at().is_none());
	assert!(!goal.linger_ready(true, 2.1));
	assert_eq!(goal.arrived_at(), Some(2.1));
	Ok(())
}

#[test]
fn zero_linger_completes_on_first_arrival() -> anyhow::Result<()> {
	let mut goal = PoiGoal::new(1, PoiId(4), None, CAMP, Vec3::ZERO, 1.0, 0.0, 0.0);
	assert!(goal.linger_ready(true, 0.0));
	assert!(!goal.linger_ready(false, 0.0));
	Ok(())
}

#[test]
fn marker_types_are_zero_sized() -> anyhow::Result<()> {
	assert_eq!(std::mem::size_of::<LocalPoi>(), 0);
	assert_eq!(std::mem::size_of::<GlobalPoi>(), 0);
	Ok(())
}

#[test]
fn drive_sets_a_route_without_movement_intelligence() -> anyhow::Result<()> {
	let mut world = World::new();
	let user = world
		.spawn((
			PoiGoal::new(1, PoiId(1), None, CAMP, Vec3::X * 40.0, 4.0, 0.0, 0.0),
			RoutingIntelligenceUser::new(RoutingSettings::from_segments([40.0])),
		))
		.id();
	world.run_system_once(drive_poi_goals).expect("drive");
	let routing = world.get::<RoutingIntelligenceUser>(user).expect("routing");
	assert_eq!(routing.destination, Some(Vec3::X * 40.0));
	assert!(world.get::<MovementIntelligence>(user).is_none());
	Ok(())
}

#[test]
fn from_known_salt_offsets_the_pin() -> anyhow::Result<()> {
	let known = KnownPoi {
		id: PoiId(11),
		entity: None,
		kind: CAMP,
		position: Vec3::new(10.0, 3.0, 0.0),
		arrival_radius: 8.0,
		salience: 1.0,
		confidence: 1.0,
		sources: PoiSource::LOCAL_SCAN,
		first_observed_at: 0.0,
		last_observed_at: 0.0,
	};
	let pin = PoiGoal::from_known(1, known, 0.0, 0.0, 0);
	assert_eq!(pin.location.point, known.position);
	let slotted = PoiGoal::from_known(1, known, 0.0, 0.0, 7);
	assert_ne!(slotted.location.point.xz(), known.position.xz());
	assert!((slotted.location.radius - AGENT_SEPARATION).abs() < 1e-5);
	assert_eq!(slotted.destination_salt, 7);
	Ok(())
}

#[test]
fn refresh_reapplies_destination_salt() -> anyhow::Result<()> {
	let mut world = World::new();
	let mut registry = PoiRegistry::default();
	let marker = Entity::from_bits(11);
	registry.upsert(
		marker,
		Poi::new(PoiId(11), CAMP).with_arrival_radius(8.0),
		Vec3::ZERO,
		true,
		false,
	)?;
	world.insert_resource(registry);
	let known = KnownPoi {
		id: PoiId(11),
		entity: Some(marker),
		kind: CAMP,
		position: Vec3::ZERO,
		arrival_radius: 8.0,
		salience: 1.0,
		confidence: 1.0,
		sources: PoiSource::LOCAL_SCAN,
		first_observed_at: 0.0,
		last_observed_at: 0.0,
	};
	let user = world.spawn(PoiGoal::from_known(1, known, 0.0, 0.0, 7)).id();
	let first = world.get::<PoiGoal>(user).map(|goal| goal.location);
	assert!(world.run_system_once(refresh_poi_goals).is_ok());
	let after = world.get::<PoiGoal>(user).map(|goal| goal.location);
	assert_eq!(first, after);
	assert!(first.is_some_and(|location| location.point.xz() != Vec2::ZERO));
	Ok(())
}

fn discover_app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin, PoiIntelligencePlugin));
	app.finish();
	app
}

fn camp_learner() -> PoiIntelligenceUser {
	PoiIntelligenceUser::new(PoiInterests::one(CAMP)).with_policy(PoiLearningPolicy {
		candidates_per_scan: 4,
		learning_rate_per_second: 8.0,
		retention_secs: 1.0,
		..default()
	})
}

fn index_poi(app: &mut App, id: u64, at: Vec3, local: bool, global: bool) -> anyhow::Result<PoiId> {
	let marker = app.world_mut().spawn_empty().id();
	let poi_id = PoiId(id);
	app.world_mut().resource_mut::<PoiRegistry>().upsert(
		marker,
		Poi::new(poi_id, CAMP),
		at,
		local,
		global,
	)?;
	Ok(poi_id)
}

fn spawn_learner(
	app: &mut App,
	user: PoiIntelligenceUser,
	knowledge: PoiKnowledge,
	lod: IntelligenceLod,
) -> Entity {
	app.world_mut().spawn((GlobalTransform::default(), user, knowledge, lod)).id()
}

#[test]
fn far_skips_local_scan_unless_near_takes_the_drain() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(PoiDiscoverLimits { max_scans_per_tick: 1 });
	let local = index_poi(&mut app, 1, Vec3::X * 10.0, true, false)?;
	let far = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
	);
	let near = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod::missing(),
	);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(near)
		.is_some_and(|knowledge| knowledge.get(local).is_some()));
	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(local).is_none()));
	anyhow::ensure!(app
		.world()
		.get::<PoiIntelligenceUser>(far)
		.is_some_and(|user| user.next_local_scan_at == 0.0));
	Ok(())
}

#[test]
fn far_local_scan_still_runs_at_fairness_cap() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(PoiDiscoverLimits { max_scans_per_tick: 1 });
	let local = index_poi(&mut app, 1, Vec3::X * 10.0, true, false)?;
	let far = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: IntelligenceLod::FAIRNESS_CAP },
	);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(local).is_some()));
	anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 0));
	Ok(())
}

#[test]
fn far_skip_does_not_reset_skips() -> anyhow::Result<()> {
	let mut app = discover_app();
	let far = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: 3 },
	);

	app.update();

	anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 3));
	anyhow::ensure!(app
		.world()
		.get::<PoiIntelligenceUser>(far)
		.is_some_and(|user| user.next_local_scan_at == 0.0));
	Ok(())
}

#[test]
fn settled_discover_leaves_capped_learner_unchanged() -> anyhow::Result<()> {
	let mut app = discover_app();
	let learner = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod::missing(),
	);
	app.update();
	if let Some(mut user) = app.world_mut().get_mut::<PoiIntelligenceUser>(learner) {
		user.learning_credit = user.policy.candidates_per_scan.max(1) as f32;
		user.next_forget_at = 100.0;
		user.next_local_scan_at = 100.0;
		user.next_global_scan_at = 100.0;
	}
	let before = app
		.world()
		.entity(learner)
		.get_change_ticks::<PoiIntelligenceUser>()
		.ok_or_else(|| anyhow::anyhow!("missing learner"))?
		.changed
		.get();
	app.update();
	let after = app
		.world()
		.entity(learner)
		.get_change_ticks::<PoiIntelligenceUser>()
		.ok_or_else(|| anyhow::anyhow!("missing learner"))?
		.changed
		.get();
	anyhow::ensure!(before == after, "settled discover rewrote learner: {before} -> {after}");
	Ok(())
}

#[test]
fn maintain_still_runs_on_far_we_do_not_scan() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(PoiDiscoverLimits { max_scans_per_tick: 1 });
	let stale = PoiId(9);
	let mut far_knowledge = PoiKnowledge::default();
	far_knowledge
		.observe(observation(Entity::from_bits(1), stale.0, PoiSource::LOCAL_SCAN, -4.0), -4.0);
	let far = spawn_learner(
		&mut app,
		camp_learner(),
		far_knowledge,
		IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
	);
	let near = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod::missing(),
	);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(stale).is_none()));
	Ok(())
}

#[test]
fn forget_does_not_run_every_frame_on_unscanned_far() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(PoiDiscoverLimits { max_scans_per_tick: 1 });
	let far = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
	);
	let near = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod::missing(),
	);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);
	app.update();

	let stale = PoiId(11);
	app.world_mut()
		.get_mut::<PoiKnowledge>(far)
		.expect("far knowledge")
		.observe(observation(far, stale.0, PoiSource::LOCAL_SCAN, -4.0), -4.0);
	app.update();
	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(stale).is_some()));

	app.world_mut()
		.get_mut::<PoiIntelligenceUser>(far)
		.expect("far user")
		.next_forget_at = 0.0;
	app.update();
	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(stale).is_none()));
	Ok(())
}

#[test]
fn far_does_not_run_global_scan_even_when_fair() -> anyhow::Result<()> {
	let mut app = discover_app();
	let local = index_poi(&mut app, 1, Vec3::X * 10.0, true, false)?;
	let global = index_poi(&mut app, 2, Vec3::X * 1_000.0, false, true)?;
	let far = spawn_learner(
		&mut app,
		camp_learner(),
		PoiKnowledge::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: IntelligenceLod::FAIRNESS_CAP },
	);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(local).is_some()));
	anyhow::ensure!(app
		.world()
		.get::<PoiKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(global).is_none()));
	Ok(())
}

#[test]
#[ignore]
fn local_query_clone_timing() {
	const POI_COUNT: usize = 128;
	const SCANS: usize = 10_000;
	const RADIUS: f32 = 80.0;
	let interests = PoiInterests::one(CAMP);

	let mut registry = PoiRegistry::default();
	for index in 0..POI_COUNT {
		let id = PoiId(index as u64 + 1);
		let angle = (index as f32 / POI_COUNT as f32) * core::f32::consts::TAU;
		let position = Vec3::new(angle.cos() * 40.0, 0.0, angle.sin() * 40.0);
		registry
			.upsert(Entity::from_bits(index as u64 + 1), Poi::new(id, CAMP), position, true, false)
			.expect("upsert poi");
	}

	let clone_start = std::time::Instant::now();
	for _ in 0..SCANS {
		let records = registry.local_matching(Vec3::ZERO, RADIUS, &interests);
		std::hint::black_box(records);
	}
	let clone_elapsed = clone_start.elapsed();

	let mut scratch = Vec::new();
	let id_start = std::time::Instant::now();
	for _ in 0..SCANS {
		registry.collect_local_matching(Vec3::ZERO, RADIUS, &interests, &mut scratch);
		for id in &scratch {
			std::hint::black_box(registry.get(*id));
		}
	}
	let id_elapsed = id_start.elapsed();

	eprintln!(
		"local_query_clone_timing: {POI_COUNT} pois × {SCANS} scans — clone local_matching() {:?}, collect_local_matching+get {:?}",
		clone_elapsed,
		id_elapsed,
	);
}

#[test]
fn collect_matching_in_xz_tile_matches_matching_in_xz_tile() -> anyhow::Result<()> {
	let mut registry = PoiRegistry::default();
	registry.upsert(
		Entity::from_bits(1),
		Poi::new(PoiId(1), CAMP),
		Vec3::new(40.0, 0.0, 40.0),
		true,
		false,
	)?;
	registry.upsert(
		Entity::from_bits(2),
		Poi::new(PoiId(2), CAMP),
		Vec3::new(400.0, 0.0, 400.0),
		false,
		true,
	)?;
	let interests = PoiInterests::one(CAMP);
	let tile = IVec2::ZERO;
	let records = registry.matching_in_xz_tile(tile, 256.0, 0.0, &interests);
	let mut ids = Vec::new();
	registry.collect_matching_in_xz_tile(tile, 256.0, 0.0, &interests, &mut ids);
	assert_eq!(
		records.iter().map(|record| record.id).collect::<Vec<_>>(),
		ids,
	);
	Ok(())
}

#[test]
#[ignore]
fn xz_tile_matching_clone_timing() {
	const GLOBAL_COUNT: usize = 512;
	const SCANS: usize = 5_000;
	const TILE_SIZE: f32 = 256.0;
	let interests = PoiInterests::one(CAMP);
	let tile = IVec2::ZERO;
	let on_tile = |position: Vec3| (position.xz() / TILE_SIZE).floor().as_ivec2() == tile;

	let mut registry = PoiRegistry::default();
	for index in 0..GLOBAL_COUNT {
		let id = PoiId(index as u64 + 1);
		let angle = (index as f32 / GLOBAL_COUNT as f32) * core::f32::consts::TAU;
		let position = Vec3::new(angle.cos() * 4_000.0, 0.0, angle.sin() * 4_000.0);
		registry
			.upsert(Entity::from_bits(index as u64 + 1), Poi::new(id, CAMP), position, true, true)
			.expect("upsert poi");
	}
	registry
		.upsert(
			Entity::from_bits(GLOBAL_COUNT as u64 + 1),
			Poi::new(PoiId(GLOBAL_COUNT as u64 + 1), CAMP),
			Vec3::new(32.0, 0.0, 32.0),
			true,
			true,
		)
		.expect("tile-local poi");

	let clone_start = std::time::Instant::now();
	for _ in 0..SCANS {
		let records = registry
			.global_matching(&interests)
			.into_iter()
			.filter(|record| on_tile(record.position))
			.collect::<Vec<_>>();
		std::hint::black_box(records);
	}
	let clone_elapsed = clone_start.elapsed();

	let mut global_ids = Vec::new();
	let global_id_start = std::time::Instant::now();
	for _ in 0..SCANS {
		registry.collect_global_matching(&interests, &mut global_ids);
		let matches = global_ids
			.iter()
			.filter(|id| registry.get(**id).is_some_and(|record| on_tile(record.position)))
			.count();
		std::hint::black_box(matches);
	}
	let global_id_elapsed = global_id_start.elapsed();

	let mut scratch = Vec::new();
	let tile_start = std::time::Instant::now();
	for _ in 0..SCANS {
		registry.collect_matching_in_xz_tile(tile, TILE_SIZE, 0.0, &interests, &mut scratch);
		for id in &scratch {
			std::hint::black_box(registry.get(*id));
		}
	}
	let tile_elapsed = tile_start.elapsed();

	eprintln!(
		"xz_tile_matching_clone_timing: {GLOBAL_COUNT} globals + 1 on-tile × {SCANS} probes — global_matching+filter {:?}, collect_global_matching+filter {:?}, collect_matching_in_xz_tile {:?}",
		clone_elapsed,
		global_id_elapsed,
		tile_elapsed,
	);
}

#[test]
#[ignore]
fn choose_in_clone_timing() {
	use crate::hash::unit_f32;
	use crate::NearbyQuery;

	const POI_COUNT: usize = 128;
	const PICKS: usize = 10_000;
	const RADIUS: f32 = 160.0;
	let interests = PoiInterests::one(CAMP);
	let query = NearbyQuery::weighted(RADIUS);
	let excluded = [PoiId(1)];

	let mut registry = PoiRegistry::default();
	for index in 0..POI_COUNT {
		let id = PoiId(index as u64 + 1);
		let angle = (index as f32 / POI_COUNT as f32) * core::f32::consts::TAU;
		let position = Vec3::new(angle.cos() * 80.0, 0.0, angle.sin() * 80.0);
		let global = index % 8 == 0;
		registry
			.upsert(Entity::from_bits(index as u64 + 1), Poi::new(id, CAMP), position, true, global)
			.expect("upsert poi");
	}

	let weight = |candidate: PoiRecord| {
		let interest = interests.weight(candidate.kind).unwrap_or(0.0);
		let proximity = 1.0 / (1.0 + Vec3::ZERO.distance(candidate.position) / RADIUS.max(1.0));
		interest * candidate.salience.max(0.1) * proximity
	};

	let clone_start = std::time::Instant::now();
	for seed in 0..PICKS {
		let candidates = registry.nearby_in(Vec3::ZERO, query, &interests, &excluded);
		let total: f32 = candidates.iter().copied().map(weight).sum();
		let mut draw = unit_f32(seed as u64) * total;
		for candidate in candidates {
			draw -= weight(candidate);
			if draw <= 0.0 {
				std::hint::black_box(candidate);
				break;
			}
		}
	}
	let clone_elapsed = clone_start.elapsed();

	let id_start = std::time::Instant::now();
	for seed in 0..PICKS {
		let chosen = registry.choose_in(Vec3::ZERO, query, &interests, &excluded, seed as u64);
		std::hint::black_box(chosen);
	}
	let id_elapsed = id_start.elapsed();

	eprintln!(
		"choose_in_clone_timing: {POI_COUNT} pois × {PICKS} picks — nearby_in+weighted {:?}, choose_in {:?}",
		clone_elapsed,
		id_elapsed,
	);
}
