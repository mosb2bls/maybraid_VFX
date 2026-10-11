#![allow(clippy::expect_used)]

use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use damage::Health;
use npc_intelligence::{NpcInstall, NpcIntelligence, Personality};
use poi_intelligence::{
	drive_poi_goals, PoiGoal, PoiId, PoiIntelligenceUser, PoiInterest, PoiInterests, PoiKind,
};
use routing_intelligence::{LayerPlan, RoutePlan, RoutingIntelligenceUser};
use tether_intelligence::{Tether, TetherIntelligenceUser, TetherObjective};
use threat_intelligence::{
	AffiliationStrength, Affiliations, ThreatGroupId, ThreatId, ThreatSubject,
};

use crate::bind::bind_mob_members;
use crate::lifecycle::{queue_downed_member_deaths, write_back_mob_roster};
use crate::lock::{
	apply_mob_tether_subjects, expire_mob_tether_locks, forget_mob_tether_lock_when_leaving,
	lock_mobs_on_poi_arrival,
};
use crate::{
	spawn_mob, MemberOf, Mob, MobAffiliations, MobId, MobInstall, MobIntelligencePlugin,
	MobKnowledge, MobMemberNeeded, MobRespawn, MobRespawnAt, MobRoster, MobSharePolicy, MobSlot,
	MobTetherLock, RosterMember,
};

const PACK: ThreatGroupId = ThreatGroupId::group(9);
const PUBLIC: ThreatGroupId = ThreatGroupId::group(1);

fn pack_affiliations() -> MobAffiliations {
	let mut affiliations = Affiliations::default();
	affiliations.join(PACK, AffiliationStrength::permanent(1.0));
	affiliations.antagonize(PUBLIC, AffiliationStrength::permanent(1.0));
	affiliations.mitigate(PACK, AffiliationStrength::permanent(1.0));
	MobAffiliations::new(affiliations)
}

fn interests() -> PoiInterests {
	PoiInterests::new([PoiInterest::new(PoiKind::new("mob/camp"), 1.0)])
}

fn spawn_host(world: &mut World, id: MobId, members: Vec<RosterMember>) -> Entity {
	let mut commands = world.commands();
	let host = spawn_mob(
		&mut commands,
		Transform::from_xyz(3.0, 0.0, 1.0),
		MobInstall::new(id, 12.0, members)
			.with_interests(interests())
			.with_affiliations(pack_affiliations()),
	);
	world.flush();
	host
}

#[test]
fn host_is_a_tether_anchor() {
	let mut world = World::new();
	let host =
		spawn_host(&mut world, MobId(1), vec![RosterMember::new(Personality::Grazer, Vec3::X)]);
	assert!(world.get::<Tether>(host).is_some());
	assert!(world.get::<Mob>(host).is_some_and(|mob| (mob.leash - 12.0).abs() < 1e-4));
	assert!(world.get::<MobKnowledge>(host).is_some());
	assert_eq!(world.get::<MobSharePolicy>(host), Some(&MobSharePolicy::on()));
}

#[test]
fn bind_by_mob_id_stamps_membership_and_installs() {
	let mut world = World::new();
	let pose = Vec3::new(4.0, 0.9, 2.0);
	let personal = PoiKind::new("mob/personal");
	let host = spawn_host(
		&mut world,
		MobId(7),
		vec![RosterMember::new(Personality::Grazer, pose)
			.with_armed(false)
			.with_interests(PoiInterests::one(personal))],
	);
	let plant = world.spawn((Transform::from_translation(pose), MobSlot(0), MobId(7))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();

	assert_eq!(world.get::<MemberOf>(plant), Some(&MemberOf { mob: host, slot: 0 }));
	assert_eq!(world.get::<MobRoster>(host).and_then(|roster| roster.get(0)?.entity), Some(plant));
	assert!(world.get::<NpcIntelligence>(plant).is_some());
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective),
		Some(TetherObjective::Tether(subject, _)) if subject == host
	));
	let affiliations = world.get::<threat_intelligence::Affiliations>(plant).expect("affiliations");
	assert!(affiliations.memberships.contains_key(&PACK));
	assert!(world.get::<ThreatSubject>(plant).is_some());
	let learner = world.get::<PoiIntelligenceUser>(plant).expect("POI learner");
	assert!(learner.interests.contains(personal));
	assert!(!learner.interests.contains(PoiKind::new("mob/camp")));
}

