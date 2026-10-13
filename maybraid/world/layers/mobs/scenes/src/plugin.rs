//! Materialize mob hosts, death replacements, and pack pursuit.

use bevy::prelude::*;
use damage::Health;
use journeying_intelligence::JourneyingIntelligencePlugin;
use lod::{
	add_lod_refresh_chunk_for, add_lod_refresh_chunk_full_for, LodChunkFulfillSystems,
	LodSceneLevel,
};
use mob_characters::{CharacterSceneSystems, MobCharacterScenesPlugin};
use mob_intelligence::{
	install_mob, MobIdAlloc, MobInstall, MobIntelligencePlugin, MobMemberNeeded, MobSlot,
	MobSystems, MobTetherLock, RosterMember,
};
use poi_intelligence::{PoiGoal, PoiId, PoiIntelligencePlugin, PoiKind};
use routing_intelligence::RoutingPlugin;
use tether_intelligence::TetherPlugin;

use crate::{MobKind, MobScene};

const PREY_POI: PoiKind = PoiKind::new("mobs/prey");
const PACK_LOCK_SECS: f32 = 45.0;
const PACK_BROWSE_SECS: f32 = 8.0;
const PACK_ARRIVAL_RADIUS: f32 = 12.0;
const RESPAWN_RETRY_SECS: f32 = 1.0;

#[derive(Component, Clone, Copy, Debug)]
struct PackPursuit {
	browse_until: f32,
	generation: u64,
}

impl Default for PackPursuit {
	fn default() -> Self {
		Self { browse_until: 0.0, generation: 1 }
	}
}

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MobSceneSystems {
	Install,
	Fulfill,
	Pursuit,
	Respawn,
	Surface,
	Center,
}

#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MobLodRefreshMode {
	#[default]
	FullScan,
	Indexed,
}

pub struct MobScenesPlugin;

impl Plugin for MobScenesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MobCharacterScenesPlugin>() {
			app.add_plugins(MobCharacterScenesPlugin);
		}
		if !app.is_plugin_added::<MobIntelligencePlugin>() {
			app.add_plugins(MobIntelligencePlugin);
		}
		if !app.is_plugin_added::<damage::DamagePlugin>() {
			app.add_plugins(damage::DamagePlugin);
		}
		if !app.is_plugin_added::<PoiIntelligencePlugin>() {
			app.add_plugins(PoiIntelligencePlugin);
		}
		if !app.is_plugin_added::<JourneyingIntelligencePlugin>() {
			app.add_plugins(JourneyingIntelligencePlugin);
		}
		if !app.is_plugin_added::<TetherPlugin>() {
			app.add_plugins(TetherPlugin);
		}
		if !app.is_plugin_added::<RoutingPlugin>() {
			app.add_plugins(RoutingPlugin);
		}
		match app.world().get_resource::<MobLodRefreshMode>().copied().unwrap_or_default() {
			MobLodRefreshMode::FullScan => add_lod_refresh_chunk_full_for::<MobScene>(app),
			MobLodRefreshMode::Indexed => add_lod_refresh_chunk_for::<MobScene>(app),
		}
		app.configure_sets(
			Update,
			(
				MobSceneSystems::Install,
				MobSceneSystems::Fulfill,
				MobSceneSystems::Pursuit,
				MobSceneSystems::Respawn,
			)
				.chain(),
		)
		.configure_sets(
			Update,
			MobSceneSystems::Fulfill
				.after(LodChunkFulfillSystems::Drain)
				.before(CharacterSceneSystems::Materialize)
				.before(MobSystems::Bind),
		)
		.configure_sets(
			Update,
			(MobSceneSystems::Surface, MobSceneSystems::Center)
				.chain()
				.after(MobSystems::Travel),
		)
		.add_systems(
			Update,
			install_mob_scenes
				.in_set(MobSceneSystems::Install)
				.before(CharacterSceneSystems::Materialize)
				.before(MobSystems::Bind),
		)
		.add_systems(
			Update,
			(start_pack_browse, packs_track_herds)
				.chain()
				.in_set(MobSceneSystems::Pursuit)
				.before(MobSystems::Travel),
		)
		.add_systems(
			Update,
			spawn_needed_members.in_set(MobSceneSystems::Respawn).after(MobSystems::Respawn),
		)
		.add_systems(Update, sync_mob_scene_centers.in_set(MobSceneSystems::Center));
		crate::roster_ref::configure_roster_ref_systems(app);
		crate::ambient::configure_herd_ambient(app);
	}
}

