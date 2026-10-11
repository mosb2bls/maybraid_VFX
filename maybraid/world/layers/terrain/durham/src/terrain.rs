//! Durham terrain model: SDF composition, LOD generation, Avian index, render.

pub mod base_noise;
pub mod cell;
pub mod collider;
pub mod config;
pub mod geography;
pub mod host;
pub mod index;
pub mod mesh;
pub mod plugin;
pub mod render;
pub mod sdf;
pub mod stamp_modulation;
pub mod stamps;
pub mod stream_lod;
pub mod watersheds;

use crate::terrain::render::cascade_chunk_for_cell;
use crate::terrain::stamps::StampLeaf;
use crate::terrain::watersheds::{
	HydroComplexCell, WatershedAproningCell, WatershedCarvingCell, WatershedRimmingCell,
};
use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use render_item::mesh::handle::Cached;
use render_item::sdf::cpu_shot::{CpuShotBuilder, WallFaces};
use std::sync::Arc;
use terrain_shaders::TerrainShader;
use terrain_stamps::StampModulation;
use terrain_watersheds::{HydroComplex, WaterFill};

pub use base_noise::BaseTerrainNoise;
pub use cell::{
	CellTiling, MacroCellLayout, OuterCellRing, TerrainCellLayout, TerrainCellRing,
	MACRO_CELL_SIZE, TERRAIN_CELL_SIZE,
};
pub use chunk::cascade::CascadeChunk;
pub use collider::{
	terrain_collider_covers_xz, TerrainColliderMeshSource, TerrainColliderSystems,
	TerrainFrictionConfig, TerrainTrimeshCollider, TERRAIN_FRICTION,
};
pub use config::TerrainConfig;
pub use geography::{
	GeographicBand, GeographicFamily, GeographicFeature, GeographicFeatureId, GeographicFeatureKind,
};
pub use host::{
	fine_patch_cell_layout, mesh_assets, playable_world_cell_layout, retarget_mesh_assets, Durham,
	TerrainCoverage, TerrainLayoutPinned, TerrainPresentPending, TerrainPresentationDirty,
	TerrainRetarget, WorldBaseTerrain, WORLD_FINE_HALF_EXTENT_CELLS, WORLD_OUTER_2X_ROWS,
	WORLD_OUTER_4X_ROWS,
};
#[cfg(test)]
pub use index::TerrainStorage;
pub use index::{DurhamRoots, TerrainHeightSnapshot, WaterSurfaceSnapshot, DURHAM_INDEX_SCALE};
pub use mesh::{TerrainMeshAssets, TerrainMeshLodBand};
pub use plugin::{register_terrain_plugin, TerrainResourcesPlugin};
pub use render::TerrainRenderItem;
pub use sdf::{ComposedTerrain, ElevationModulation, TerrainSdf};
pub use stamp_modulation::ComposedElevationOp;
pub use stamps::{
	CanyonHighPassControllerCell, CanyonHighPassControllerLayout, CanyonHighPassStampCell,
	CanyonLowPassControllerCell, CanyonLowPassControllerLayout, CanyonLowPassStampCell,
	MassifHighPassControllerCell, MassifHighPassControllerLayout, MassifHighPassStampCell,
	MassifLowPassControllerCell, MassifLowPassControllerLayout, MassifLowPassStampCell,
	PlateauControllerLayout, PlateauHighPassControllerCell, PlateauHighPassControllerLayout,
	PlateauHighPassStampCell, PlateauLowPassControllerCell, PlateauLowPassControllerLayout,
	PlateauLowPassStampCell, PocketWaterHighPassControllerCell,
	PocketWaterHighPassControllerLayout, PocketWaterHighPassStampCell,
	PocketWaterLowPassControllerCell, PocketWaterLowPassControllerLayout,
	PocketWaterLowPassStampCell, RollingHighPassControllerCell, RollingHighPassControllerLayout,
	RollingHighPassStampCell, RollingLowPassControllerCell, RollingLowPassControllerLayout,
	RollingLowPassStampCell, TerrainStampConfigs, ValleyHighPassControllerCell,
	ValleyHighPassControllerLayout, ValleyHighPassStampCell, ValleyLowPassControllerCell,
	ValleyLowPassControllerLayout, ValleyLowPassStampCell,
};
pub use stamps::{
	CanyonLowPassStampCell as CanyonStampCell, MassifLowPassStampCell as MassifStampCell,
	PlateauLowPassStampCell as PlateauStampCell,
	PocketWaterLowPassStampCell as PocketWaterStampCell,
	RollingLowPassStampCell as RollingStampCell, ValleyLowPassStampCell as ValleyStampCell,
};
pub use stream_lod::{
	stream_banded_draws, stream_banded_level, stream_banded_scene, StreamBandedLod,
};
pub use watersheds::{
	PocketHighPassCell, PocketLowPassCell, PocketWater, PocketWatersHighPass, PocketWatersLowPass,
	PrePocketHighPassCell, PrePocketHighPassLayout, PrePocketLowPassCell, PrePocketLowPassLayout,
	WatershedBandPass, WatershedConfigs, WatershedLeafBounds, WatershedLeafKind,
};
/// Low-pass aliases kept for older HUD / call sites.
pub use watersheds::{
	PocketLowPassCell as PocketCell, PocketWatersLowPass as LakeStampCell,
	PrePocketLowPassCell as PrePocketCell, PrePocketLowPassLayout as PrePocketLayout,
};