#[test]
fn bound_packmates_keep_group_mitigation() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(17),
		vec![
			RosterMember::new(Personality::Brawler, Vec3::X),
			RosterMember::new(Personality::Brawler, Vec3::NEG_X),
		],
	);
	let first = world.spawn((Transform::from_xyz(1.0, 0.9, 0.0), MobSlot(0), MobId(17))).id();
	let second = world.spawn((Transform::from_xyz(-1.0, 0.9, 0.0), MobSlot(1), MobId(17))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();

	let first_affiliations = world.get::<Affiliations>(first).expect("first affiliations");
	let second_affiliations = world.get::<Affiliations>(second).expect("second affiliations");
	assert!(first_affiliations.memberships.contains_key(&PACK));
	assert!(second_affiliations.memberships.contains_key(&PACK));
	assert!(first_affiliations.known_allies.contains_key(&PACK));
	assert!(second_affiliations.known_allies.contains_key(&PACK));
	assert_eq!(first_affiliations.threat_weight(second_affiliations, 0.0), 0.0);
	assert_eq!(second_affiliations.threat_weight(first_affiliations, 0.0), 0.0);
	assert_eq!(
		world.get::<ThreatSubject>(first).map(|subject| subject.id),
		Some(ThreatId(first.to_bits()))
	);
	assert_eq!(
		world.get::<ThreatSubject>(second).map(|subject| subject.id),
		Some(ThreatId(second.to_bits()))
	);
	assert!(world.get::<MobRoster>(host).is_some_and(|roster| {
		roster.get(0).is_some_and(|member| member.entity == Some(first))
			&& roster.get(1).is_some_and(|member| member.entity == Some(second))
	}));
}

#[test]
fn bind_by_ancestor_when_the_plant_has_no_mob_id() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(2),
		vec![RosterMember::new(Personality::Civilian, Vec3::ZERO).with_armed(false)],
	);
	let root = world.spawn((MobLevelRoot, ChildOf(host))).id();
	let plant = world.spawn((Transform::default(), MobSlot(0), ChildOf(root))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	assert_eq!(world.get::<MemberOf>(plant).map(|membership| membership.mob), Some(host));
}

#[derive(Component)]
struct MobLevelRoot;

#[test]
fn occupied_slot_does_not_steal_a_live_member() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(3),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let first = world.spawn((Transform::default(), MobSlot(0), MobId(3))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	let second = world.spawn((Transform::default(), MobSlot(0), MobId(3))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	assert_eq!(world.get::<MemberOf>(first).map(|membership| membership.mob), Some(host));
	assert!(world.get::<MemberOf>(second).is_none());
}

#[test]
fn writeback_clears_a_culled_pointer_without_respawn() {
	let mut world = World::new();
	let pose = Vec3::new(8.0, 0.9, -2.0);
	let host = spawn_host(
		&mut world,
		MobId(4),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	world.entity_mut(host).insert(MobRespawn {
		delay_secs: 1.0,
		max_replacements: Some(3),
		..MobRespawn::default()
	});
	let plant = world.spawn((Transform::from_translation(pose), MobSlot(0), MobId(4))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world.entity_mut(plant).insert(Health::from_max(40.0));
	world.run_system_once(write_back_mob_roster).expect("writeback");
	assert!(world.get::<MobRoster>(host).is_some_and(|roster| roster.get(0).is_some_and(
		|member| {
			(member.pose - pose).length() < 1e-4 && (member.health.current - 40.0).abs() < 1e-4
		}
	)));

	world.entity_mut(plant).despawn();
	world.run_system_once(write_back_mob_roster).expect("writeback");
	let member = world.get::<MobRoster>(host).and_then(|roster| roster.get(0)).expect("slot");
	assert!(member.entity.is_none());
	assert!(member.respawn_at.is_none());
	assert_eq!(member.replacements_used, 0);
}

#[test]
fn downed_schedules_a_replacement_and_queues_despawn() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let pose = Vec3::new(8.0, 0.9, -2.0);
	let host = spawn_host(
		&mut world,
		MobId(14),
		vec![RosterMember::new(Personality::Grazer, pose).with_armed(false)],
	);
	world.entity_mut(host).insert(MobRespawn {
		delay_secs: 1.0,
		max_replacements: Some(1),
		corpse_secs: 0.5,
		..MobRespawn::default()
	});
	let plant = world.spawn((Transform::from_translation(pose), MobSlot(0), MobId(14))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world
		.entity_mut(plant)
		.insert((Health::from_max(40.0), damage::Downed { source: None, point: pose, at: 0.0 }));
	world.run_system_once(queue_downed_member_deaths).expect("downed");
	world.flush();
	let member = world.get::<MobRoster>(host).and_then(|roster| roster.get(0)).expect("slot");
	assert!(member.respawn_at.is_some());
	assert_eq!(member.replacements_used, 1);
	assert!((member.health.current - 40.0).abs() < 1e-4);
	assert!(world.get::<damage::DespawnAfter>(plant).is_some());
}

#[test]
fn cull_does_not_respawn_when_the_policy_is_missing() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(15),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	world.entity_mut(host).remove::<MobRespawn>();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(15))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world.init_resource::<Time>();
	world
		.entity_mut(plant)
		.insert(damage::Downed { source: None, point: Vec3::ZERO, at: 0.0 });
	world.run_system_once(queue_downed_member_deaths).expect("downed");
	world.flush();
	let member = world.get::<MobRoster>(host).and_then(|roster| roster.get(0)).expect("slot");
	assert!(member.respawn_at.is_none());
	assert_eq!(member.replacements_used, 0);
	assert!(world.get::<damage::DespawnAfter>(plant).is_some());
}

#[test]
fn downed_still_queues_despawn_when_the_roster_pointer_is_stale() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_host(
		&mut world,
		MobId(16),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	world
		.entity_mut(host)
		.insert(MobRespawn { corpse_secs: 0.5, ..MobRespawn::default() });
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(16))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	if let Some(mut roster) = world.get_mut::<MobRoster>(host) {
		if let Some(member) = roster.get_mut(0) {
			member.entity = None;
		}
	}
	world.entity_mut(plant).insert((
		Health::from_max(40.0),
		damage::Downed { source: None, point: Vec3::ZERO, at: 0.0 },
	));
	world.run_system_once(queue_downed_member_deaths).expect("downed");
	world.flush();
	assert!(world.get::<damage::DespawnAfter>(plant).is_some());
	let member = world.get::<MobRoster>(host).and_then(|roster| roster.get(0)).expect("slot");
	assert!(member.respawn_at.is_none());
}

