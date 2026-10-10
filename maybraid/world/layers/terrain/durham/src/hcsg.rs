//! Durham on the shared HCSG runtime ([`lod::hcsg`]): terrain and
//! water generate on the worker and present as [`HcsgNode`] hosts.
//!
//! Start a session with [`lod::hcsg::request_hcsg_session_restart`].

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::system::{SystemParam, SystemParamItem};
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::Id;
use lod::hcsg::{
	register_session_seed, Busy, HcsgBounds, HcsgClass, HcsgStorage, PresentationPlugin,
};
use lod::LodViewer;
use render_item::mesh::handle::MeshFulfillBudget;
use terrain_layer_model::TerrainStreaming;
use terrain_shaders::{RefractionWater, TerrainShader, TerrainShaderPlugin};
use visual_geometry_core::{
	install_enforced_mesh_cache, share_terrain_chunk_refs, VisualGeometryCorePlugin,
};

use crate::register_durham_plugin;
use crate::terrain::index::{origin_cell_ids_at, seed_durham_hcsg_roots};
use crate::terrain::{
	mesh_assets, playable_world_cell_layout, BaseTerrainNoise, Durham, Terrain, TerrainCellLayout,
	TerrainConfig, TerrainCoverage, TerrainMeshBuilder, WorldBaseTerrain,
	WORLD_FINE_HALF_EXTENT_CELLS,
};
use crate::water::{ComposedWater, Water, WaterColumn, WaterMeshAssets};

/// The [`TerrainCellLayout`] resource's window: its request region.
pub struct DurhamWindow;

impl HcsgBounds for DurhamWindow {
	const CLASS: HcsgClass = HcsgClass::Near;

	type Param = Res<'static, TerrainCellLayout>;

	fn regions(layout: &SystemParamItem<Self::Param>) -> Vec<Aabb3d> {
		vec![layout.request_region()]
	}

	fn focus(layout: &SystemParamItem<Self::Param>) -> Option<Vec3> {
		Some(layout.region_center_xz())
	}
}

/// Presents Durham terrain and water within channel `C`'s regions from the
/// shared storage.
///
/// Scenes come from [`Terrain`] and [`Water`]'s own `LodScene` impls; cells
/// that seed collision carry the trimesh source.
pub struct DurhamPresentationPlugin<C>(PhantomData<fn() -> C>);

impl<C> Default for DurhamPresentationPlugin<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static> Plugin for DurhamPresentationPlugin<C> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<WaterPresentationPlugin<C>>() {
			app.add_plugins(WaterPresentationPlugin::<C>::default());
		}
		app.add_plugins(PresentationPlugin::<C, Terrain>::default());
	}
}

/// Presents Durham water within channel `C`'s regions from the shared
/// storage, for a layer that presents its own surface over Durham terrain.
pub struct WaterPresentationPlugin<C>(PhantomData<fn() -> C>);

impl<C> Default for WaterPresentationPlugin<C> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static> Plugin for WaterPresentationPlugin<C> {
	fn build(&self, app: &mut App) {
		app.add_plugins(PresentationPlugin::<C, Water>::default());
	}
}

/// Registers Durham's HCSG session root seeder once per app.
struct DurhamHcsgSessionPlugin;

impl Plugin for DurhamHcsgSessionPlugin {
	fn build(&self, app: &mut App) {
		register_session_seed(app, seed_durham_hcsg_roots);
	}
}

/// Installs [`DurhamHcsgSessionPlugin`] at most once. World installs call this
/// from [`DurhamWorldPlugin`]; integration tests without the world plugin call
/// it directly.
pub fn register_durham_hcsg_session(app: &mut App) {
	if !app.is_plugin_added::<DurhamHcsgSessionPlugin>() {
		app.add_plugins(DurhamHcsgSessionPlugin);
	}
}

/// The playable world's Durham: its rings, seed, meshing, shaders and
/// collision. Streams present over it (see [`StreamPresentationPlugin`]); a
/// session seeds its roots through [`HcsgSessionSeed`](lod::hcsg::HcsgSessionSeed).
pub struct DurhamWorldPlugin {
	pub seed: u32,
}

