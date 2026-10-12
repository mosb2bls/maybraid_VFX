use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use combat_targeting::{CombatTargeting, TargetSource};
use damage::DamageApplied;
use npc_intelligence::Personality;
use spotting_intelligence::SpottingUser;
use threat_intelligence::{
	export_threat_spotting_hints, AffiliationStrength, Affiliations, ThreatDiscoveryPolicy,
	ThreatGroupId, ThreatId, ThreatIntelligenceUser, ThreatKnowledge, ThreatRecord, ThreatRegistry,
	ThreatSource, ThreatSubject,
};
use threat_intelligence_damage::ThreatIntelligenceDamagePlugin;

use crate::bind::bind_mob_members;
use crate::share::{share_mob_targets, share_mob_threats, MobKnowledge, MobSharePolicy};
use crate::{spawn_mob, MemberOf, MobId, MobInstall, MobRoster, MobSlot, RosterMember};

fn pack_member(id: ThreatId) -> Affiliations {
	Affiliations::with_self(id)
}

fn user_with_threshold(threat_threshold: f32) -> ThreatIntelligenceUser {
	ThreatIntelligenceUser::new(ThreatDiscoveryPolicy { threat_threshold, ..default() })
}

fn insert_subject(
	world: &mut World,
	id: ThreatId,
	position: Vec3,
) -> anyhow::Result<(Entity, ThreatRecord)> {
	let affiliations = Affiliations::with_self(id);
	let entity = world
		.spawn((
			ThreatSubject::new(id),
			affiliations.clone(),
			GlobalTransform::from_translation(position),
		))
		.id();
	let record = ThreatRecord { id, entity, position, salience: 1.0, affiliations };
	world.resource_mut::<ThreatRegistry>().upsert(
		entity,
		ThreatSubject::new(id),
		&record.affiliations,
		position,
	)?;
	Ok((entity, record))
}

fn observe_first_hand(
	world: &mut World,
	plant: Entity,
	record: &ThreatRecord,
	source: ThreatSource,
) -> anyhow::Result<()> {
	let mut affiliations = world
		.get::<Affiliations>(plant)
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("missing affiliations"))?;
	affiliations
		.antagonize(ThreatGroupId::individual(record.id), AffiliationStrength::permanent(1.0));
	let mut knowledge = world
		.get::<ThreatKnowledge>(plant)
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("missing knowledge"))?;
	anyhow::ensure!(knowledge.observe(&record, &affiliations, source, 1.0, 0.0, 0.2).is_some());
	world.entity_mut(plant).insert((affiliations, knowledge));
	Ok(())
}

fn spawn_pack(
	world: &mut World,
	policy: MobSharePolicy,
) -> anyhow::Result<(Entity, Entity, Entity)> {
	world.init_resource::<Time>();
	world.init_resource::<ThreatRegistry>();
	let host = spawn_mob(
		&mut world.commands(),
		Transform::from_xyz(3.0, 0.0, 1.0),
		MobInstall::new(
			MobId(1),
			12.0,
			vec![
				RosterMember::new(Personality::Grazer, Vec3::X),
				RosterMember::new(Personality::Grazer, Vec3::NEG_X),
			],
		),
	);
	world.flush();
	world.entity_mut(host).insert(policy);
	let first_id = ThreatId(11);
	let second_id = ThreatId(12);
	let first = world
		.spawn((
			MemberOf { mob: host, slot: 0 },
			ThreatSubject::new(first_id),
			pack_member(first_id),
			user_with_threshold(0.2),
			ThreatKnowledge::default(),
			CombatTargeting::default(),
			SpottingUser::default(),
			GlobalTransform::from_xyz(1.0, 0.9, 0.0),
		))
		.id();
	let second = world
		.spawn((
			MemberOf { mob: host, slot: 1 },
			ThreatSubject::new(second_id),
			pack_member(second_id),
			user_with_threshold(0.2),
			ThreatKnowledge::default(),
			CombatTargeting::default(),
			SpottingUser::default(),
			GlobalTransform::from_xyz(-1.0, 0.9, 0.0),
		))
		.id();
	if let Some(mut roster) = world.get_mut::<MobRoster>(host) {
		if let Some(member) = roster.get_mut(0) {
			member.entity = Some(first);
		}
		if let Some(member) = roster.get_mut(1) {
			member.entity = Some(second);
		}
	}
	Ok((host, first, second))
}

