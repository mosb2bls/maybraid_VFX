use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use intelligence_lod::{IntelligenceBand, IntelligenceLod, IntelligencePriority};
use spotting_intelligence::{InterestLayers, SpotDirective, SpottingUser};

use crate::{
	AffiliationStrength, Affiliations, ThreatDiscoverLimits, ThreatDiscoveryPolicy, ThreatGroupId,
	ThreatId, ThreatIntelligencePlugin, ThreatIntelligenceUser, ThreatKnowledge, ThreatRecord,
	ThreatRegistry, ThreatSource, ThreatSubject,
};

const FFA: ThreatGroupId = ThreatGroupId::group(1);

fn ffa_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(FFA, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(FFA, AffiliationStrength::permanent(1.0));
	affiliations
}

const PUBLIC: ThreatGroupId = ThreatGroupId::group(2);
const GUARD: ThreatGroupId = ThreatGroupId::group(3);
const CRIMINAL: ThreatGroupId = ThreatGroupId::group(4);

fn guard_affiliations(id: ThreatId) -> Affiliations {
	let mut affiliations = Affiliations::with_self(id);
	affiliations.join(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.join(GUARD, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(GUARD, AffiliationStrength::permanent(1.0));
	affiliations
}

#[test]
fn shared_ffa_group_classifies_another_member_as_a_threat() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let recipient = ffa_affiliations(ThreatId(1));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity,
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let mut knowledge = ThreatKnowledge::default();
	assert!(knowledge
		.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2)
		.is_some());
	assert_eq!(knowledge.len(), 1);
	Ok(())
}

#[test]
fn guard_mitigation_cancels_public_aggravation() -> anyhow::Result<()> {
	let civilian = {
		let mut affiliations = Affiliations::with_self(ThreatId(2));
		affiliations.join(PUBLIC, AffiliationStrength::permanent(1.0));
		affiliations
	};
	let other_guard = guard_affiliations(ThreatId(3));
	let guard = guard_affiliations(ThreatId(1));
	assert!(guard.threat_weight(&civilian, 0.0) >= 0.2);
	assert_eq!(guard.threat_weight(&other_guard, 0.0), 0.0);
	Ok(())
}

#[test]
fn stronger_aggravation_survives_mitigation() -> anyhow::Result<()> {
	let mut rogue = guard_affiliations(ThreatId(2));
	rogue.join(CRIMINAL, AffiliationStrength::permanent(2.0));
	let mut guard = guard_affiliations(ThreatId(1));
	guard.antagonize(CRIMINAL, AffiliationStrength::permanent(1.0));
	assert!((guard.threat_weight(&rogue, 0.0) - 1.0).abs() < 1e-5);
	Ok(())
}

#[test]
fn decayed_antagonism_reclassifies_retained_threats() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let mut recipient = Affiliations::with_self(ThreatId(1));
	recipient.antagonize(FFA, AffiliationStrength::decaying(1.0, 0.0, 1.0));
	let mut subject = Affiliations::with_self(ThreatId(2));
	subject.join(FFA, AffiliationStrength::permanent(1.0));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity,
		position: Vec3::X,
		salience: 1.0,
		affiliations: subject,
	};
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	knowledge.maintain(
		&recipient,
		ThreatDiscoveryPolicy { threat_threshold: 0.2, ..default() },
		4.0,
	);
	assert!(knowledge.is_empty());
	Ok(())
}

#[test]
fn source_removal_does_not_erase_another_reason() -> anyhow::Result<()> {
	let mut world = World::new();
	let entity = world.spawn_empty().id();
	let recipient = ffa_affiliations(ThreatId(1));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity,
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	knowledge.observe(&record, &recipient, ThreatSource::SESSION, 1.0, 0.0, 0.2);
	assert!(knowledge.remove_source(record.id, ThreatSource::LOCAL_SCAN));
	assert!(knowledge.get(record.id).is_some());
	Ok(())
}

