//! Vegetation LOD refresh: bullseye + spotlight → Gimme host index → levels → chunk sync.
//!
//! Fine-phase domain hosts ([`FoliageNode`], [`StickNode`]) stay registered for
//! any leftover nested kit nodes. Isolated plants and woody grove children share one
//! family: [`FlattenedComponentsOnly`]`<`[`PlacedVegetation`]`<`[`std::sync::Arc`]`<T>>>`.
//! Groves register as themselves. Cull uses a rotating [`OpenLattice`] annulus.

use crate::host::ChicoGroveHost;
use avian3d::prelude::PhysicsPlugins;
use avian3d::schedule::PhysicsSchedulePlugin;
use bevy::prelude::*;
use lod::{
	Bullseye, LodChunkFulfillBudget, LodCullProduceCadence, LodCullRegionCursor,
	LodRefreshCorePlugin, LodSceneCullRegionPlugin, LodSceneRefreshRegionPlugin, OpenLattice,
	Spotlight,
};
use lod_gimme::{GimmeLodSceneCullPlugin, GimmeLodSceneRefreshPlugin};
use lod_lazy_refs::LodLazyRefsPlugin;
use sbs_trees::{
	BraidOakTree, DatePalm, FriendsConifer, HighBushShoots, HonuBanyan, JungleGrowth,
	JungleStorybookTree, KamakuraTorch, LiamsConifer, NorthernConifer, PalmBush, PalmCrown,
	PenmarchTorch, RorysHeadTrained, SimplemansHedge, SopesBanyan, StorybookTree, TemperateConifer,
	TuftPatch, VaseTree, WaialeaPalm,
};
use scene_ref::SceneRefAdmitBudget;
use vegetation_components::{FlattenedComponentsOnly, FoliageNode, PlacedVegetation, StickNode};
use vegetation_groves::{
	Alpine, AridConiferSapling, BraidGrass, BushScrub, ChristmasTaiga, CommonTufts,
	ConiferMassives, ConiferSapling, DateGrove, Dryland, ForlornSavanna, GoettingenFollow,
	HighBush, JerrysChaparral, JungleLowerMassives, JungleMassives, Leeward, LevantineScrub,
	LowBush, MonsterGrass, OasisDatePalm, Orchard, PalmShade, RiparianGeneral, RiparianMix,
	RiverineGreen, RollingOaks, Shamanhome, SpottyBushes, Storytellers, StrangeOasis, TallGrass,
	TemperateLowerMassives, TemperateMassives, TradeWinds, TropicalThicket, TropicalTufts,
	TropicalUndergrowth, UnendingJungle, Vineyard, WanderingAcacia, WildGrass,
};

use crate::stick_physics::{register_vegetation_stick_colliders, StickPhysicsPlugin};

/// Shared produce domain for bullseye and spotlight vegetation refresh.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationRefresh;

/// Channel marker for bullseye [`lod::LodSceneRefreshRegion`] messages.
pub type VegetationBullseye = VegetationRefresh;

/// Channel marker for spotlight [`lod::LodSceneRefreshRegion`] messages.
pub type VegetationSpotlight = VegetationRefresh;

/// Channel marker for OpenLattice [`lod::LodSceneCullRegion`] messages.
#[derive(Debug, Clone, Copy, Default)]
pub struct VegetationCull;

/// Isolated `/show` plant and grove-nested plant host.
type FlattenedPlant<T> = FlattenedComponentsOnly<PlacedVegetation<std::sync::Arc<T>>>;

/// Historical Avian-named wrapper. Prefer `gimme_host!`.
#[allow(unused_macros)]
macro_rules! avian_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
							lod_avian::AvianLodSceneRefreshPlugin::<$ty, VegetationRefresh, With<Camera>>::without_full_scan_cull(),
							lod_avian::AvianLodSceneCullPlugin::<$ty, VegetationCull, With<Camera>>::default(),
						));
	}};
}

/// Register Gimme refresh + cull for one LOD host type (fine-phase or structural).
macro_rules! gimme_host {
	($app:expr, $ty:ty) => {{
		$app.add_plugins((
							GimmeLodSceneRefreshPlugin::<$ty, VegetationRefresh, With<Camera>>::without_full_scan_cull(),
							GimmeLodSceneCullPlugin::<$ty, VegetationCull, With<Camera>>::default(),
						));
	}};
}

/// Flattened plant hosts also get High-IR stick capsules at High/Medium (no nested [`StickNode`] hosts).
macro_rules! flattened_plant_host {
	($app:expr, $ty:ty) => {{
		gimme_host!($app, FlattenedPlant<$ty>);
		register_vegetation_stick_colliders::<FlattenedPlant<$ty>>($app);
	}};
}

/// Full modern refresh stack for structural + fine-phase vegetation hosts.
///
/// 1. Camera → [`Bullseye`] / [`Spotlight`] region messages
/// 2. [`PatchSceneBounds`](lod::PatchSceneBounds) stamps host bounds from
///    [`LodScene::scene_bounds`](lod::LodScene::scene_bounds)
/// 3. Gimme region index → level messages for structural and child component hosts
/// 4. Entity refresh (max fold) + chunk sync
/// 5. [`OpenLattice`] cull regions → Gimme index → budgeted root teardown
pub struct VegetationLodRefreshPlugin;

