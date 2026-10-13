//! Avian pitch apply + jump suspend for the vegetation player and NPC capsules.

use bevy::ecs::query::Has;
use bevy::prelude::*;
use characters::{
	apply_terrain_pitch, ApplyTerrainPitch, CharacterHeading, SuspendTerrainPitch, TerrainPitch,
};
use ground_avian::AvianElevationProbe;
use player::{CharacterController, Jumping};
use world_player::{Jumping as VegetationJumping, Player};

pub(crate) fn sync_suspend_terrain_pitch(
	mut commands: Commands,
	players: Query<
		(Entity, Has<VegetationJumping>, Has<Jumping>, Has<SuspendTerrainPitch>),
		With<Player>,
	>,
	npcs: Query<
		(Entity, Has<Jumping>, Has<SuspendTerrainPitch>),
		(With<CharacterController>, Without<Player>),
	>,
) {
	for (entity, jumping, has) in players
		.iter()
		.map(|(entity, vegetation, player, has)| (entity, vegetation || player, has))
		.chain(npcs.iter())
	{
		set_suspend_marker(&mut commands, entity, jumping, has);
	}
}

fn set_suspend_marker(commands: &mut Commands, entity: Entity, want: bool, has: bool) {
	match (want, has) {
		(true, false) => {
			commands.entity(entity).insert(SuspendTerrainPitch);
		}
		(false, true) => {
			commands.entity(entity).remove::<SuspendTerrainPitch>();
		}
		_ => {}
	}
}

/// Concrete system: [`AvianElevationProbe`] is a [`bevy::ecs::system::SystemParam`].
pub(crate) fn apply_avian_terrain_pitch(
	time: Res<Time>,
	probe: AvianElevationProbe,
	visuals: Query<
		(Entity, &mut Transform, &GlobalTransform, &mut TerrainPitch, &mut CharacterHeading),
		With<ApplyTerrainPitch>,
	>,
	child_of: Query<&ChildOf>,
	parents: Query<&GlobalTransform, Without<ApplyTerrainPitch>>,
	suspended: Query<(), With<SuspendTerrainPitch>>,
) {
	apply_terrain_pitch(time, probe, visuals, child_of, parents, suspended);
}