#[test]
fn registry_returns_only_nearby_subjects() -> anyhow::Result<()> {
	let mut world = World::new();
	let near = world.spawn_empty().id();
	let far = world.spawn_empty().id();
	let mut registry = ThreatRegistry::default();
	registry.upsert(
		near,
		ThreatSubject::new(ThreatId(1)),
		&ffa_affiliations(ThreatId(1)),
		Vec3::X * 10.0,
	)?;
	registry.upsert(
		far,
		ThreatSubject::new(ThreatId(2)),
		&ffa_affiliations(ThreatId(2)),
		Vec3::X * 100.0,
	)?;
	let nearby = registry.local(Vec3::ZERO, 20.0);
	assert_eq!(nearby.len(), 1);
	assert_eq!(nearby[0].entity, near);
	Ok(())
}

#[test]
fn discovery_excludes_self_but_learns_another_ffa_member() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, ThreatIntelligencePlugin));
	let observer_id = ThreatId(1);
	let other_id = ThreatId(2);
	let observer = app
		.world_mut()
		.spawn((
			ThreatSubject::new(observer_id),
			ffa_affiliations(observer_id),
			ThreatIntelligenceUser::default(),
			ThreatKnowledge::default(),
			GlobalTransform::default(),
		))
		.id();
	app.world_mut().spawn((
		ThreatSubject::new(other_id),
		ffa_affiliations(other_id),
		GlobalTransform::from_translation(Vec3::X),
	));
	app.update();
	let knowledge = app.world().get::<ThreatKnowledge>(observer);
	assert!(knowledge.is_some_and(|knowledge| {
		knowledge.get(observer_id).is_none() && knowledge.get(other_id).is_some()
	}));
	Ok(())
}

#[test]
fn exported_threat_hint_is_removed_with_knowledge() -> Result<(), bevy::ecs::system::RunSystemError>
{
	let mut world = World::new();
	let threat = world.spawn_empty().id();
	let record = ThreatRecord {
		id: ThreatId(2),
		entity: threat,
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let recipient = ffa_affiliations(ThreatId(1));
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	let user = world.spawn((knowledge, SpottingUser::default())).id();
	world.run_system_once(crate::export_threat_spotting_hints)?;
	assert!(world
		.get::<SpottingUser>(user)
		.is_some_and(|spotting| spotting.hints.contains_key(&threat)));
	world
		.get_mut::<ThreatKnowledge>(user)
		.map(|mut knowledge| knowledge.remove_source(record.id, ThreatSource::LOCAL_SCAN));
	world.run_system_once(crate::export_threat_spotting_hints)?;
	assert!(world
		.get::<SpottingUser>(user)
		.is_some_and(|spotting| !spotting.hints.contains_key(&threat)));
	Ok(())
}

#[test]
fn damage_or_share_raises_spotting_to_high_band() -> anyhow::Result<()> {
	let mut world = World::new();
	let threat = world.spawn_empty().id();
	let record = ThreatRecord {
		id: ThreatId(2),
		entity: threat,
		position: Vec3::X * 150.0,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let recipient = ffa_affiliations(ThreatId(1));
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::RECEIVED_DAMAGE, 1.0, 0.0, 0.2);
	let idle = 40.0;
	let observer = world
		.spawn((
			knowledge,
			ThreatIntelligenceUser::default(),
			SpottingUser::new(Vec3::Y, [SpotDirective::new(InterestLayers::CHARACTER, idle)]),
		))
		.id();
	anyhow::ensure!(world.run_system_once(crate::export_threat_spotting_hints).is_ok());
	let spotting = world
		.get::<SpottingUser>(observer)
		.ok_or_else(|| anyhow::anyhow!("missing spotting"))?;
	anyhow::ensure!(spotting
		.directives
		.iter()
		.all(|directive| { (directive.range - ThreatDiscoveryPolicy::ALERT_RANGE).abs() < 1e-4 }));

	world
		.get_mut::<ThreatKnowledge>(observer)
		.map(|mut knowledge| knowledge.remove_source(record.id, ThreatSource::RECEIVED_DAMAGE));
	anyhow::ensure!(world.run_system_once(crate::export_threat_spotting_hints).is_ok());
	let spotting = world
		.get::<SpottingUser>(observer)
		.ok_or_else(|| anyhow::anyhow!("missing spotting after restore"))?;
	anyhow::ensure!(spotting
		.directives
		.iter()
		.all(|directive| (directive.range - idle).abs() < 1e-4));
	Ok(())
}

#[test]
fn local_scan_alone_does_not_raise_perception() -> anyhow::Result<()> {
	let recipient = ffa_affiliations(ThreatId(1));
	let record = ThreatRecord {
		id: ThreatId(2),
		entity: Entity::from_bits(2),
		position: Vec3::X,
		salience: 1.0,
		affiliations: ffa_affiliations(ThreatId(2)),
	};
	let mut knowledge = ThreatKnowledge::default();
	knowledge.observe(&record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.2);
	anyhow::ensure!(!knowledge.wants_alert_perception());
	knowledge.observe(&record, &recipient, ThreatSource::SHARED, 1.0, 0.0, 0.2);
	anyhow::ensure!(knowledge.wants_alert_perception());
	Ok(())
}

fn discover_app() -> App {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, ThreatIntelligencePlugin));
	app
}

