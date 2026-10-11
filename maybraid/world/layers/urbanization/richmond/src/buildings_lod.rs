//! Gimme LOD refresh for Richmond development hosts.

use avian3d::prelude::PhysicsPlugins;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;
use building_components::{
	BuildingStructuralLodPlugin, ComponentsOnly, DoorNode, FloorNode, FurnitureNode, JointNode,
	LabelNode, PanelNode, PartitionNode, RoofNode, StairNode,
};
use buildings::wall_demo::TerrainPerimeterWall;
use buildings::wizards_tower::WizardsTower;
use buildings::SingleHighrise;
use buildings::{
	ConnectingStairwell, MixedUseLesHallesStorey, PitchedRoof, RectangularPitchedRoofComplex,
};
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullRegionCursor, LodRefreshCorePlugin,
	LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice, Spotlight,
};
use lod_gimme::{GimmeLodSceneCullPlugin, GimmeLodSceneRefreshPlugin};
use std::sync::Arc;
use urbanization_developments::{
	CircularTower, GalleryColonnade, GalleryTerrace, ShepherdsHouse, ShepherdsHut,
	Skybridge as SkybridgeHall, TempleSanctum, TrazaloidTower,
};

/// Shared produce domain for bullseye and spotlight building refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsRefresh;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
pub type BuildingsBullseye = BuildingsRefresh;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
pub type BuildingsSpotlight = BuildingsRefresh;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct BuildingsCull;

/// Historical Avian-named wrapper. Prefer `gimme_host!`.
#[allow(unused_macros)]
macro_rules! avian_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
								lod_avian::AvianLodSceneRefreshPlugin::<$ty, BuildingsRefresh, With<Camera>>::without_full_scan_cull(),
								lod_avian::AvianLodSceneCullPlugin::<$ty, BuildingsCull, With<Camera>>::default(),
							));
	}};
}

macro_rules! gimme_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
								GimmeLodSceneRefreshPlugin::<$ty, BuildingsRefresh, With<Camera>>::without_full_scan_cull(),
								GimmeLodSceneCullPlugin::<$ty, BuildingsCull, With<Camera>>::default(),
							));
	}};
}

/// Full refresh stack for structural + leftover fine-phase development hosts.
#[derive(Default)]
pub struct DevelopmentsBuildingsLodPlugin;

/// Idempotent registration of [`DevelopmentsBuildingsLodPlugin`].
pub fn register_developments_buildings_lod_plugin(app: &mut App) {
	if app.is_plugin_added::<DevelopmentsBuildingsLodPlugin>() {
		return;
	}
	app.add_plugins(DevelopmentsBuildingsLodPlugin);
}

impl Plugin for DevelopmentsBuildingsLodPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<BuildingStructuralLodPlugin>() {
			app.add_plugins(BuildingStructuralLodPlugin);
		}

		app.insert_resource(Bullseye { inner: 80.0, outer: 500.0 })
			.insert_resource(Spotlight { extent: 80.0 })
			.insert_resource(OpenLattice {
				exclude_extent: 1000.0,
				outer_extent: 5000.0,
				tile_size: 500.0,
			})
			.insert_resource(LodCullRegionCursor::default().with_regions_per_tick(1))
			.insert_resource(LodChunkFulfillBudget {
				spawn_weights_per_frame: 256,
				cull_weights_per_frame: 128,
				cull_root_despawns_per_frame: 2,
				begins_per_frame: 48,
				begin_scan_per_frame: 192,
				begin_weights_per_frame: 256,
				begin_prefill_weights_per_job: 8,
				completes_per_frame: 128,
				..Default::default()
			})
			.add_plugins((
				LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, BuildingsBullseye>::default(),
				LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, BuildingsSpotlight>::default(
				),
				LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, BuildingsCull>::default(),
			));

		gimme_host!(app, PanelNode);
		gimme_host!(app, PartitionNode);
		gimme_host!(app, RoofNode);
		gimme_host!(app, FloorNode);
		gimme_host!(app, StairNode);
		gimme_host!(app, DoorNode);
		gimme_host!(app, JointNode);
		gimme_host!(app, FurnitureNode);
		gimme_host!(app, LabelNode);
		gimme_host!(app, ComponentsOnly<Arc<MixedUseLesHallesStorey>>);
		gimme_host!(app, ComponentsOnly<MixedUseLesHallesStorey>);
		gimme_host!(app, ComponentsOnly<TerrainPerimeterWall>);
		gimme_host!(app, ComponentsOnly<ConnectingStairwell>);
		gimme_host!(app, ComponentsOnly<PitchedRoof>);
		gimme_host!(app, ComponentsOnly<Arc<ShepherdsHouse>>);
		gimme_host!(app, ComponentsOnly<Arc<ShepherdsHut>>);
		gimme_host!(app, ComponentsOnly<Arc<CircularTower>>);
		gimme_host!(app, ComponentsOnly<Arc<TrazaloidTower>>);
		gimme_host!(app, ComponentsOnly<GalleryTerrace>);
		gimme_host!(app, ComponentsOnly<GalleryColonnade>);
		gimme_host!(app, ComponentsOnly<RectangularPitchedRoofComplex>);
		gimme_host!(app, ComponentsOnly<Arc<SingleHighrise>>);
		gimme_host!(app, ComponentsOnly<Arc<TempleSanctum>>);
		gimme_host!(app, WizardsTower);
		gimme_host!(app, ComponentsOnly<Arc<SkybridgeHall>>);
	}
}
