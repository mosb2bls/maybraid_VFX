//! Spawn High roster plants off the host transform tree; cull them with the stub.

use std::collections::HashMap;

use bevy::prelude::*;
use lod::{LodChunkCullSystems, LodSceneLevel};
use mob_characters::CharacterSceneRecipe;
use mob_intelligence::{ancestor_mob, MemberOf, Mob, MobId, MobSlot, RosterBinding, RosterRef};
use poi_intelligence::{NearbyPlace, AGENT_SEPARATION};

use crate::plugin::MobSceneSystems;
use crate::MobScene;

pub type MemberRosterRef = RosterRef<CharacterSceneRecipe>;

#[derive(Resource, Default)]
struct RosterStubBodies(HashMap<Entity, RosterBinding>);

pub(crate) fn configure_roster_ref_systems(app: &mut App) {
	app.init_resource::<RosterStubBodies>()
		.add_systems(
			Update,
			(fulfill_roster_refs, record_roster_bindings)
				.chain()
				.in_set(MobSceneSystems::Fulfill),
		)
		.add_systems(
			Update,
			(despawn_roster_bodies_for_removed_stubs, cull_members_when_high_drops)
				.chain()
				.after(LodChunkCullSystems::Drain),
		);
}

fn fulfill_roster_refs(
	mut commands: Commands,
	stubs: Query<(Entity, &MemberRosterRef), Without<RosterBinding>>,
	child_of: Query<&ChildOf>,
	mobs: Query<(), With<Mob>>,
	hosts: Query<
		(Entity, &MobId, &Transform, Option<&GlobalTransform>, Option<&MobScene>),
		With<Mob>,
	>,
	live: Query<(&MemberOf, &Transform)>,
) {
	let mut placed: Vec<(Entity, Vec3)> = Vec::new();
	for (stub, roster) in &stubs {
		let Some(host) = ancestor_mob(stub, &child_of, &mobs) else {
			continue;
		};
		let Ok((host, id, transform, global, scene)) = hosts.get(host) else {
			continue;
		};
		let preferred = global
			.map(|global| global.transform_point(roster.offset))
			.unwrap_or_else(|| transform.transform_point(roster.offset));
		let occupied: Vec<Vec3> = live
			.iter()
			.filter(|(member, _)| member.mob == host)
			.map(|(_, transform)| transform.translation)
			.chain(placed.iter().filter(|(placed_host, _)| *placed_host == host).map(|(_, at)| *at))
			.collect();
		let cleared = NearbyPlace { position: preferred, poi: None }
			.clear_of(
				transform.translation,
				&occupied,
				AGENT_SEPARATION,
				u64::from(roster.slot).wrapping_add(1),
			)
			.position;
		let at = Vec3::new(cleared.x, preferred.y, cleared.z);
		let body = roster.recipe.spawn(&mut commands, Transform::from_translation(at));
		commands.entity(body).insert((MobSlot(roster.slot), *id));
		if let Some(kind) = scene.map(|scene| scene.mob.kind) {
			commands.entity(body).insert(kind);
		}
		commands.entity(stub).insert(RosterBinding { body, host, slot: roster.slot });
		placed.push((host, at));
	}
}

fn record_roster_bindings(
	added: Query<(Entity, &RosterBinding), Added<RosterBinding>>,
	mut live: ResMut<RosterStubBodies>,
) {
	for (stub, binding) in &added {
		live.0.insert(stub, *binding);
	}
}

fn despawn_roster_bodies_for_removed_stubs(
	mut removed: RemovedComponents<RosterBinding>,
	mut live: ResMut<RosterStubBodies>,
	members: Query<&MemberOf>,
	mut commands: Commands,
) {
	for stub in removed.read() {
		let Some(binding) = live.0.remove(&stub) else {
			continue;
		};
		let Ok(member) = members.get(binding.body) else {
			continue;
		};
		if member.mob == binding.host && member.slot == binding.slot {
			commands.entity(binding.body).try_despawn();
		}
	}
}