fn short_retention_user() -> ThreatIntelligenceUser {
	ThreatIntelligenceUser::new(ThreatDiscoveryPolicy { retention_secs: 1.0, ..default() })
}

fn spawn_observer(
	app: &mut App,
	id: ThreatId,
	user: ThreatIntelligenceUser,
	lod: IntelligenceLod,
) -> Entity {
	app.world_mut()
		.spawn((
			ThreatSubject::new(id),
			ffa_affiliations(id),
			user,
			ThreatKnowledge::default(),
			GlobalTransform::default(),
			lod,
		))
		.id()
}

fn spawn_subject(app: &mut App, id: ThreatId, at: Vec3, lod: Option<IntelligenceLod>) -> Entity {
	let mut entity = app.world_mut().spawn((
		ThreatSubject::new(id),
		ffa_affiliations(id),
		GlobalTransform::from_translation(at),
	));
	if let Some(lod) = lod {
		entity.insert(lod);
	}
	entity.id()
}

#[test]
fn far_skips_scan_unless_near_takes_the_drain() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(ThreatDiscoverLimits { max_scans_per_tick: 1 });
	let other_id = ThreatId(2);
	spawn_subject(&mut app, other_id, Vec3::X, None);
	let far = spawn_observer(
		&mut app,
		ThreatId(1),
		ThreatIntelligenceUser::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
	);
	let near = spawn_observer(
		&mut app,
		ThreatId(3),
		ThreatIntelligenceUser::default(),
		IntelligenceLod::missing(),
	);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<ThreatKnowledge>(near)
		.is_some_and(|knowledge| knowledge.get(other_id).is_some()));
	anyhow::ensure!(app
		.world()
		.get::<ThreatKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(other_id).is_none()));
	anyhow::ensure!(app
		.world()
		.get::<ThreatIntelligenceUser>(far)
		.is_some_and(|user| user.next_scan_at == 0.0));
	Ok(())
}

#[test]
fn far_scan_still_runs_at_fairness_cap() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(ThreatDiscoverLimits { max_scans_per_tick: 1 });
	let other_id = ThreatId(2);
	spawn_subject(&mut app, other_id, Vec3::X, None);
	let far = spawn_observer(
		&mut app,
		ThreatId(1),
		ThreatIntelligenceUser::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: IntelligenceLod::FAIRNESS_CAP },
	);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<ThreatKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(other_id).is_some()));
	anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 0));
	Ok(())
}

#[test]
fn far_skip_does_not_reset_skips() -> anyhow::Result<()> {
	let mut app = discover_app();
	let far = spawn_observer(
		&mut app,
		ThreatId(1),
		ThreatIntelligenceUser::default(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: 3 },
	);

	app.update();

	anyhow::ensure!(app.world().get::<IntelligenceLod>(far).is_some_and(|lod| lod.skips == 3));
	anyhow::ensure!(app
		.world()
		.get::<ThreatIntelligenceUser>(far)
		.is_some_and(|user| user.next_scan_at == 0.0));
	Ok(())
}