fn install_mob_scenes(
	mut commands: Commands,
	mut ids: ResMut<MobIdAlloc>,
	hosts: Query<(Entity, &MobScene, &Transform), Added<MobScene>>,
) {
	for (host, scene, transform) in &hosts {
		let id = ids.allocate();
		let members = scene
			.mob
			.roster
			.members
			.iter()
			.enumerate()
			.map(|(slot, member)| {
				let recipe = &member.character;
				let mut roster = RosterMember::new(
					recipe.brains.personality(recipe.armed()),
					transform.translation + member.offset,
				)
				.with_armed(recipe.armed())
				.with_keep_tether_in_combat(Some(recipe.brains.keep_tether_in_combat()))
				.with_interests(recipe.brains.interests_for_slot(slot));
				roster.health = Health::from_max(f32::from(recipe.sheet().health));
				roster
			})
			.collect();
		let brain = &scene.mob.intelligence;
		let mut install = MobInstall::new(id, brain.leash, members)
			.with_interests(brain.interests.clone())
			.with_affiliations(brain.affiliations.clone())
			.with_respawn(brain.respawn)
			.with_journey(brain.journey);
		if let Some(travel) = brain.travel {
			install = install.with_travel(travel);
		}
		install_mob(&mut commands, host, install);
		commands.entity(host).insert(brain.clone());
		if scene.mob.kind == MobKind::Pack {
			commands.entity(host).insert(PackPursuit::default());
		}
	}
}

fn spawn_needed_members(
	time: Res<Time>,
	mut commands: Commands,
	mut needed: MessageReader<MobMemberNeeded>,
	mut mobs: Query<(&MobScene, Option<&LodSceneLevel>, &mut mob_intelligence::MobRoster)>,
) {
	let retry_at = time.elapsed_secs() + RESPAWN_RETRY_SECS;
	for request in needed.read() {
		let Ok((mob, level, mut roster)) = mobs.get_mut(request.mob) else {
			continue;
		};
		if level.is_some_and(|level| *level != LodSceneLevel::High) {
			if let Some(member) = roster.get_mut(request.slot) {
				member.spawn_requested = false;
				member.respawn_at = Some(retry_at);
			}
			continue;
		}
		let Some(member) = mob.mob.roster.members.get(request.slot as usize) else {
			continue;
		};
		let body = member.character.spawn(&mut commands, Transform::from_translation(request.pose));
		commands.entity(body).insert((MobSlot(request.slot), request.id, mob.mob.kind));
	}
}

fn sync_mob_scene_centers(mut hosts: Query<(&Transform, &mut MobScene), Changed<Transform>>) {
	for (transform, mut scene) in &mut hosts {
		scene.center = transform.translation;
	}
}

fn start_pack_browse(
	time: Res<Time>,
	mut released: RemovedComponents<MobTetherLock>,
	mut packs: Query<&mut PackPursuit>,
) {
	let now = time.elapsed_secs();
	for entity in released.read() {
		let Ok(mut pursuit) = packs.get_mut(entity) else {
			continue;
		};
		if pursuit.browse_until <= now {
			pursuit.browse_until = now + PACK_BROWSE_SECS;
		}
	}
}

fn packs_track_herds(
	time: Res<Time>,
	mut commands: Commands,
	hosts: Query<(Entity, &MobScene, &GlobalTransform)>,
	mut packs: Query<(Entity, Option<&MobTetherLock>, &mut PackPursuit)>,
	mut goals: Query<&mut PoiGoal>,
) {
	let herds: Vec<_> = hosts
		.iter()
		.filter(|(_, scene, _)| scene.mob.kind == MobKind::Herd)
		.map(|(entity, _, transform)| (entity, transform.translation()))
		.collect();
	let now = time.elapsed_secs();
	for (pack, lock, mut pursuit) in &mut packs {
		let Ok((_, _, pack_transform)) = hosts.get(pack) else {
			continue;
		};
		let pack_at = pack_transform.translation();
		let target = lock
			.and_then(|lock| herds.iter().copied().find(|(entity, _)| *entity == lock.subject))
			.or_else(|| {
				herds.iter().copied().min_by(|(_, a), (_, b)| {
					pack_at.distance_squared(*a).total_cmp(&pack_at.distance_squared(*b))
				})
			});
		let Some((prey, prey_at)) = target else {
			continue;
		};
		if lock.is_none() && pursuit.browse_until > now {
			if goals.get(pack).is_ok_and(|goal| goal.kind == PREY_POI) {
				commands.entity(pack).remove::<PoiGoal>();
			}
			continue;
		}
		if lock.is_none() && pursuit.browse_until > 0.0 {
			pursuit.browse_until = 0.0;
			pursuit.generation = pursuit.generation.saturating_add(1).max(1);
		}
		let point = Vec3::new(prey_at.x, pack_at.y, prey_at.z);
		if let Ok(mut goal) = goals.get_mut(pack) {
			goal.generation = pursuit.generation;
			goal.target = PoiId(prey.to_bits());
			goal.kind = PREY_POI;
			goal.poi_entity = Some(prey);
			goal.location.point = point;
			goal.location.radius = PACK_ARRIVAL_RADIUS;
			goal.linger_secs = PACK_LOCK_SECS;
		} else {
			commands.entity(pack).insert(PoiGoal::new(
				pursuit.generation,
				PoiId(prey.to_bits()),
				Some(prey),
				PREY_POI,
				point,
				PACK_ARRIVAL_RADIUS,
				now,
				PACK_LOCK_SECS,
			));
		}
	}
}