/// CpuShot wrapper stored on [`Terrain`] and used by Durham fill + overlay presenters.
pub type TerrainMeshBuilder = CpuShotBuilder<Arc<ComposedTerrain>>;

/// Stamp-composed terrain **before** watershed stamps.
#[derive(Debug, Clone, Component)]
pub struct PreWatershedTerrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	pub modulations: Vec<StampModulation>,
	pub jersey_leaves: Vec<Aabb3d>,
	pub sdf: ComposedTerrain,
}

impl PreWatershedTerrain {
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		modulations: &[StampModulation],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation.clone()));
		}
		ComposedTerrain::from_terrain(sdf)
	}

	/// [`Self::sample_height`] on the context.
	pub fn sample_height_in(cx: &mut GenerationContext, x: f32, z: f32) -> Option<f32> {
		let layout = cx.get_or_generate::<TerrainCellLayout>(Id::Universal)?;
		let id = Id::from_cell(layout.fine_cell_bounds_containing(x, z));
		let pre = cx.get_or_generate::<Self>(id)?;
		Some(pre.sdf.terrain().height_at_with_all_modulations(x, z))
	}
}

/// Final terrain cell: pre-watershed heightfield + Watershed lake stamps.
#[derive(Debug, Clone, Component)]
pub struct Terrain {
	pub cell: Aabb3d,
	pub base: BaseTerrainNoise,
	/// Flattened jersey + marazion leaf elevation ops.
	pub modulations: Vec<ComposedElevationOp>,
	/// Leaf AABBs whose jersey stamps contributed (debug / HUD).
	pub jersey_leaves: Vec<Aabb3d>,
	/// Leaf AABBs whose Watershed lake stamps contributed (plus empties for debug).
	pub marazion_leaves: Vec<WatershedLeafBounds>,
	/// Fills from this origin cell's cellular [`HydroComplexCell`].
	///
	/// ComplexCell unions hydrology nodes from intersecting authored leaves
	/// (all bands); fills share that union φ. Collected with carve → rim →
	/// apron before SDF compose so [`crate::water::Water`] can evaluate wet
	/// volume against the finished heightfield.
	pub marazion_fills: Vec<WaterFill>,
	pub sdf: Arc<ComposedTerrain>,
	pub material: Handle<TerrainShader>,
	pub res_2: u8,
	/// Moving presentation band for streamed near / far / background terrain.
	pub stream_ring: Option<TerrainCellRing>,
	/// Per-face CpuShot edge height walls (LOD seam skirts).
	pub wall_faces: WallFaces,
}

impl Terrain {
	pub fn compose_sdf(
		base: &BaseTerrainNoise,
		modulations: &[ComposedElevationOp],
	) -> ComposedTerrain {
		let mut sdf = base.sdf.clone();
		for modulation in modulations {
			sdf.add_elevation_modulation(Box::new(modulation.clone()));
		}
		ComposedTerrain::from_terrain(sdf)
	}

	/// Shared CpuShot identity for fill [`Cached`] dispatch and overlay chunk refs.
	pub fn mesh_builder(&self) -> TerrainMeshBuilder {
		CpuShotBuilder::new(Arc::clone(&self.sdf)).with_wall_faces(self.wall_faces)
	}

	/// World pose for CpuShot verts, which are local to the cascade origin.
	pub fn chunk_pose(&self) -> Transform {
		Transform::from_translation(cascade_chunk_for_cell(self.cell, self.res_2).origin)
	}