#[test]
fn maintain_still_runs_on_far_we_do_not_scan() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(ThreatDiscoverLimits { max_scans_per_tick: 1 });
	let stale_id = ThreatId(9);
	let stale_entity = app.world_mut().spawn_empty().id();
	let mut far_knowledge = ThreatKnowledge::default();
	far_knowledge.observe(
		&ThreatRecord {
			id: stale_id,
			entity: stale_entity,
			position: Vec3::X,
			salience: 1.0,
			affiliations: ffa_affiliations(stale_id),
		},
		&ffa_affiliations(ThreatId(1)),
		ThreatSource::LOCAL_SCAN,
		1.0,
		-4.0,
		0.2,
	);
	let far = app
		.world_mut()
		.spawn((
			ThreatSubject::new(ThreatId(1)),
			ffa_affiliations(ThreatId(1)),
			short_retention_user(),
			far_knowledge,
			GlobalTransform::default(),
			IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
		))
		.id();
	let near = spawn_observer(
		&mut app,
		ThreatId(3),
		ThreatIntelligenceUser::default(),
		IntelligenceLod::missing(),
	);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);

	app.update();

	anyhow::ensure!(app
		.world()
		.get::<ThreatKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(stale_id).is_none()));
	Ok(())
}

#[test]
fn forget_does_not_run_every_frame_on_unscanned_far() -> anyhow::Result<()> {
	let mut app = discover_app();
	app.insert_resource(ThreatDiscoverLimits { max_scans_per_tick: 1 });
	let far = spawn_observer(
		&mut app,
		ThreatId(1),
		short_retention_user(),
		IntelligenceLod { band: IntelligenceBand::Far, skips: 0 },
	);
	let near = spawn_observer(
		&mut app,
		ThreatId(3),
		ThreatIntelligenceUser::default(),
		IntelligenceLod::missing(),
	);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(near, 0);
	app.world_mut().resource_mut::<IntelligencePriority>().rank.insert(far, 1);
	app.update();

	let stale_id = ThreatId(11);
	let stale_entity = app.world_mut().spawn_empty().id();
	app.world_mut().get_mut::<ThreatKnowledge>(far).expect("far knowledge").observe(
		&ThreatRecord {
			id: stale_id,
			entity: stale_entity,
			position: Vec3::X,
			salience: 1.0,
			affiliations: ffa_affiliations(stale_id),
		},
		&ffa_affiliations(ThreatId(1)),
		ThreatSource::LOCAL_SCAN,
		1.0,
		-4.0,
		0.2,
	);
	app.update();
	anyhow::ensure!(app
		.world()
		.get::<ThreatKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(stale_id).is_some()));

	app.world_mut()
		.get_mut::<ThreatIntelligenceUser>(far)
		.expect("far user")
		.next_forget_at = 0.0;
	app.update();
	anyhow::ensure!(app
		.world()
		.get::<ThreatKnowledge>(far)
		.is_some_and(|knowledge| knowledge.get(stale_id).is_none()));
	Ok(())
}

#[test]
fn settled_discover_leaves_knowledge_unchanged() -> anyhow::Result<()> {
	let mut app = discover_app();
	let observer = spawn_observer(
		&mut app,
		ThreatId(1),
		ThreatIntelligenceUser::default(),
		IntelligenceLod::missing(),
	);
	app.update();
	if let Some(mut user) = app.world_mut().get_mut::<ThreatIntelligenceUser>(observer) {
		user.next_forget_at = 100.0;
		user.next_scan_at = 100.0;
	}
	let before = app
		.world()
		.entity(observer)
		.get_change_ticks::<ThreatKnowledge>()
		.ok_or_else(|| anyhow::anyhow!("missing knowledge"))?
		.changed
		.get();
	app.update();
	let after = app
		.world()
		.entity(observer)
		.get_change_ticks::<ThreatKnowledge>()
		.ok_or_else(|| anyhow::anyhow!("missing knowledge"))?
		.changed
		.get();
	anyhow::ensure!(before == after, "settled discover rewrote knowledge: {before} -> {after}");
	Ok(())
}

