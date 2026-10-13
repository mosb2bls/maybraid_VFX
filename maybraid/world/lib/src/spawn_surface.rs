//! Ground newly spawned agents on composed Durham height.

use bevy::prelude::*;
use characters::LocomotionCapsule;
use durham::DurhamSurface;
use mob_intelligence::MobSlot;
use mob_scenes::MobSceneSystems;

pub(crate) struct SpawnSurfacePlugin;

impl Plugin for SpawnSurfacePlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			lift_new_mob_members_above_composed
				.after(MobSceneSystems::Respawn)
				.in_set(MobSceneSystems::Surface),
		);
	}
}

/// POI respawn copies registry Y and a small capsule bump. When that Y is
/// stale or zero, the body sits under composed ground. Lift only — already
/// standing members keep their pose.
fn lift_new_mob_members_above_composed(
	surface: DurhamSurface,
	mut members: Query<&mut Transform, Added<MobSlot>>,
) {
	let clearance = LocomotionCapsule::HUMANOID.spawn_height();
	for mut transform in &mut members {
		transform.translation =
			surface.lift_origin_above_composed(transform.translation, clearance);
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{BaseTerrainNoise, Terrain, TerrainCellLayout, TerrainConfig, WorldBaseTerrain};
	use lod::gen::Id;
	use lod::hcsg::HcsgStorage;
	use std::sync::Arc;

	fn surface_world(publish_origin: bool) -> World {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let storage = HcsgStorage::default();
		if publish_origin {
			let terrain = Terrain::base_cell_for_test(&layout, 0, 0, base.clone());
			let cell = terrain.cell;
			storage.publish(Id::from_cell(cell), Arc::new(terrain), cell);
		}
		world.insert_resource(storage);
		world.insert_resource(layout);
		world.insert_resource(WorldBaseTerrain(base));
		world
	}

	#[test]
	fn new_mob_members_lift_onto_composed_height() -> anyhow::Result<()> {
		let mut world = surface_world(true);
		let buried = world.spawn((Transform::from_xyz(8.0, 4.0, 8.0), MobSlot(0))).id();
		world
			.run_system_once(lift_new_mob_members_above_composed)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let mut state = bevy::ecs::system::SystemState::<DurhamSurface>::new(&mut world);
		let surface = state.get(&world)?;
		let ground = surface.composed_height_at(Vec2::new(8.0, 8.0)).expect("published cell");
		drop(state);

		let raised = world.get::<Transform>(buried).expect("body").translation;
		assert_eq!(raised.xz(), Vec2::new(8.0, 8.0));
		assert!((raised.y - (ground + LocomotionCapsule::HUMANOID.spawn_height())).abs() < 1e-4);
		assert!(raised.y > 4.0);
		Ok(())
	}

	#[test]
	fn unpublished_cells_keep_the_authored_mob_pose() -> anyhow::Result<()> {
		let mut world = surface_world(false);
		let pose = Vec3::new(8.0, 4.0, 8.0);
		let member = world.spawn((Transform::from_translation(pose), MobSlot(0))).id();
		world
			.run_system_once(lift_new_mob_members_above_composed)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world.get::<Transform>(member).map(|transform| transform.translation),
			Some(pose)
		);
		Ok(())
	}
}