#[cfg(test)]
mod tests {
	use anyhow::Result;
	use bevy::ecs::system::RunSystemOnce;
	use npc_intelligence::Personality;

	use super::*;
	use mob_characters::CharacterSceneRecipe;
	use mob_intelligence::{MobId, MobMemberNeeded, MobRoster};

	#[test]
	fn plugin_update_schedule_is_acyclic() -> Result<()> {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
			.insert_resource(MobLodRefreshMode::Indexed)
			.add_plugins(MobScenesPlugin);
		app.world_mut()
			.schedule_scope(Update, |world, schedule| schedule.initialize(world))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn rejected_low_lod_respawn_is_retryable() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Messages<MobMemberNeeded>>();
		let host = world
			.spawn((
				MobScene::of_kind(MobKind::Herd, 0.2),
				Transform::default(),
				LodSceneLevel::UltraLow,
				MobRoster::new(vec![RosterMember::new(Personality::Grazer, Vec3::Y)]),
			))
			.id();
		{
			let mut roster = world
				.get_mut::<MobRoster>(host)
				.ok_or_else(|| anyhow::anyhow!("missing roster"))?;
			let member =
				roster.get_mut(0).ok_or_else(|| anyhow::anyhow!("missing roster member"))?;
			member.spawn_requested = true;
		}
		world.resource_mut::<Messages<MobMemberNeeded>>().write(MobMemberNeeded {
			mob: host,
			id: MobId(7),
			slot: 0,
			pose: Vec3::Y,
		});

		world
			.run_system_once(spawn_needed_members)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let member = world
			.get::<MobRoster>(host)
			.and_then(|roster| roster.get(0))
			.ok_or_else(|| anyhow::anyhow!("missing roster member after retry"))?;
		assert!(!member.spawn_requested);
		assert_eq!(member.respawn_at, Some(RESPAWN_RETRY_SECS));
		Ok(())
	}

	#[test]
	fn respawn_stamps_the_host_mob_kind_on_the_body() -> Result<()> {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.init_resource::<Messages<MobMemberNeeded>>();
		let host = world
			.spawn((
				MobScene::of_kind(MobKind::Raider, 0.2),
				Transform::default(),
				LodSceneLevel::High,
				MobRoster::new(vec![RosterMember::new(Personality::Assassin, Vec3::Y)]),
			))
			.id();
		world.resource_mut::<Messages<MobMemberNeeded>>().write(MobMemberNeeded {
			mob: host,
			id: MobId(7),
			slot: 0,
			pose: Vec3::Y,
		});

		world
			.run_system_once(spawn_needed_members)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let kind = world
			.query::<(&MobKind, &CharacterSceneRecipe)>()
			.iter(&world)
			.map(|(kind, _)| *kind)
			.next()
			.ok_or_else(|| anyhow::anyhow!("respawned body missing mob kind"))?;
		assert_eq!(kind, MobKind::Raider);
		Ok(())
	}

	#[test]
	fn moving_host_updates_the_mob_lod_center() -> Result<()> {
		let mut world = World::new();
		let at = Vec3::new(40.0, 12.0, -9.0);
		let host = world
			.spawn((MobScene::of_kind(MobKind::Pack, 0.4), Transform::from_translation(at)))
			.id();

		world
			.run_system_once(sync_mob_scene_centers)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(world.get::<MobScene>(host).map(|scene| scene.center), Some(at));
		Ok(())
	}
}