#[test]
fn far_translation_under_quantum_does_not_reindex() -> anyhow::Result<()> {
	let mut app = discover_app();
	let subject = spawn_subject(
		&mut app,
		ThreatId(1),
		Vec3::ZERO,
		Some(IntelligenceLod { band: IntelligenceBand::Far, skips: 0 }),
	);
	app.update();
	let first = app
		.world()
		.resource::<ThreatRegistry>()
		.get_entity(subject)
		.map(|record| record.position);
	anyhow::ensure!(first == Some(Vec3::ZERO));

	*app.world_mut().get_mut::<GlobalTransform>(subject).expect("subject transform") =
		GlobalTransform::from_translation(Vec3::X * 8.0);
	app.update();
	let held = app
		.world()
		.resource::<ThreatRegistry>()
		.get_entity(subject)
		.map(|record| record.position);
	anyhow::ensure!(held == Some(Vec3::ZERO));

	*app.world_mut().get_mut::<GlobalTransform>(subject).expect("subject transform") =
		GlobalTransform::from_translation(Vec3::X * 20.0);
	app.update();
	let moved = app
		.world()
		.resource::<ThreatRegistry>()
		.get_entity(subject)
		.map(|record| record.position);
	anyhow::ensure!(moved == Some(Vec3::X * 20.0));
	Ok(())
}

#[test]
fn identity_change_reindexes_even_under_move_quantum() -> anyhow::Result<()> {
	let mut app = discover_app();
	let subject = spawn_subject(
		&mut app,
		ThreatId(1),
		Vec3::ZERO,
		Some(IntelligenceLod { band: IntelligenceBand::Far, skips: 0 }),
	);
	app.update();

	*app.world_mut().get_mut::<GlobalTransform>(subject).expect("subject transform") =
		GlobalTransform::from_translation(Vec3::X);
	app.world_mut().get_mut::<ThreatSubject>(subject).expect("subject").salience = 4.0;
	app.update();

	let record = app.world().resource::<ThreatRegistry>().get_entity(subject);
	anyhow::ensure!(record.is_some_and(|record| {
		record.position == Vec3::X && (record.salience - 4.0).abs() < 1e-5
	}));
	Ok(())
}

#[test]
fn missing_lod_uses_near_move_quantum() -> anyhow::Result<()> {
	let mut app = discover_app();
	let subject = spawn_subject(&mut app, ThreatId(1), Vec3::ZERO, None);
	app.update();

	*app.world_mut().get_mut::<GlobalTransform>(subject).expect("subject transform") =
		GlobalTransform::from_translation(Vec3::X * 0.5);
	app.update();
	anyhow::ensure!(app
		.world()
		.resource::<ThreatRegistry>()
		.get_entity(subject)
		.is_some_and(|record| record.position == Vec3::ZERO));

	*app.world_mut().get_mut::<GlobalTransform>(subject).expect("subject transform") =
		GlobalTransform::from_translation(Vec3::X * 2.0);
	app.update();
	anyhow::ensure!(app
		.world()
		.resource::<ThreatRegistry>()
		.get_entity(subject)
		.is_some_and(|record| record.position == Vec3::X * 2.0));
	Ok(())
}

