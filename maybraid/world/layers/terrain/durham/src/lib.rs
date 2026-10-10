//! Durham terrain models: HCSG generation over [`lod::hcsg::HcsgStorage`], SDF meshing.
//!
//! Each model owns an idempotent plugin (e.g. [`terrain::TerrainResourcesPlugin`]). The
//! crate-root [`DurhamTerrainModelsPlugin`] composes those model plugins.

pub mod hcsg;
pub mod terrain;
pub mod water;

pub use hcsg::{
	register_durham_hcsg_session, BackgroundStream, DurhamPresentationPlugin, DurhamSurface,
	DurhamWindow, DurhamWorldPlugin, FarStream, NearStream, PlayableStreams, SharedTerrainStorage,
	StreamPresentationPlugin, StreamRing, Streamed, TerrainStream, WaterPresentationPlugin,
};
#[cfg(any(test, feature = "test-support"))]
pub use hcsg::SharedTerrainStorageTestExt;
/// Frame-side terrain reads use [`SharedTerrainStorage`]; storage itself is
/// [`lod::hcsg::HcsgStorage`].
pub use terrain::render::cascade_chunk_for_cell;
pub use terrain::{
	fine_patch_cell_layout, mesh_assets, playable_world_cell_layout, register_terrain_plugin,
	retarget_mesh_assets, stream_banded_draws, stream_banded_level, stream_banded_scene,
	terrain_collider_covers_xz, BaseTerrainNoise, CanyonHighPassControllerLayout,
	CanyonLowPassControllerLayout, CanyonStampCell, CascadeChunk, CellTiling, ComposedTerrain,
	Durham, DurhamRoots, GeographicBand, GeographicFamily, GeographicFeature, GeographicFeatureId,
	GeographicFeatureKind, MacroCellLayout, MassifHighPassControllerLayout,
	MassifLowPassControllerLayout, MassifStampCell, OuterCellRing, PlateauControllerLayout,
	PlateauHighPassControllerLayout, PlateauLowPassControllerLayout, PlateauStampCell,
	PocketWaterHighPassControllerLayout, PocketWaterLowPassControllerLayout, PocketWaterStampCell,
	PrePocketHighPassLayout, PrePocketLowPassLayout, PreWatershedTerrain,
	RollingHighPassControllerLayout, RollingLowPassControllerLayout, RollingStampCell,
	StreamBandedLod, Terrain, TerrainCellLayout, TerrainCellRing, TerrainColliderMeshSource,
	TerrainColliderSystems, TerrainConfig, TerrainCoverage, TerrainFrictionConfig,
	TerrainHeightSnapshot, TerrainLayoutPinned, TerrainMeshAssets, TerrainMeshBuilder,
	TerrainMeshLodBand, TerrainPresentPending, TerrainPresentationDirty, TerrainRenderItem,
	TerrainResourcesPlugin, TerrainRetarget, TerrainSdf, TerrainStampConfigs,
	TerrainTrimeshCollider, ValleyHighPassControllerLayout, ValleyLowPassControllerLayout,
	ValleyStampCell, WaterSurfaceSnapshot, WatershedBandPass, WatershedConfigs,
	WatershedLeafBounds, WatershedLeafKind, WorldBaseTerrain, MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
	TERRAIN_FRICTION, WORLD_FINE_HALF_EXTENT_CELLS, WORLD_OUTER_2X_ROWS, WORLD_OUTER_4X_ROWS,
};
pub use water::{
	register_water_plugin, ComposedWater, Water, WaterColumn, WaterMeshAssets, WaterPlugin,
};

use bevy::prelude::*;

/// Crate-level composition of Durham model plugins.
///
/// Add this once at the app root; it registers each model plugin idempotently.
pub struct DurhamTerrainModelsPlugin;

impl Default for DurhamTerrainModelsPlugin {
	fn default() -> Self {
		Self
	}
}

/// Idempotent registration of [`DurhamTerrainModelsPlugin`].
pub fn register_durham_plugin(app: &mut App) {
	if app.is_plugin_added::<DurhamTerrainModelsPlugin>() {
		return;
	}
	app.add_plugins(DurhamTerrainModelsPlugin);
}

impl Plugin for DurhamTerrainModelsPlugin {
	fn build(&self, app: &mut App) {
		register_terrain_plugin(app);
		register_water_plugin(app);
	}
}
