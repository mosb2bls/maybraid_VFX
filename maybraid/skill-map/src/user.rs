//! Capsule install for Discover skill maps, following [`firearm_user::FirearmUser`].

use bevy::prelude::*;
use character_items::SkillMapSpec;

use crate::cursor::{CURSOR_SPEED, FLICK_REGION, WATER_LOCK_SECS};

/// Which map the session should present. Index 0 of the bag queue.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillMapEquip {
	pub spec: Option<SkillMapSpec>,
}

impl SkillMapEquip {
	pub fn from_spec(spec: Option<SkillMapSpec>) -> Self {
		Self { spec }
	}
}

/// Capsule using Discover skill maps.
///
/// This is a 1:1 Bevy relationship onto the map session (`maps`). Inserting it
/// stamps [`MappedBy`] on the session, so despawn/replace stays consistent and
/// queries can go either way.
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = MappedBy)]
pub struct SkillMapUser {
	#[relationship]
	pub maps: Entity,
	pub settings: SkillMapUserSettings,
}

impl SkillMapUser {
	pub fn mapping(maps: Entity) -> Self {
		Self { maps, settings: SkillMapUserSettings::default() }
	}
}

/// Per-user steer knobs. Map grammar lives on the session, not here.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SkillMapUserSettings {
	pub cursor_speed: f32,
	pub flick_scale: f32,
	pub water_lock_secs: f32,
}

impl Default for SkillMapUserSettings {
	fn default() -> Self {
		Self {
			cursor_speed: CURSOR_SPEED,
			flick_scale: FLICK_REGION,
			water_lock_secs: WATER_LOCK_SECS,
		}
	}
}

/// Session-side 1:1 target of [`SkillMapUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = SkillMapUser)]
pub struct MappedBy(Entity);

impl MappedBy {
	pub fn user(&self) -> Entity {
		self.0
	}
}

/// World-posed map session a [`SkillMapUser`] walks. Cameras, tiles, and chrome
/// members point back here.
#[derive(Component, Debug, Default)]
pub struct SkillMapSession {
	pub cameras: std::collections::HashMap<crate::SkillMapId, Entity>,
	pub nodes: std::collections::HashMap<crate::SkillMapId, Entity>,
	pub presented: Option<SkillMapSpec>,
}

/// Entity that belongs to a [`SkillMapSession`].
#[derive(Component, Clone, Copy, Debug)]
pub struct SkillMapMember {
	pub user: Entity,
	pub session: Entity,
}

/// Remaining seconds this user cannot steer (water / debraid).
#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct SkillMapSteerLock {
	pub remaining: f32,
}

impl SkillMapSteerLock {
	pub fn locked(self) -> bool {
		self.remaining > 0.0
	}

	pub fn arm(&mut self, secs: f32) {
		self.remaining = secs.max(self.remaining);
	}
}

/// True while this user is holding **RB** and the map is enabled.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SkillMapHeld(pub bool);

/// Create the session and stamp [`SkillMapUser`] on `user`.
///
/// Chrome and tiles are filled later by [`crate::viewport::present_skill_maps`]
/// once render assets exist — same split as a held kit that poses after spawn.
pub fn spawn_skill_maps(commands: &mut Commands, user: Entity) -> Entity {
	spawn_skill_maps_with(commands, user, SkillMapUserSettings::default())
}

pub fn spawn_skill_maps_with(
	commands: &mut Commands,
	user: Entity,
	settings: SkillMapUserSettings,
) -> Entity {
	let maps = commands
		.spawn((Name::new("skill-map-session"), SkillMapSession::default()))
		.id();
	commands.entity(user).insert((
		SkillMapUser { maps, settings },
		SkillMapEquip::default(),
		SkillMapSteerLock::default(),
		SkillMapHeld(false),
	));
	maps
}

/// Sessions are world-posed, not parented. When the user is culled or despawned
/// without a drop path, [`MappedBy`] leaves and the maps would otherwise float.
pub fn despawn_orphaned_skill_maps(
	mut commands: Commands,
	sessions: Query<Entity, (With<SkillMapSession>, Without<MappedBy>)>,
	members: Query<(Entity, &SkillMapMember)>,
) {
	for session in &sessions {
		for (entity, member) in &members {
			if member.session == session {
				commands.entity(entity).try_despawn();
			}
		}
		commands.entity(session).try_despawn();
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;

	use super::{
		despawn_orphaned_skill_maps, spawn_skill_maps, SkillMapEquip, SkillMapSession, SkillMapUser,
	};

	#[test]
	fn spawn_stamps_the_user_and_session() {
		let mut world = World::new();
		let user = world.spawn_empty().id();
		let maps = world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		let stamped = world.get::<SkillMapUser>(user).expect("user");
		assert_eq!(stamped.maps, maps);
		assert!(world.get::<SkillMapSession>(maps).is_some());
		assert_eq!(world.get::<SkillMapEquip>(user).copied(), Some(SkillMapEquip::default()));
	}

	#[test]
	fn orphaned_session_despawns_when_the_user_is_gone() {
		let mut world = World::new();
		let user = world.spawn_empty().id();
		let maps = world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		world.despawn(user);
		world.flush();
		world.run_system_once(despawn_orphaned_skill_maps).expect("orphan");
		assert!(!world.entities().contains(maps));
	}

	#[test]
	fn session_stays_while_linked() {
		let mut world = World::new();
		let user = world.spawn_empty().id();
		let maps = world
			.run_system_once(move |mut commands: Commands| spawn_skill_maps(&mut commands, user))
			.expect("spawn");
		world.flush();
		world.run_system_once(despawn_orphaned_skill_maps).expect("linked");
		assert!(world.entities().contains(maps));
		assert!(world.entities().contains(user));
	}
}