impl Plugin for DurhamWorldPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VisualGeometryCorePlugin>() {
			app.add_plugins(VisualGeometryCorePlugin);
		}
		register_durham_plugin(app);
		if !app.is_plugin_added::<TerrainShaderPlugin>() {
			app.add_plugins(TerrainShaderPlugin);
		}
		install_enforced_mesh_cache::<TerrainMeshBuilder, TerrainShader>(app);
		share_terrain_chunk_refs::<TerrainMeshBuilder>(app, false);
		install_enforced_mesh_cache::<ComposedWater, RefractionWater>(app);
		let config = TerrainConfig::new(self.seed);
		app.insert_resource(MeshFulfillBudget::<TerrainMeshBuilder>::new(8, 16, 256))
			.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&config)))
			.insert_resource(config)
			.insert_resource(playable_world_cell_layout())
			.init_resource::<TerrainStreaming<Durham>>()
			.add_systems(Update, prefer_meshes_near_the_viewer);
	}

	fn finish(&self, app: &mut App) {
		let world = app.world_mut();
		let material = world.resource_mut::<Assets<TerrainShader>>().add(TerrainShader::default());
		let water = world.resource_mut::<Assets<RefractionWater>>().add(RefractionWater::default());
		let config = world.resource::<TerrainConfig>().clone();
		world.insert_resource(mesh_assets(
			config,
			material,
			TerrainCoverage::PlayableWorld,
			WORLD_FINE_HALF_EXTENT_CELLS,
		));
		world.insert_resource(WaterMeshAssets { material: water });
		register_durham_hcsg_session(app);
	}
}

fn prefer_meshes_near_the_viewer(
	viewers: Query<&Transform, With<LodViewer>>,
	mut budget: ResMut<MeshFulfillBudget<TerrainMeshBuilder>>,
) {
	if let Some(viewer) = viewers.iter().next() {
		budget.prefer_xz = Some(Vec3::new(viewer.translation.x, 0.0, viewer.translation.z));
	}
}

/// Frame-side terrain reads over the shared storage. [`Busy`] while the
/// worker holds a store; try again next frame.
pub trait SharedTerrainStorage {
	/// The published origin cell covering `(x, z)`: fine grid first, then
	/// outer and stream rings.
	fn try_terrain_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<Arc<Terrain>>, Busy>;

	/// Composed terrain height (stamps and watersheds) at `(x, z)`, once
	/// that cell is published.
	fn try_composed_height_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<f32>, Busy> {
		let terrain = self.try_terrain_at(layout, x, z)?;
		Ok(terrain.map(|terrain| terrain.sdf.terrain().height_at_with_all_modulations(x, z)))
	}

	/// The wet column at `(x, z)`, once its water cell is published.
	fn try_water_column_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<WaterColumn>, Busy>;

}

impl SharedTerrainStorage for HcsgStorage {
	fn try_terrain_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<Arc<Terrain>>, Busy> {
		for id in origin_cell_ids_at(layout, x, z) {
			if let Some(entry) = self.try_entry::<Terrain>(id)? {
				return Ok(Some(entry.value));
			}
		}
		Ok(None)
	}

	fn try_water_column_at(
		&self,
		layout: &TerrainCellLayout,
		x: f32,
		z: f32,
	) -> Result<Option<WaterColumn>, Busy> {
		for id in origin_cell_ids_at(layout, x, z) {
			if let Some(entry) = self.try_entry::<Water>(id)? {
				return Ok(entry.value.column_at(x, z));
			}
		}
		Ok(None)
	}

}

/// Test-only direct publishes into [`HcsgStorage`]. Frame code uses
/// [`DurhamSurface`] or presentation hosts.
#[cfg(any(test, feature = "test-support"))]
pub trait SharedTerrainStorageTestExt: SharedTerrainStorage {
	/// Publishes origin cell `(ix, iz)` of `layout` with `base` as its whole
	/// SDF.
	fn publish_base_terrain_for_test(
		&self,
		layout: &TerrainCellLayout,
		ix: i32,
		iz: i32,
		base: BaseTerrainNoise,
	);
}