#[test]
fn source_agnostic_write_up_stamps_shared_only() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let subject_id = ThreatId(99);
	let (_subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	observe_first_hand(&mut world, first, &record, ThreatSource::LOCAL_SCAN)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());

	let first_sources = world
		.get::<ThreatKnowledge>(first)
		.and_then(|knowledge| knowledge.get(subject_id))
		.map(|known| known.sources)
		.ok_or_else(|| anyhow::anyhow!("finder lost first-hand knowledge"))?;
	anyhow::ensure!(first_sources == ThreatSource::LOCAL_SCAN);
	let second_sources = world
		.get::<ThreatKnowledge>(second)
		.and_then(|knowledge| knowledge.get(subject_id))
		.map(|known| known.sources)
		.ok_or_else(|| anyhow::anyhow!("sibling missing shared knowledge"))?;
	anyhow::ensure!(second_sources == ThreatSource::SHARED);
	Ok(())
}

#[test]
fn damage_is_just_another_first_hand_bit() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins((MinimalPlugins, TransformPlugin, ThreatIntelligenceDamagePlugin))
		.add_message::<DamageApplied>();
	let host = spawn_mob(
		&mut app.world_mut().commands(),
		Transform::default(),
		MobInstall::new(
			MobId(2),
			12.0,
			vec![
				RosterMember::new(Personality::Grazer, Vec3::X),
				RosterMember::new(Personality::Grazer, Vec3::NEG_X),
			],
		),
	);
	app.world_mut().flush();
	let attacker_id = ThreatId(7);
	let attacker = app
		.world_mut()
		.spawn((
			ThreatSubject::new(attacker_id),
			Affiliations::with_self(attacker_id),
			GlobalTransform::from_translation(Vec3::X * 6.0),
		))
		.id();
	let first_id = ThreatId(21);
	let second_id = ThreatId(22);
	let first = app
		.world_mut()
		.spawn((
			MemberOf { mob: host, slot: 0 },
			ThreatSubject::new(first_id),
			pack_member(first_id),
			ThreatIntelligenceUser::default(),
			ThreatKnowledge::default(),
			GlobalTransform::default(),
		))
		.id();
	let second = app
		.world_mut()
		.spawn((
			MemberOf { mob: host, slot: 1 },
			ThreatSubject::new(second_id),
			pack_member(second_id),
			ThreatIntelligenceUser::default(),
			ThreatKnowledge::default(),
			GlobalTransform::from_xyz(-1.0, 0.0, 0.0),
		))
		.id();
	if let Some(mut roster) = app.world_mut().get_mut::<MobRoster>(host) {
		if let Some(member) = roster.get_mut(0) {
			member.entity = Some(first);
		}
		if let Some(member) = roster.get_mut(1) {
			member.entity = Some(second);
		}
	}
	app.update();
	app.world_mut().write_message(DamageApplied {
		target: first,
		source: Some(attacker),
		amount: 10.0,
		remaining: 90.0,
		point: Vec3::ZERO,
	});
	app.update();
	app.update();
	anyhow::ensure!(app.world_mut().run_system_once(share_mob_threats).is_ok());

	let first_sources = app
		.world()
		.get::<ThreatKnowledge>(first)
		.and_then(|knowledge| knowledge.get(attacker_id))
		.map(|known| known.sources)
		.ok_or_else(|| anyhow::anyhow!("victim missing damage knowledge"))?;
	anyhow::ensure!(first_sources.contains(ThreatSource::RECEIVED_DAMAGE));
	anyhow::ensure!(first_sources.is_first_hand());
	let second_sources = app
		.world()
		.get::<ThreatKnowledge>(second)
		.and_then(|knowledge| knowledge.get(attacker_id))
		.map(|known| known.sources);
	anyhow::ensure!(second_sources == Some(ThreatSource::SHARED));
	Ok(())
}

fn member_change_ticks(world: &World, plant: Entity) -> anyhow::Result<(u32, u32)> {
	let entity = world.entity(plant);
	let affiliations = entity
		.get_change_ticks::<Affiliations>()
		.ok_or_else(|| anyhow::anyhow!("missing affiliations"))?;
	let knowledge = entity
		.get_change_ticks::<ThreatKnowledge>()
		.ok_or_else(|| anyhow::anyhow!("missing knowledge"))?;
	Ok((affiliations.changed.get(), knowledge.changed.get()))
}

