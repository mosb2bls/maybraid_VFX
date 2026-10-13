//! Avian pitch apply + copy jump-in-flight onto [`SuspendTerrainPitch`].

use bevy::ecs::query::Has;
use bevy::prelude::*;
use characters::{
	apply_terrain_pitch, ApplyTerrainPitch, CharacterHeading, SuspendTerrainPitch, TerrainPitch,
};
use ground_avian::AvianElevationProbe;

use crate::player::{Jumping, Player};

pub(crate) fn sync_suspend_terrain_pitch(
	mut commands: Commands,
	players: Query<(Entity, Has<Jumping>, Has<SuspendTerrainPitch>), With<Player>>,
) {
	for (entity, jumping, has) in &players {
		match (jumping, has) {
			(true, false) => {
				commands.entity(entity).insert(SuspendTerrainPitch);
			}
			(false, true) => {
				commands.entity(entity).remove::<SuspendTerrainPitch>();
			}
			_ => {}
		}
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