#[cfg(any(test, feature = "test-support"))]
impl SharedTerrainStorageTestExt for HcsgStorage {
	fn publish_base_terrain_for_test(
		&self,
		layout: &TerrainCellLayout,
		ix: i32,
		iz: i32,
		base: BaseTerrainNoise,
	) {
		let terrain = Terrain::base_cell_for_test(layout, ix, iz, base);
		let cell = terrain.cell;
		self.publish(Id::from_cell(cell), Arc::new(terrain), cell);
	}
}

/// The playable ground under frame-side readers: composed heights and wet
/// columns from the shared storage. A cell not yet published reads as
/// `Ok(None)`; [`Busy`] means retry next frame.
#[derive(SystemParam)]
pub struct DurhamSurface<'w> {
	storage: Res<'w, HcsgStorage>,
	layout: Res<'w, TerrainCellLayout>,
	base: Res<'w, WorldBaseTerrain>,
}

impl DurhamSurface<'_> {
	pub fn height_at(&self, xz: Vec2) -> Result<Option<f32>, lod::hcsg::Busy> {
		self.storage.try_composed_height_at(&self.layout, xz.x, xz.y)
	}

	/// Base noise, where no composed cell is published.
	pub fn fallback_height_at(&self, xz: Vec2) -> f32 {
		self.base.0.height_at(xz.x, xz.y)
	}

	pub fn height_or_fallback(&self, xz: Vec2) -> Result<f32, lod::hcsg::Busy> {
		Ok(self.height_at(xz)?.unwrap_or_else(|| self.fallback_height_at(xz)))
	}

	pub fn water_column_at(&self, xz: Vec2) -> Result<Option<WaterColumn>, lod::hcsg::Busy> {
		self.storage.try_water_column_at(&self.layout, xz.x, xz.y)
	}

	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	pub fn base(&self) -> &BaseTerrainNoise {
		&self.base.0
	}
}

mod stream;