	/// Near-ring (or unbanded FinePatch) cells carry a trimesh on this scene.
	pub fn seeds_collision(&self) -> bool {
		self.stream_ring.map(|ring| ring.seeds_collision()).unwrap_or(true)
	}

	/// Posed fill entity. [`Mesh3d`] (and the trimesh, when [`Self::seeds_collision`])
	/// land on this same entity after Cached fulfill.
	pub fn spawn_fill(
		&self,
		commands: &mut Commands,
		visibility: Visibility,
		collide: bool,
	) -> Entity {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let entity = commands
			.spawn((
				self.chunk_pose(),
				chunk,
				Cached::new(self.mesh_builder()),
				MeshMaterial3d(self.material.clone()),
				visibility,
			))
			.id();
		if collide {
			commands.entity(entity).insert(TerrainColliderMeshSource);
		}
		entity
	}

	pub fn scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = self.chunk_pose();
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<TerrainShader>({material.clone()})
			TerrainColliderMeshSource
		}
	}

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	fn mesh_scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = self.chunk_pose();
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<TerrainShader>({material.clone()})
		}
	}
}

impl crate::terrain::stream_lod::StreamBandedLod for Terrain {
	fn stream_ring(&self) -> Option<TerrainCellRing> {
		self.stream_ring
	}

	fn stream_center(&self) -> Vec3 {
		self.center()
	}
}

impl LodScene for Terrain {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		stream_banded_level(self, lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = stream_banded_level(self, lod_ref.previous_transform);
		let current = stream_banded_level(self, lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		if self.seeds_collision() {
			stream_banded_scene(self, level, || self.scene())
		} else {
			stream_banded_scene(self, level, || self.mesh_scene())
		}
	}
}

/// Non-empty jersey stamp leaves on one origin cell, in pull order.
#[derive(Default)]
struct JerseyStamps {
	modulations: Vec<StampModulation>,
	leaves: Vec<Aabb3d>,
}

impl JerseyStamps {
	fn pull_in<T: StampLeaf + hcsg::GenerationScheme>(
		&mut self,
		cx: &mut GenerationContext,
		bounds: Aabb3d,
	) -> Option<()> {
		for stamp in cx.get_or_generate_all_in::<T>(bounds)? {
			self.take(&*stamp);
		}
		Some(())
	}