#[test]
#[ignore]
fn local_query_clone_timing() {
	const SUBJECT_COUNT: usize = 128;
	const SCANS: usize = 10_000;
	const RADIUS: f32 = 80.0;

	let mut registry = ThreatRegistry::default();
	for index in 0..SUBJECT_COUNT {
		let id = ThreatId(index as u64 + 1);
		let angle = (index as f32 / SUBJECT_COUNT as f32) * core::f32::consts::TAU;
		let position = Vec3::new(angle.cos() * 40.0, 0.0, angle.sin() * 40.0);
		registry
			.upsert(
				Entity::from_bits(index as u64 + 1),
				ThreatSubject::new(id),
				&ffa_affiliations(id),
				position,
			)
			.expect("upsert threat");
	}

	let clone_start = std::time::Instant::now();
	for _ in 0..SCANS {
		let records = registry.local(Vec3::ZERO, RADIUS);
		std::hint::black_box(records);
	}
	let clone_elapsed = clone_start.elapsed();

	let mut scratch = Vec::new();
	let id_start = std::time::Instant::now();
	for _ in 0..SCANS {
		registry.collect_local(Vec3::ZERO, RADIUS, &mut scratch);
		for id in &scratch {
			std::hint::black_box(registry.get(*id));
		}
	}
	let id_elapsed = id_start.elapsed();

	eprintln!(
		"local_query_clone_timing: {SUBJECT_COUNT} subjects × {SCANS} scans — clone local() {:?}, collect_local+get {:?}",
		clone_elapsed,
		id_elapsed,
	);
}

#[test]
fn replace_if_changed_preserves_affiliations() {
	let source = guard_affiliations(ThreatId(7));
	let mut stored = source.clone();
	stored.replace_if_changed(&source);
	assert_eq!(stored, source);
}

/// `cargo test -p threat-intelligence observe_reconcile_memberships_timing --release -- --ignored --nocapture`
#[test]
#[ignore = "microbench"]
fn observe_reconcile_memberships_timing() {
	const KNOWN_COUNT: usize = 64;
	const OBSERVES: usize = 200_000;
	const RECONCILES: usize = 20_000;

	let mut registry = ThreatRegistry::default();
	let recipient = ffa_affiliations(ThreatId(1));
	let mut records = Vec::with_capacity(KNOWN_COUNT);
	for index in 0..KNOWN_COUNT {
		let id = ThreatId(index as u64 + 10);
		let affiliations = guard_affiliations(id);
		let position = Vec3::new(index as f32, 0.0, 0.0);
		registry
			.upsert(
				Entity::from_bits(index as u64 + 100),
				ThreatSubject::new(id),
				&affiliations,
				position,
			)
			.expect("upsert threat");
		records.push(registry.get(id).expect("indexed").clone());
	}

	let mut knowledge = ThreatKnowledge::default();
	for record in &records {
		knowledge
			.observe(record, &recipient, ThreatSource::LOCAL_SCAN, 1.0, 0.0, 0.0)
			.expect("observe");
	}

	let steady_record = &records[0];
	let legacy_assign = {
		let mut stored = steady_record.affiliations.clone();
		let start = std::time::Instant::now();
		for _ in 0..OBSERVES {
			stored = steady_record.affiliations.clone();
			std::hint::black_box(&stored);
		}
		start.elapsed()
	};

	let optimized_assign = {
		let mut stored = steady_record.affiliations.clone();
		let start = std::time::Instant::now();
		for _ in 0..OBSERVES {
			stored.replace_if_changed(&steady_record.affiliations);
			std::hint::black_box(&stored);
		}
		start.elapsed()
	};

	let legacy_reconcile_start = std::time::Instant::now();
	for _ in 0..RECONCILES {
		for record in &records {
			if knowledge.get(record.id).is_some() {
				std::hint::black_box(record.affiliations.clone());
			}
		}
	}
	let legacy_reconcile = legacy_reconcile_start.elapsed();

	let reconcile_start = std::time::Instant::now();
	for _ in 0..RECONCILES {
		knowledge.reconcile_registry(&registry);
	}
	let optimized_reconcile = reconcile_start.elapsed();

	eprintln!(
		"observe_reconcile_memberships_timing: {KNOWN_COUNT} known — legacy assign {:?}, replace_if_changed {:?} ({:.2}x); legacy reconcile {:?}, optimized reconcile {:?} ({:.2}x)",
		legacy_assign,
		optimized_assign,
		legacy_assign.as_secs_f64() / optimized_assign.as_secs_f64(),
		legacy_reconcile,
		optimized_reconcile,
		legacy_reconcile.as_secs_f64() / optimized_reconcile.as_secs_f64(),
	);
}