pub use stream::{
	BackgroundStream, FarStream, NearStream, PlayableStreams, StreamPresentationPlugin, StreamRing,
	Streamed, TerrainStream,
};

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::scene::ScenePlugin;
	use lod::gen::{Id, OriginalId, Version};
	use lod::hcsg::{HcsgBoundsPlugin, HcsgDemand, HcsgNode, HcsgRestartRequest};
	use lod::lod_ref::LodNodePose;
	use lod::LodViewer;

	use super::*;
	use super::SharedTerrainStorageTestExt;
	use crate::terrain::{
		fine_patch_cell_layout, CellTiling, TerrainColliderMeshSource, TerrainConfig,
		TerrainMeshAssets, TerrainMeshLodBand, TerrainStampConfigs, WatershedConfigs,
	};
	use crate::water::WaterMeshAssets;

	const IDLE: Duration = Duration::from_secs(120);

	fn seed_resources(app: &mut App, seed: u32) {
		app.insert_resource(TerrainStampConfigs::from_world_seed(seed))
			.insert_resource(WatershedConfigs::default().with_seed(seed))
			.insert_resource(TerrainMeshAssets {
				config: TerrainConfig::new(seed),
				material: Handle::default(),
				lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
				outer_add_walls: false,
				fine_grid_max_radius: Some(1),
				macro_seam_half_extents: Vec::new(),
				macro_cell_min_size: None,
				macro_res_2: None,
			})
			.insert_resource(HcsgRestartRequest::queued());
	}

	/// A 2×2 fine patch at the origin, presented from the shared runtime.
	fn app() -> App {
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
			.insert_resource(WaterMeshAssets { material: Handle::default() })
			.add_plugins((
				HcsgBoundsPlugin::<DurhamWindow>::default(),
				DurhamPresentationPlugin::<DurhamWindow>::default(),
			));
		seed_resources(&mut app, 1);
		register_durham_hcsg_session(&mut app);
		let at = Transform::IDENTITY;
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
		app
	}

	/// Restart and subscribe, let the worker finish, then spawn and fulfill hosts.
	fn settle(app: &mut App) -> anyhow::Result<()> {
		for _ in 0..2 {
			app.update();
			let demand = app.world().resource::<HcsgDemand>().clone();
			anyhow::ensure!(demand.wait_idle(IDLE), "worker did not go idle");
		}
		for _ in 0..4 {
			app.update();
		}
		Ok(())
	}

	fn hosts<T: lod::hcsg::HcsgValue>(app: &mut App) -> Vec<(Id, Version)> {
		let mut hosts: Vec<_> = app
			.world_mut()
			.query::<&HcsgNode<T>>()
			.iter(app.world())
			.map(|node| (node.id, node.version))
			.collect();
		hosts.sort();
		hosts
	}

	/// Height inside the patch, off the noise lattice (noise is zero on it).
	fn patch_height(app: &App) -> anyhow::Result<Option<f32>> {
		let layout = app.world().resource::<TerrainCellLayout>();
		let at = layout.region_center_xz() + Vec3::new(0.37, 0.0, 0.61) * layout.cell_size;
		let storage = app.world().resource::<HcsgStorage>();
		storage
			.try_composed_height_at(layout, at.x, at.z)
			.map_err(|_| anyhow::anyhow!("storage busy after idle"))
	}

	#[test]
	fn the_window_presents_terrain_and_water_with_collision() -> anyhow::Result<()> {
		let mut app = app();
		settle(&mut app)?;

		let layout = app.world().resource::<TerrainCellLayout>().clone();
		let mut cells: Vec<Id> = layout
			.cell_ids(layout.request_region())
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		cells.sort();
		let terrain: Vec<Id> = hosts::<Terrain>(&mut app).into_iter().map(|(id, _)| id).collect();
		assert_eq!(terrain, cells, "one terrain host per layout cell");
		let water: Vec<Id> = hosts::<Water>(&mut app).into_iter().map(|(id, _)| id).collect();
		let mut wet = app
			.world()
			.resource::<HcsgStorage>()
			.try_overlapping::<Water>(layout.request_region())
			.expect("storage idle after worker settle");
		wet.sort();
		assert_eq!(water, wet, "one water host per wet cell");
		assert!(patch_height(&app)?.is_some());

		let colliders = app
			.world_mut()
			.query_filtered::<(), With<TerrainColliderMeshSource>>()
			.iter(app.world())
			.count();
		assert_eq!(colliders, cells.len(), "fine patch cells seed collision");
		Ok(())
	}

	#[test]
	fn a_restart_replaces_every_host_with_the_new_session() -> anyhow::Result<()> {
		let mut app = app();
		settle(&mut app)?;
		let before = hosts::<Terrain>(&mut app);
		let height = patch_height(&app)?;

		seed_resources(&mut app, 2);
		settle(&mut app)?;
		let after = hosts::<Terrain>(&mut app);

		assert_eq!(after.len(), before.len());
		for ((id, old), (new_id, new)) in before.iter().zip(&after) {
			assert_eq!(id, new_id);
			assert!(new > old, "{id:?} still shows the previous session");
		}
		assert_ne!(patch_height(&app)?, height, "the new seed reshapes the terrain");
		Ok(())
	}

	fn storage_busy(busy: lod::hcsg::Busy) -> anyhow::Error {
		anyhow::anyhow!("HcsgStorage busy: {busy:?}")
	}

	#[test]
	fn the_surface_reads_published_cells_and_falls_back_elsewhere() -> anyhow::Result<()> {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let storage = HcsgStorage::default();
		storage.publish_base_terrain_for_test(&layout, 0, 0, base.clone());
		world.insert_resource(storage);
		world.insert_resource(layout.clone());
		world.insert_resource(WorldBaseTerrain(base.clone()));

		let mut state = bevy::ecs::system::SystemState::<DurhamSurface>::new(&mut world);
		let surface = state.get(&world)?;
		let inside = Vec2::splat(0.5 * layout.cell_size);
		let outside = Vec2::splat(10.5 * layout.cell_size);
		assert_eq!(
			surface.height_at(inside).map_err(storage_busy)?,
			Some(base.height_at(inside.x, inside.y))
		);
		assert_eq!(surface.height_at(outside).map_err(storage_busy)?, None);
		assert_eq!(
			surface.height_or_fallback(outside).map_err(storage_busy)?,
			base.height_at(outside.x, outside.y)
		);
		assert_eq!(surface.water_column_at(inside).map_err(storage_busy)?, None);
		Ok(())
	}
}