	fn take(&mut self, stamp: &impl StampLeaf) {
		if !stamp.modulations().is_empty() {
			self.leaves.push(stamp.cell());
			self.modulations.extend_from_slice(stamp.modulations());
		}
	}
}

impl PreWatershedTerrain {
	fn compose(cell: Aabb3d, base: BaseTerrainNoise, stamps: JerseyStamps) -> Self {
		let JerseyStamps { modulations, leaves: jersey_leaves } = stamps;
		let sdf = Self::compose_sdf(&base, &modulations);
		Self { cell, base, modulations, jersey_leaves, sdf }
	}
}

/// Pre-watershed: base noise + jersey landform stamps (including pocket-water height).
///
/// Origin-grid root: tiles [`TerrainCellLayout`] directly. Each stamp band is
/// one leaf bound; its controller grid and configs are that band's concern.

impl hcsg::GenerationScheme for PreWatershedTerrain {
	lod::hcsg_index_scale!(crate::terrain::index::DURHAM_INDEX_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		TerrainCellLayout::origin_ids_in(cx, region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let base = cx.get_or_generate::<BaseTerrainNoise>(Id::Universal)?;

		// Composition order is global: high-pass (regional) bands, then low-pass (detail).
		let mut stamps = JerseyStamps::default();
		stamps.pull_in::<PlateauHighPassStampCell>(cx, bounds)?;
		stamps.pull_in::<MassifHighPassStampCell>(cx, bounds)?;
		stamps.pull_in::<CanyonHighPassStampCell>(cx, bounds)?;
		stamps.pull_in::<PocketWaterHighPassStampCell>(cx, bounds)?;
		stamps.pull_in::<RollingHighPassStampCell>(cx, bounds)?;
		stamps.pull_in::<ValleyHighPassStampCell>(cx, bounds)?;
		stamps.pull_in::<PlateauLowPassStampCell>(cx, bounds)?;
		stamps.pull_in::<MassifLowPassStampCell>(cx, bounds)?;
		stamps.pull_in::<CanyonLowPassStampCell>(cx, bounds)?;
		stamps.pull_in::<PocketWaterLowPassStampCell>(cx, bounds)?;
		stamps.pull_in::<RollingLowPassStampCell>(cx, bounds)?;
		stamps.pull_in::<ValleyLowPassStampCell>(cx, bounds)?;
		Some((Self::compose(bounds, BaseTerrainNoise::clone(&base), stamps), bounds))
	}
}

/// Pocket-water leaf metadata for one origin cell after [`HydroComplexCell`]
/// has materialized both bands (avoids a second `original_ids_for` walk).
fn marazion_leaves_in(cx: &mut GenerationContext, bounds: Aabb3d) -> Vec<WatershedLeafBounds> {
	let mut high: Vec<Id> = cx.overlapping::<PocketWatersHighPass>(bounds);
	high.sort();
	let mut leaves: Vec<WatershedLeafBounds> = high
		.into_iter()
		.filter_map(|id| cx.get::<PocketWatersHighPass>(id).map(|leaf| leaf.leaf_bounds()))
		.collect();
	let mut low: Vec<Id> = cx.overlapping::<PocketWatersLowPass>(bounds);
	low.sort();
	leaves.extend(
		low.into_iter()
			.filter_map(|id| cx.get::<PocketWatersLowPass>(id).map(|leaf| leaf.leaf_bounds())),
	);
	leaves
}

/// Final terrain: pre-watershed + Watershed correction stages (carve → rim → apron).
///
/// Shares [`PreWatershedTerrain`]'s origin ids. Pocket-water leaves, the
/// hydro complex, and stage cells are bound only as the leaves this scheme
/// reads; their pocket / pre-pocket / config stacks resolve at the index.

impl hcsg::GenerationScheme for Terrain {
	lod::hcsg_index_scale!(crate::terrain::index::DURHAM_INDEX_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<PreWatershedTerrain>(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let pre = cx.get_or_generate::<PreWatershedTerrain>(id)?;
		let complex_cell = cx.get_or_generate::<HydroComplexCell>(id)?;
		let marazion_leaves = marazion_leaves_in(cx, bounds);
		let complex = complex_cell.indexed().cloned();

		// Keep stage cells materialized for later policy work; elevation uses
		// the cellular HydroComplex directly (internal carve → rim → apron).
		cx.get_or_generate::<WatershedCarvingCell>(id)?;
		cx.get_or_generate::<WatershedRimmingCell>(id)?;
		cx.get_or_generate::<WatershedAproningCell>(id)?;

		let layout = cx.get::<TerrainCellLayout>(Id::Universal).unwrap_or_default();
		let assets = cx.get_or_generate::<TerrainMeshAssets>(Id::Universal)?;
		let terrain = Self::compose(bounds, &pre, marazion_leaves, complex, &layout, &assets);
		Some((terrain, bounds))
	}
}

impl Terrain {
	/// Corrects `pre` with the cell's hydro `complex` and fits its mesh to
	/// `layout`'s streams.
	fn compose(
		cell: Aabb3d,
		pre: &PreWatershedTerrain,
		marazion_leaves: Vec<WatershedLeafBounds>,
		complex: Option<Arc<HydroComplex>>,
		layout: &TerrainCellLayout,
		assets: &TerrainMeshAssets,
	) -> Self {
		let marazion_fills = complex.iter().cloned().map(WaterFill::from_hydro).collect();
		let modulations: Vec<_> = pre
			.modulations
			.iter()
			.cloned()
			.map(ComposedElevationOp::Stamp)
			.chain(complex.map(ComposedElevationOp::Hydro))
			.collect();
		let sdf = Arc::new(Self::compose_sdf(&pre.base, &modulations));
		let (res_2, wall_faces) = assets.mesh_params_for_cell(cell, layout);
		let cell_size = (Vec3::from(cell.max) - Vec3::from(cell.min)).x;
		Self {
			cell,
			base: pre.base.clone(),
			modulations,
			jersey_leaves: pre.jersey_leaves.clone(),
			marazion_leaves,
			marazion_fills,
			sdf,
			material: assets.material.clone(),
			res_2,
			stream_ring: layout.stream_ring_for_cell_size(cell_size),
			wall_faces,
		}
	}
}

#[cfg(test)]
mod scheme_perf_tests {
	use super::*;
	use crate::terrain::host::TerrainCoverage;
	use bevy::math::IVec2;
	use lod::gen::OriginalId;
	use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};
	use std::time::Instant;