#[test]
fn settled_board_leaves_members_unchanged() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let subject_id = ThreatId(99);
	let (_subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	observe_first_hand(&mut world, first, &record, ThreatSource::LOCAL_SCAN)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world
		.get::<ThreatKnowledge>(second)
		.is_some_and(|knowledge| knowledge.get(subject_id).is_some()));
	let before = (member_change_ticks(&world, first)?, member_change_ticks(&world, second)?);

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	let after = (member_change_ticks(&world, first)?, member_change_ticks(&world, second)?);
	anyhow::ensure!(before == after, "settled share rewrote members: {before:?} -> {after:?}");
	Ok(())
}

#[test]
fn share_stays_inside_each_mob() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, _second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let other_host = spawn_mob(
		&mut world.commands(),
		Transform::from_xyz(-30.0, 0.0, 0.0),
		MobInstall::new(MobId(9), 12.0, vec![RosterMember::new(Personality::Grazer, Vec3::X)]),
	);
	world.flush();
	world.entity_mut(other_host).insert(MobSharePolicy::on());
	let stranger_id = ThreatId(31);
	let stranger = world
		.spawn((
			MemberOf { mob: other_host, slot: 0 },
			ThreatSubject::new(stranger_id),
			pack_member(stranger_id),
			user_with_threshold(0.2),
			ThreatKnowledge::default(),
			GlobalTransform::from_xyz(-30.0, 0.9, 0.0),
		))
		.id();
	let subject_id = ThreatId(99);
	let (_subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	observe_first_hand(&mut world, first, &record, ThreatSource::LOCAL_SCAN)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world
		.get::<ThreatKnowledge>(stranger)
		.is_some_and(|knowledge| knowledge.get(subject_id).is_none()));
	anyhow::ensure!(world
		.get::<MobKnowledge>(other_host)
		.is_some_and(|board| board.threats.is_empty()));
	Ok(())
}

#[test]
fn affiliation_gate_still_runs() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	world.entity_mut(second).insert(user_with_threshold(1.1));
	let subject_id = ThreatId(99);
	let (_subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	observe_first_hand(&mut world, first, &record, ThreatSource::LOCAL_SCAN)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world
		.get::<ThreatKnowledge>(second)
		.is_some_and(|knowledge| knowledge.get(subject_id).is_none()));
	Ok(())
}

#[test]
fn pack_mate_subjects_are_not_adopted() -> anyhow::Result<()> {
	let mut world = World::new();
	let (host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let second_id = world
		.get::<ThreatSubject>(second)
		.map(|subject| subject.id)
		.ok_or_else(|| anyhow::anyhow!("missing sibling subject"))?;
	let second_affiliations = world
		.get::<Affiliations>(second)
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("missing sibling affiliations"))?;
	world.resource_mut::<ThreatRegistry>().upsert(
		second,
		ThreatSubject::new(second_id),
		&second_affiliations,
		Vec3::NEG_X,
	)?;
	let record = ThreatRecord {
		id: second_id,
		entity: second,
		position: Vec3::NEG_X,
		salience: 1.0,
		affiliations: second_affiliations,
	};
	observe_first_hand(&mut world, first, &record, ThreatSource::RECEIVED_DAMAGE)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world
		.get::<MobKnowledge>(host)
		.is_some_and(|board| !board.threats.contains(&second_id)));
	anyhow::ensure!(world
		.get::<ThreatKnowledge>(second)
		.is_some_and(|knowledge| knowledge.get(second_id).is_none()));
	Ok(())
}

#[test]
fn shared_only_entries_do_not_echo() -> anyhow::Result<()> {
	let mut world = World::new();
	let (host, _first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let subject_id = ThreatId(99);
	let (_subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	let mut affiliations = world
		.get::<Affiliations>(second)
		.cloned()
		.ok_or_else(|| anyhow::anyhow!("missing affiliations"))?;
	affiliations
		.antagonize(ThreatGroupId::individual(subject_id), AffiliationStrength::permanent(1.0));
	let mut knowledge = ThreatKnowledge::default();
	anyhow::ensure!(knowledge
		.observe(&record, &affiliations, ThreatSource::SHARED, 1.0, 0.0, 0.2)
		.is_some());
	world.entity_mut(second).insert((affiliations, knowledge));

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world
		.get::<MobKnowledge>(host)
		.is_some_and(|board| !board.threats.contains(&subject_id)));
	Ok(())
}

#[test]
fn share_does_not_fabricate_a_sighting() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let subject_id = ThreatId(99);
	let (subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	observe_first_hand(&mut world, first, &record, ThreatSource::LOCAL_SCAN)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world.run_system_once(export_threat_spotting_hints).is_ok());
	anyhow::ensure!(world
		.get::<SpottingUser>(second)
		.is_some_and(|spotting| spotting.contacts.is_empty()));
	anyhow::ensure!(world
		.get::<CombatTargeting>(second)
		.is_some_and(
			|targeting| targeting.memory.is_empty() && targeting.contact(subject).is_none()
		));
	Ok(())
}