impl Plugin for VegetationLodRefreshPlugin {
	fn build(&self, app: &mut App) {
		// Same sentinel as Durham terrain resources: `PhysicsPlugins` is a group.
		if !app.is_plugin_added::<PhysicsSchedulePlugin>() {
			app.add_plugins(PhysicsPlugins::default());
		}
		if !app.is_plugin_added::<LodRefreshCorePlugin>() {
			app.add_plugins(LodRefreshCorePlugin);
		}
		if !app.is_plugin_added::<StickPhysicsPlugin>() {
			app.add_plugins(StickPhysicsPlugin);
		}

		app.insert_resource(Bullseye {
			inner: 50.0,
			// Edge length: ±1000 m so produce covers the 1 km present ring (Medium
			// at 350–700 m). `500` is ±250 m and already inside High.
			outer: 2000.0,
		})
		.insert_resource(Spotlight { extent: 50.0 })
		.insert_resource(OpenLattice {
			exclude_extent: 1000.0,
			outer_extent: 5000.0,
			tile_size: 500.0,
		})
		.insert_resource(LodCullRegionCursor::default().with_regions_per_tick(1))
		.insert_resource(LodCullProduceCadence::every_n_frames(4))
		.insert_resource(LodChunkFulfillBudget {
			spawn_weights_per_frame: 1024,
			cull_weights_per_frame: 128,
			cull_root_despawns_per_frame: 2,
			begins_per_frame: 96,
			begin_scan_per_frame: 384,
			begin_weights_per_frame: 1024,
			begin_prefill_weights_per_job: 32,
			completes_per_frame: 1024,
			..Default::default()
		})
		.insert_resource(SceneRefAdmitBudget {
			per_frame: 256,
			new_merge_meshes_per_frame: 64,
			inspect_per_frame: 256,
		})
		.add_plugins((
			LodSceneRefreshRegionPlugin::<Bullseye, With<Camera>, VegetationBullseye>::default(),
			LodSceneRefreshRegionPlugin::<Spotlight, With<Camera>, VegetationSpotlight>::default(),
			LodSceneCullRegionPlugin::<OpenLattice, With<Camera>, VegetationCull>::default(),
		));

		// Fine-phase domain hosts nested under grove LodScene roots.
		gimme_host!(app, FoliageNode);
		gimme_host!(app, StickNode);

		// Forest / world present: one host wrapping ForestGroveTile ([#652](https://github.com/ramate-io/maybraid/issues/652)).
		gimme_host!(app, ChicoGroveHost);

		// Tuft grove roots (LodScene).
		gimme_host!(app, MonsterGrass);
		gimme_host!(app, BraidGrass);
		gimme_host!(app, TropicalTufts);
		gimme_host!(app, CommonTufts);
		gimme_host!(app, TallGrass);
		gimme_host!(app, WildGrass);

		// Woody grove roots (LodScene).
		gimme_host!(app, BushScrub);
		gimme_host!(app, TropicalUndergrowth);
		gimme_host!(app, LevantineScrub);
		gimme_host!(app, StrangeOasis);
		gimme_host!(app, TropicalThicket);
		gimme_host!(app, RollingOaks);
		gimme_host!(app, Orchard);
		gimme_host!(app, RiparianGeneral);
		gimme_host!(app, ForlornSavanna);
		gimme_host!(app, GoettingenFollow);
		gimme_host!(app, Vineyard);
		gimme_host!(app, Dryland);
		gimme_host!(app, Leeward);
		gimme_host!(app, TemperateLowerMassives);
		gimme_host!(app, TemperateMassives);
		gimme_host!(app, Storytellers);
		gimme_host!(app, WanderingAcacia);
		gimme_host!(app, TradeWinds);
		gimme_host!(app, HighBush);
		gimme_host!(app, SpottyBushes);
		gimme_host!(app, RiverineGreen);
		gimme_host!(app, LowBush);
		gimme_host!(app, JungleMassives);
		gimme_host!(app, JungleLowerMassives);
		gimme_host!(app, UnendingJungle);
		gimme_host!(app, JerrysChaparral);
		gimme_host!(app, RiparianMix);
		gimme_host!(app, Alpine);
		gimme_host!(app, ChristmasTaiga);
		gimme_host!(app, ConiferSapling);
		gimme_host!(app, AridConiferSapling);
		gimme_host!(app, ConiferMassives);
		gimme_host!(app, PalmShade);
		gimme_host!(app, Shamanhome);
		gimme_host!(app, DateGrove);

		// Isolated /show plants. Grove tiles pose kits under ChicoGroveHost.
		flattened_plant_host!(app, StorybookTree);
		flattened_plant_host!(app, VaseTree);
		flattened_plant_host!(app, JungleStorybookTree);
		flattened_plant_host!(app, BraidOakTree);
		flattened_plant_host!(app, RorysHeadTrained);
		flattened_plant_host!(app, PenmarchTorch);
		flattened_plant_host!(app, KamakuraTorch);
		flattened_plant_host!(app, HighBushShoots);
		flattened_plant_host!(app, JungleGrowth);
		flattened_plant_host!(app, SimplemansHedge);
		flattened_plant_host!(app, PalmBush);
		flattened_plant_host!(app, HonuBanyan);
		flattened_plant_host!(app, SopesBanyan);
		flattened_plant_host!(app, DatePalm);
		flattened_plant_host!(app, WaialeaPalm);
		flattened_plant_host!(app, OasisDatePalm);
		flattened_plant_host!(app, TuftPatch);
		flattened_plant_host!(app, FriendsConifer);
		flattened_plant_host!(app, LiamsConifer);
		flattened_plant_host!(app, NorthernConifer);
		flattened_plant_host!(app, TemperateConifer);
		flattened_plant_host!(app, PalmCrown);

		if !app.is_plugin_added::<LodLazyRefsPlugin>() {
			app.add_plugins(LodLazyRefsPlugin);
		}
	}
}