	fn seed_storage(storage: &HcsgStorage, seed: u32, layout: &TerrainCellLayout) {
		storage.seed(layout.clone(), universal_bounds());
		storage.seed(TerrainStampConfigs::from_world_seed(seed), universal_bounds());
		storage.seed(WatershedConfigs::default().with_seed(seed), universal_bounds());
		storage.seed(
			mesh_assets(
				TerrainConfig::new(seed),
				Handle::default(),
				TerrainCoverage::FinePatch,
				1,
			),
			universal_bounds(),
		);
	}

	fn leaves_via_pocket_discovery(
		cx: &mut GenerationContext,
		bounds: Aabb3d,
	) -> Vec<WatershedLeafBounds> {
		let high = cx.get_or_generate_all_in::<PocketWatersHighPass>(bounds).unwrap();
		let low = cx.get_or_generate_all_in::<PocketWatersLowPass>(bounds).unwrap();
		high
			.iter()
			.map(|leaf| leaf.leaf_bounds())
			.chain(low.iter().map(|leaf| leaf.leaf_bounds()))
			.collect()
	}

	#[test]
	fn marazion_leaves_match_pocket_water_discovery() -> anyhow::Result<()> {
		let storage = HcsgStorage::default();
		let layout = fine_patch_cell_layout(1, IVec2::new(-1, -1));
		seed_storage(&storage, 11, &layout);
		let mut cx = GenerationContext::new(&storage);
		for OriginalId(id) in layout.cell_ids(layout.request_region()) {
			let bounds = id.origin_cell_bounds().unwrap();
			anyhow::ensure!(cx.get_or_generate::<HydroComplexCell>(id).is_some());
			let indexed = marazion_leaves_in(&mut cx, bounds);
			let discovered = leaves_via_pocket_discovery(&mut cx, bounds);
			assert_eq!(format!("{indexed:?}"), format!("{discovered:?}"), "{id:?}");
		}
		Ok(())
	}

	/// Mirrors `main`'s pocket-water discovery order (before [`HydroComplexCell`]).
	fn build_terrain_main_order(cx: &mut GenerationContext, id: Id) -> Option<()> {
		let bounds = id.origin_cell_bounds()?;
		let pre = cx.get_or_generate::<PreWatershedTerrain>(id)?;
		let high = cx.get_or_generate_all_in::<PocketWatersHighPass>(bounds)?;
		let low = cx.get_or_generate_all_in::<PocketWatersLowPass>(bounds)?;
		let marazion_leaves = high
			.iter()
			.map(|leaf| leaf.leaf_bounds())
			.chain(low.iter().map(|leaf| leaf.leaf_bounds()))
			.collect();
		let complex = cx.get_or_generate::<HydroComplexCell>(id)?.indexed().cloned();
		cx.get_or_generate::<WatershedCarvingCell>(id)?;
		cx.get_or_generate::<WatershedRimmingCell>(id)?;
		cx.get_or_generate::<WatershedAproningCell>(id)?;
		let layout = cx.get::<TerrainCellLayout>(Id::Universal).unwrap_or_default();
		let assets = cx.get_or_generate::<TerrainMeshAssets>(Id::Universal)?;
		let _terrain =
			Terrain::compose(bounds, &pre, marazion_leaves, complex, &layout, &assets);
		Some(())
	}

	fn cold_build_time(seed: u32, ids: &[Id], main_order: bool) -> std::time::Duration {
		let storage = HcsgStorage::default();
		let layout = fine_patch_cell_layout(1, IVec2::new(-1, -1));
		seed_storage(&storage, seed, &layout);
		let mut cx = GenerationContext::new(&storage);
		let start = Instant::now();
		for id in ids {
			if main_order {
				build_terrain_main_order(&mut cx, *id).expect("main-order build");
			} else {
				cx.get_or_generate::<Terrain>(*id).expect("terrain");
			}
		}
		start.elapsed()
	}

	/// `cargo test -p durham terrain_scheme_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore = "microbench"]
	fn terrain_scheme_microbench() {
		let layout = fine_patch_cell_layout(1, IVec2::new(-1, -1));
		let ids: Vec<Id> = layout
			.cell_ids(layout.request_region())
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		let count = ids.len();
		let main_order = cold_build_time(42, &ids, true);
		let optimized = cold_build_time(42, &ids, false);
		eprintln!(
			"terrain_scheme_microbench: {} cold cells main-order {:?} optimized {:?} ({:.2}x)",
			count,
			main_order,
			optimized,
			main_order.as_secs_f64() / optimized.as_secs_f64().max(1e-9)
		);
	}
}