#[test]
fn respawn_emits_at_the_host_with_full_health() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins).add_plugins(MobIntelligencePlugin);
	let last = Vec3::new(1.0, 0.9, 4.0);
	let host_at = Vec3::new(3.0, 0.0, 1.0);
	let host = spawn_mob(
		&mut app.world_mut().commands(),
		Transform::from_translation(host_at),
		MobInstall::new(MobId(5), 8.0, vec![RosterMember::new(Personality::Grazer, last)])
			.with_respawn(MobRespawn {
				delay_secs: 0.0,
				max_replacements: Some(1),
				at: MobRespawnAt::Host,
				..MobRespawn::default()
			}),
	);
	app.world_mut().flush();
	let plant = app
		.world_mut()
		.spawn((Transform::from_translation(last), MobSlot(0), MobId(5)))
		.id();
	app.world_mut().run_system_once(bind_mob_members).expect("bind");
	app.world_mut().flush();
	app.world_mut().entity_mut(plant).insert((
		Health { current: 0.0, max: 40.0 },
		damage::Downed { source: None, point: last, at: 0.0 },
	));
	app.world_mut().run_system_once(queue_downed_member_deaths).expect("downed");
	app.world_mut().flush();
	app.world_mut().entity_mut(plant).despawn();
	app.world_mut().run_system_once(write_back_mob_roster).expect("writeback");
	app.update();
	let events: Vec<_> =
		app.world_mut().resource_mut::<Messages<MobMemberNeeded>>().drain().collect();
	assert_eq!(events.len(), 1);
	assert_eq!(events[0].mob, host);
	assert_eq!(events[0].slot, 0);
	assert!((events[0].pose.x - host_at.x).abs() < 1e-4);
	assert!((events[0].pose.z - host_at.z).abs() < 1e-4);
	assert!((events[0].pose.y - last.y).abs() < 1e-4);
	let member = app
		.world()
		.get::<MobRoster>(host)
		.and_then(|roster| roster.get(0))
		.expect("slot");
	assert!((member.health.current - member.health.max).abs() < 1e-4);
	app.update();
	let again: Vec<_> =
		app.world_mut().resource_mut::<Messages<MobMemberNeeded>>().drain().collect();
	assert!(again.is_empty());
}

#[test]
fn last_pose_respawn_keeps_the_death_site() {
	assert_eq!(
		crate::lifecycle::spawn_pose(MobRespawnAt::LastPose, Vec3::X * 9.0, Vec3::Z * 2.0),
		Vec3::Z * 2.0
	);
}

