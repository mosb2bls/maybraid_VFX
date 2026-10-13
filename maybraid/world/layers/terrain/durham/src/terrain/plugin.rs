//! Idempotent plugin for the Durham terrain model.

use crate::terrain::cell::TerrainCellLayout;
use crate::terrain::collider::{
	queue_terrain_trimesh_colliders, TerrainColliderSystems, TerrainFrictionConfig,
};
use crate::terrain::stamps::TerrainStampConfigs;
use crate::terrain::watersheds::WatershedConfigs;
use avian3d::prelude::PhysicsPlugins;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;

/// Registers Avian (if needed) and resources for the terrain model.
pub struct TerrainResourcesPlugin;

impl Default for TerrainResourcesPlugin {
	fn default() -> Self {
		Self
	}
}

/// Idempotent registration of [`TerrainResourcesPlugin`].
pub fn register_terrain_plugin(app: &mut App) {
	if app.is_plugin_added::<TerrainResourcesPlugin>() {
		return;
	}
	app.add_plugins(TerrainResourcesPlugin);
}

impl Plugin for TerrainResourcesPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		// [`lod::hcsg`] presentation/generation plugins own `HcsgStorage` init.
		app.init_resource::<TerrainCellLayout>()
			.init_resource::<TerrainStampConfigs>()
			.init_resource::<WatershedConfigs>()
			.init_resource::<TerrainFrictionConfig>()
			.configure_sets(
				Update,
				(
					TerrainColliderSystems::SyncOverlays,
					TerrainColliderSystems::SyncHosts,
					TerrainColliderSystems::QueueMeshes,
				)
					.chain(),
			)
			.add_systems(
				Update,
				queue_terrain_trimesh_colliders
					.in_set(TerrainColliderSystems::QueueMeshes)
					.in_set(
						terrain_layer_model::TerrainLayerSystems::<crate::Durham>::QueueColliders,
					),
			);
	}
}