fn cull_members_when_high_drops(
	hosts: Query<(Entity, &LodSceneLevel), With<Mob>>,
	members: Query<(Entity, &MemberOf)>,
	mut commands: Commands,
) {
	for (host, level) in &hosts {
		if *level == LodSceneLevel::High {
			continue;
		}
		for (entity, membership) in &members {
			if membership.mob == host {
				commands.entity(entity).try_despawn();
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use std::sync::Arc;

	use crate::MobKind;

	fn host_and_stub(world: &mut World, offset: Vec3) -> (Entity, Entity) {
		let recipe = Arc::new(CharacterSceneRecipe::default());
		let mut commands = world.commands();
		let host = commands
			.spawn((
				Transform::from_xyz(10.0, 2.0, 4.0),
				GlobalTransform::from_translation(Vec3::new(10.0, 2.0, 4.0)),
				Mob::new(24.0),
				MobId(9),
			))
			.id();
		let stub = commands
			.spawn((MemberRosterRef::new(Arc::clone(&recipe), 0, offset), ChildOf(host)))
			.id();
		world.flush();
		(host, stub)
	}

	#[test]
	fn fulfill_spawns_a_world_body_off_the_host_tree() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let offset = Vec3::new(5.0, 1.0, 0.0);
		let (host, stub) = host_and_stub(&mut world, offset);
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let binding = world.get::<RosterBinding>(stub).expect("binding");
		assert_eq!(binding.host, host);
		assert!(world.get::<ChildOf>(binding.body).is_none());
		let at = world.get::<Transform>(binding.body).expect("pose").translation;
		assert!((at - Vec3::new(15.0, 3.0, 4.0)).length() < 1e-3);
		assert_eq!(world.get::<MobSlot>(binding.body), Some(&MobSlot(0)));
		assert_eq!(world.get::<MobId>(binding.body), Some(&MobId(9)));
		assert!(world.get::<CharacterSceneRecipe>(stub).is_none());
	}

	#[test]
	fn host_translation_does_not_tow_the_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (host, stub) = host_and_stub(&mut world, Vec3::new(5.0, 0.0, 0.0));
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		let before = world.get::<Transform>(body).expect("pose").translation;
		world.entity_mut(host).insert(Transform::from_xyz(100.0, 2.0, 4.0));
		world.flush();
		let after = world.get::<Transform>(body).expect("pose").translation;
		assert!((after - before).length() < 1e-4);
	}

	#[test]
	fn removing_the_stub_despawns_a_still_bound_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (host, stub) = host_and_stub(&mut world, Vec3::X);
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		world.entity_mut(body).insert(MemberOf { mob: host, slot: 0 });
		world.run_system_once(record_roster_bindings).expect("record");
		world.flush();
		world.entity_mut(stub).despawn();
		world.flush();
		world.run_system_once(despawn_roster_bodies_for_removed_stubs).expect("cull");
		world.flush();
		assert!(world.get_entity(body).is_err());
	}

	#[test]
	fn removing_the_stub_does_not_despawn_a_recycled_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let (_host, stub) = host_and_stub(&mut world, Vec3::X);
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		world.run_system_once(record_roster_bindings).expect("record");
		world.flush();
		world.entity_mut(stub).despawn();
		world.flush();
		world.run_system_once(despawn_roster_bodies_for_removed_stubs).expect("cull");
		world.flush();
		assert!(world.get_entity(body).is_ok());
		assert_eq!(world.get::<MobId>(body), Some(&MobId(9)));
	}

	#[test]
	fn fulfill_stamps_the_host_mob_kind_on_the_body() {
		let mut world = World::new();
		world.init_resource::<RosterStubBodies>();
		let recipe = Arc::new(CharacterSceneRecipe::default());
		let mut commands = world.commands();
		let host = commands
			.spawn((
				Transform::from_xyz(10.0, 2.0, 4.0),
				GlobalTransform::from_translation(Vec3::new(10.0, 2.0, 4.0)),
				Mob::new(24.0),
				MobId(9),
				MobScene::of_kind(MobKind::Brawler, 1.0),
			))
			.id();
		let stub = commands
			.spawn((MemberRosterRef::new(Arc::clone(&recipe), 0, Vec3::X), ChildOf(host)))
			.id();
		world.flush();
		world.run_system_once(fulfill_roster_refs).expect("fulfill");
		world.flush();
		let body = world.get::<RosterBinding>(stub).expect("binding").body;
		assert_eq!(world.get::<MobKind>(body), Some(&MobKind::Brawler));
	}

	#[test]
	fn leaving_high_despawns_unparented_members() {
		let mut world = World::new();
		let host = world.spawn((Mob::new(12.0), LodSceneLevel::High)).id();
		let body = world.spawn(MemberOf { mob: host, slot: 0 }).id();
		world.entity_mut(host).insert(LodSceneLevel::UltraLow);
		world.flush();
		world.run_system_once(cull_members_when_high_drops).expect("cull");
		world.flush();
		assert!(world.get_entity(body).is_err());
	}
}