#[test]
fn replacements_count_after_the_original() {
	assert!(!MobRespawn::never().allows(0));
	let one = MobRespawn { max_replacements: Some(1), ..MobRespawn::default() };
	assert!(one.allows(0));
	assert!(!one.allows(1));
	assert!(MobRespawn::default().allows(8));
}

#[test]
fn travel_steps_the_host_across_the_ground() {
	let at = crate::travel::step_xz(Vec3::new(3.0, 1.0, 1.0), Vec3::new(10.0, 0.0, 1.0), 5.0);
	assert!((at.x - 8.0).abs() < 1e-3);
	assert!((at.y - 1.0).abs() < 1e-3);
	assert!((at.z - 1.0).abs() < 1e-3);
	let arrived = crate::travel::step_xz(Vec3::X * 9.0, Vec3::X * 10.0, 4.0);
	assert!((arrived.x - 10.0).abs() < 1e-4);
}

#[test]
fn travel_lerps_along_a_routing_chord() {
	let at = crate::travel::step_chord(Vec3::ZERO, Vec3::new(6.0, 8.0, 0.0), 5.0);
	assert!((at - Vec3::new(3.0, 4.0, 0.0)).length() < 1e-3);
}

#[test]
fn journeying_host_is_a_routing_user() {
	let mut world = World::new();
	let host = spawn_mob(
		&mut world.commands(),
		Transform::from_xyz(3.0, 0.0, 1.0),
		MobInstall::new(MobId(21), 12.0, vec![RosterMember::new(Personality::Grazer, Vec3::X)])
			.with_journey(true)
			.with_travel(crate::MobTravel::new(2.0)),
	);
	world.flush();
	assert!(world.get::<RoutingIntelligenceUser>(host).is_some());
	assert!(world.get::<journeying_intelligence::JourneyingIntelligenceUser>(host).is_some());
}

#[test]
fn poi_goal_hands_the_host_to_routing() {
	let mut world = World::new();
	let host = spawn_mob(
		&mut world.commands(),
		Transform::from_xyz(3.0, 0.0, 1.0),
		MobInstall::new(MobId(22), 12.0, vec![RosterMember::new(Personality::Grazer, Vec3::X)])
			.with_journey(true)
			.with_travel(crate::MobTravel::new(2.0)),
	);
	world.flush();
	let dest = Vec3::new(40.0, 6.0, 0.0);
	world.entity_mut(host).insert(PoiGoal::new(
		1,
		PoiId(9),
		None,
		PoiKind::new("mob/camp"),
		dest,
		8.0,
		0.0,
		0.0,
	));
	world.run_system_once(drive_poi_goals).expect("drive");
	let routing = world.get::<RoutingIntelligenceUser>(host).expect("routing");
	assert_eq!(routing.destination, Some(dest));
}

#[test]
fn travel_follows_a_routing_hop_including_y() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_mob(
		&mut world.commands(),
		Transform::from_xyz(0.0, 1.0, 0.0),
		MobInstall::new(MobId(23), 12.0, vec![RosterMember::new(Personality::Grazer, Vec3::X)])
			.with_journey(true)
			.with_travel(crate::MobTravel::new(10.0)),
	);
	world.flush();
	let dest = Vec3::new(80.0, 5.0, 0.0);
	world.entity_mut(host).insert(PoiGoal::new(
		1,
		PoiId(9),
		None,
		PoiKind::new("mob/camp"),
		dest,
		8.0,
		0.0,
		0.0,
	));
	{
		let mut routing = world.get_mut::<RoutingIntelligenceUser>(host).expect("routing");
		routing.destination = Some(dest);
		routing.plan = RoutePlan {
			layers: vec![LayerPlan {
				segment: 40.0,
				waypoints: vec![Vec3::new(0.0, 1.0, 0.0), Vec3::new(40.0, 4.0, 0.0), dest],
			}],
		};
	}
	world.resource_mut::<Time>().advance_by(std::time::Duration::from_secs_f32(0.5));
	world.run_system_once(crate::travel::travel_mobs).expect("travel");
	let at = world.get::<Transform>(host).expect("transform").translation;
	assert!(at.x > 0.5);
	assert!(at.y > 1.0);
}

#[test]
fn preinstalled_mixer_is_retargeted_to_the_host() {
	let mut world = World::new();
	let host = spawn_host(
		&mut world,
		MobId(8),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let decoy = world.spawn_empty().id();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(8))).id();
	Personality::Grazer.install(
		&mut world.commands(),
		plant,
		NpcInstall { tether: Some(decoy), armed: false, ..NpcInstall::default() },
	);
	world.flush();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective.subject()),
		Some(subject) if subject == host
	));
}