#[test]
fn target_membership_shares_semantic_bits_only() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let subject = Entity::from_bits(77);
	if let Some(mut targeting) = world.get_mut::<CombatTargeting>(first) {
		anyhow::ensure!(targeting.include(subject, TargetSource::OBJECTIVE));
	}

	anyhow::ensure!(world.run_system_once(share_mob_targets).is_ok());
	let second_targeting = world
		.get::<CombatTargeting>(second)
		.ok_or_else(|| anyhow::anyhow!("missing sibling targeting"))?;
	anyhow::ensure!(second_targeting
		.active_target(subject)
		.is_some_and(|target| target.sources == TargetSource::SHARED));
	anyhow::ensure!(second_targeting.contact(subject).is_none());
	Ok(())
}

#[test]
fn settled_target_board_leaves_members_unchanged() -> anyhow::Result<()> {
	let mut world = World::new();
	let (_host, first, second) = spawn_pack(&mut world, MobSharePolicy::on())?;
	let subject = Entity::from_bits(77);
	if let Some(mut targeting) = world.get_mut::<CombatTargeting>(first) {
		anyhow::ensure!(targeting.include(subject, TargetSource::OBJECTIVE));
	}

	anyhow::ensure!(world.run_system_once(share_mob_targets).is_ok());
	let before = world
		.entity(second)
		.get_change_ticks::<CombatTargeting>()
		.ok_or_else(|| anyhow::anyhow!("missing targeting"))?
		.changed
		.get();
	anyhow::ensure!(world.run_system_once(share_mob_targets).is_ok());
	let after = world
		.entity(second)
		.get_change_ticks::<CombatTargeting>()
		.ok_or_else(|| anyhow::anyhow!("missing targeting"))?
		.changed
		.get();
	anyhow::ensure!(before == after, "settled target share rewrote targeting: {before} -> {after}");
	Ok(())
}

#[test]
fn bind_drains_existing_board_as_shared() -> anyhow::Result<()> {
	let mut world = World::new();
	world.init_resource::<Time>();
	world.init_resource::<ThreatRegistry>();
	let subject_id = ThreatId(99);
	let host = spawn_mob(
		&mut world.commands(),
		Transform::from_xyz(3.0, 0.0, 1.0),
		MobInstall::new(MobId(8), 12.0, vec![RosterMember::new(Personality::Grazer, Vec3::X)]),
	);
	world.flush();
	let (_subject, _record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	if let Some(mut board) = world.get_mut::<MobKnowledge>(host) {
		board.adopt_threat(subject_id);
	}
	let plant = world.spawn((Transform::from_xyz(1.0, 0.9, 0.0), MobSlot(0), MobId(8))).id();
	anyhow::ensure!(world.run_system_once(bind_mob_members).is_ok());
	world.flush();

	let sources = world
		.get::<ThreatKnowledge>(plant)
		.and_then(|knowledge| knowledge.get(subject_id))
		.map(|known| known.sources);
	anyhow::ensure!(sources == Some(ThreatSource::SHARED));
	Ok(())
}

#[test]
fn disabled_policy_writes_nothing() -> anyhow::Result<()> {
	let mut world = World::new();
	let (host, first, second) = spawn_pack(&mut world, MobSharePolicy { enabled: false })?;
	let subject_id = ThreatId(99);
	let (_subject, record) = insert_subject(&mut world, subject_id, Vec3::X * 4.0)?;
	observe_first_hand(&mut world, first, &record, ThreatSource::LOCAL_SCAN)?;

	anyhow::ensure!(world.run_system_once(share_mob_threats).is_ok());
	anyhow::ensure!(world.run_system_once(share_mob_targets).is_ok());
	anyhow::ensure!(world
		.get::<MobKnowledge>(host)
		.is_some_and(|board| board.threats.is_empty() && board.targets.is_empty()));
	anyhow::ensure!(world
		.get::<ThreatKnowledge>(second)
		.is_some_and(|knowledge| knowledge.get(subject_id).is_none()));
	Ok(())
}