#[test]
fn lock_retargets_member_tethers_to_the_destination() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_host(
		&mut world,
		MobId(10),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let dest = world.spawn((Transform::from_xyz(20.0, 0.0, 0.0), Tether)).id();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(10))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world
		.entity_mut(host)
		.insert(MobTetherLock { subject: dest, generation: 1, until: 8.0 });
	world.run_system_once(apply_mob_tether_subjects).expect("apply");
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective.subject()),
		Some(subject) if subject == dest
	));
}

#[test]
fn lock_gives_each_slot_a_distinct_standoff() -> anyhow::Result<()> {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_host(
		&mut world,
		MobId(30),
		vec![
			RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false),
			RosterMember::new(Personality::Grazer, Vec3::X).with_armed(false),
			RosterMember::new(Personality::Grazer, Vec3::Z).with_armed(false),
		],
	);
	let dest = world.spawn((Transform::from_xyz(20.0, 0.0, 0.0), Tether)).id();
	let plants = [
		world.spawn((Transform::default(), MobSlot(0), MobId(30))).id(),
		world.spawn((Transform::default(), MobSlot(1), MobId(30))).id(),
		world.spawn((Transform::default(), MobSlot(2), MobId(30))).id(),
	];
	assert!(world.run_system_once(bind_mob_members).is_ok());
	world.flush();
	world
		.entity_mut(host)
		.insert(MobTetherLock { subject: dest, generation: 1, until: 8.0 });
	assert!(world.run_system_once(apply_mob_tether_subjects).is_ok());
	let radii: Vec<f32> = plants
		.into_iter()
		.filter_map(|plant| {
			world
				.get::<TetherIntelligenceUser>(plant)
				.and_then(|user| match user.objective {
					TetherObjective::Tether(subject, radius) if subject == dest => Some(radius),
					_ => None,
				})
		})
		.collect();
	assert_eq!(radii, vec![6.0, 8.0, 10.0]);
	Ok(())
}

#[test]
fn expired_lock_restores_the_host_subject() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let host = spawn_host(
		&mut world,
		MobId(11),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let dest = world.spawn((Transform::from_xyz(20.0, 0.0, 0.0), Tether)).id();
	let plant = world.spawn((Transform::default(), MobSlot(0), MobId(11))).id();
	world.run_system_once(bind_mob_members).expect("bind");
	world.flush();
	world
		.entity_mut(host)
		.insert(MobTetherLock { subject: dest, generation: 1, until: -1.0 });
	world.run_system_once(expire_mob_tether_locks).expect("expire");
	world.flush();
	world.run_system_once(apply_mob_tether_subjects).expect("apply");
	assert!(world.get::<MobTetherLock>(host).is_none());
	assert!(matches!(
		world.get::<TetherIntelligenceUser>(plant).map(|user| user.objective.subject()),
		Some(subject) if subject == host
	));
}

#[test]
fn arrival_locks_once_until_the_host_leaves() {
	let mut world = World::new();
	world.init_resource::<Time>();
	let at = Vec3::new(6.0, 0.0, 2.0);
	let host = spawn_host(
		&mut world,
		MobId(12),
		vec![RosterMember::new(Personality::Grazer, Vec3::ZERO).with_armed(false)],
	);
	let dest = world.spawn((Transform::from_translation(at), Tether)).id();
	world.entity_mut(host).insert((
		GlobalTransform::from_translation(at),
		PoiGoal::new(3, PoiId(40), Some(dest), PoiKind::new("mob/camp"), at, 4.0, 0.0, 2.5),
	));
	world.run_system_once(lock_mobs_on_poi_arrival).expect("lock");
	world.flush();
	assert!(world
		.get::<MobTetherLock>(host)
		.is_some_and(|lock| lock.subject == dest && lock.generation == 3));

	world
		.entity_mut(host)
		.insert(crate::lock::MobTetherLockMemory { generation: 3 });
	world.entity_mut(host).remove::<MobTetherLock>();
	world.run_system_once(lock_mobs_on_poi_arrival).expect("no-relock");
	world.flush();
	assert!(world.get::<MobTetherLock>(host).is_none());

	world
		.entity_mut(host)
		.insert(GlobalTransform::from_translation(Vec3::new(80.0, 0.0, 0.0)));
	world.run_system_once(forget_mob_tether_lock_when_leaving).expect("leave");
	world.flush();
	assert!(world.get::<crate::lock::MobTetherLockMemory>(host).is_none());
}
