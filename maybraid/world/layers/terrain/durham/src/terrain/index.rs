//! Durham's view of [`HcsgStorage`]: node registration and terrain read helpers.

use crate::terrain::base_noise::BaseTerrainNoise;
use crate::terrain::cell::{
	cell_bounds, universal_bounds, TerrainCellLayout, TERRAIN_CELL_SIZE,
	TERRAIN_CELL_VERTICAL_HALF_EXTENT,
};
use crate::terrain::mesh::TerrainMeshAssets;
use crate::terrain::stamps::TerrainStampConfigs;
use crate::terrain::watersheds::WatershedConfigs;
use crate::terrain::Terrain;
use crate::water::{ComposedWater, WaterColumn, WaterMeshAssets};
use bevy::ecs::system::SystemParam;
use bevy::math::DVec3;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg;
use lod::hcsg::HcsgStorage;
use std::collections::HashMap;
use std::sync::Arc;
#[cfg(test)]
use {
	crate::terrain::cell::CellTiling, crate::terrain::geography::GeographicFeature,
	crate::water::Water, bevy::math::bounding::Aabb3d, lod::gen::Version, lod::hcsg::Busy,
	procedural_common::Bounds2,
};

/// Index scale for every Durham [`GenerationScheme`].
pub const DURHAM_INDEX_SCALE: DVec3 = DVec3::new(
	TERRAIN_CELL_SIZE as f64,
	2.0 * TERRAIN_CELL_VERTICAL_HALF_EXTENT as f64,
	TERRAIN_CELL_SIZE as f64,
);

/// Durham's seeded root inputs as live resources.
///
/// The [`crate::TerrainWindow`] producer keeps these seeded during streaming.
/// Hosts that regenerate synchronously reseed here.
#[derive(SystemParam)]
pub struct DurhamRoots<'w> {
	layout: Res<'w, TerrainCellLayout>,
	stamps: Res<'w, TerrainStampConfigs>,
	watersheds: Res<'w, WatershedConfigs>,
	terrain_assets: Res<'w, TerrainMeshAssets>,
	water_assets: Res<'w, WaterMeshAssets>,
}

impl DurhamRoots<'_> {
	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	/// Seeds Durham's session roots in the shared storage.
	pub fn seed(&self, storage: &hcsg::HcsgStorage) {
		storage.seed(self.layout.clone(), universal_bounds());
		storage.seed(self.stamps.clone(), universal_bounds());
		storage.seed(self.watersheds.clone(), universal_bounds());
		storage.seed(self.terrain_assets.clone(), universal_bounds());
		storage.seed(self.water_assets.clone(), universal_bounds());
	}
}

/// Seeds [`DurhamRoots`] during an HCSG session restart.
pub fn seed_durham_hcsg_roots(roots: DurhamRoots, storage: Res<HcsgStorage>) {
	roots.seed(storage.as_ref());
}

/// Cheap owned view of composed height fields for background consumers.
#[derive(Clone, Default)]
pub struct TerrainHeightSnapshot {
	terrain: Arc<HashMap<Id, Arc<crate::terrain::ComposedTerrain>>>,
}

impl TerrainHeightSnapshot {
	pub fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32> {
		origin_cell_ids_at(layout, x, z)
			.find_map(|id| self.terrain.get(&id))
			.map(|sdf| sdf.terrain().height_at_with_all_modulations(x, z))
	}
}

/// Cheap owned view of composed wet columns for buoyancy and placement.
#[derive(Clone, Default)]
pub struct WaterSurfaceSnapshot {
	water: Arc<HashMap<Id, Arc<ComposedWater>>>,
}

impl WaterSurfaceSnapshot {
	pub fn column(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<WaterColumn> {
		origin_cell_ids_at(layout, x, z)
			.find_map(|id| self.water.get(&id))
			.and_then(|sdf| sdf.column_at(x, z))
	}
}

/// Origin ids covering `(x, z)`: fine grid first, then outer and stream rings.
pub(crate) fn origin_cell_ids_at(
	layout: &TerrainCellLayout,
	x: f32,
	z: f32,
) -> impl Iterator<Item = Id> {
	layout.origin_ids_covering_xz(x, z).into_iter().map(|OriginalId(id)| id)
}

/// Blocking terrain reads over [`HcsgStorage`]. Tests only: frame code uses
/// [`crate::SharedTerrainStorage`].
#[cfg(test)]
pub trait TerrainStorage {
	fn terrain(&self, id: Id) -> Option<Arc<Terrain>>;

	/// Number of stored terrain origin cells.
	fn terrain_count(&self) -> usize;

	/// Advances whenever a terrain cell is stored, moved, or dropped.
	fn terrain_revision(&self) -> u64;

	/// Advances whenever a store read by [`Self::geographic_features_overlapping`]
	/// changes membership. Other writes, including other layers', leave it alone.
	fn geography_revision(&self) -> u64;

	/// Origin ids already stored in `region` (GET; does not admit missing cells).
	fn terrain_ids_overlapping(&self, region: Aabb3d) -> Vec<Id>;

	/// Every origin cell of `layout`'s request window is stored. Generation
	/// admits a few cells per frame, so a stamp read earlier misses the rest.
	fn fills_layout(&self, layout: &TerrainCellLayout) -> bool;

	fn water(&self, id: Id) -> Option<Arc<Water>>;

	fn water_version(&self, id: Id) -> Option<Version>;

	fn base_noise(&self) -> Option<Arc<BaseTerrainNoise>>;

	fn height_snapshot(&self) -> TerrainHeightSnapshot;

	fn water_snapshot(&self) -> WaterSurfaceSnapshot;

	/// Wet column at `(x, z)` when a covering water cell is stored.
	fn water_column_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<WaterColumn>;

	/// Composed terrain height (jersey + Watershed) at `(x, z)`, if that cell is stored.
	fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32>;

	/// Stored geographic sources whose leaf bounds overlap `region`.
	///
	/// Reads family-specific stamp records (skipping empty modulation lists) and
	/// authored high/low-pass watershed leaves (skipping
	/// [`crate::terrain::watersheds::PocketWater::Empty`]). Does not generate
	/// cells or walk [`HydroComplexCell`].
	fn geographic_features_overlapping(&self, region: Bounds2) -> Vec<GeographicFeature>;

	/// Store one origin cell whose SDF is `base` with no jersey / hydro ops.
	///
	/// Bounds match [`Self::composed_height_at`]'s lookup for `(ix, iz)` on `layout`.
	/// Only for composed-surface parity tests — not a production insert path.
	#[doc(hidden)]
	fn insert_base_terrain_for_test(
		&self,
		layout: &TerrainCellLayout,
		ix: i32,
		iz: i32,
		base: BaseTerrainNoise,
	);
}

#[cfg(test)]
fn test_storage_idle<T>(result: Result<T, Busy>) -> T {
	result.expect("HcsgStorage busy during unit test")
}

#[cfg(test)]
impl TerrainStorage for HcsgStorage {
	fn terrain(&self, id: Id) -> Option<Arc<Terrain>> {
		test_storage_idle(self.try_entry::<Terrain>(id)).map(|entry| entry.value)
	}

	fn terrain_count(&self) -> usize {
		test_storage_idle(self.try_overlapping::<Terrain>(universal_bounds())).len()
	}

	fn terrain_revision(&self) -> u64 {
		test_storage_idle(self.try_membership_revision::<Terrain>())
	}

	fn geography_revision(&self) -> u64 {
		test_storage_idle(crate::terrain::geography::geography_revision(self))
	}

	fn terrain_ids_overlapping(&self, region: Aabb3d) -> Vec<Id> {
		test_storage_idle(self.try_overlapping::<Terrain>(region))
	}

	fn fills_layout(&self, layout: &TerrainCellLayout) -> bool {
		layout
			.cell_ids(layout.request_region())
			.into_iter()
			.all(|OriginalId(id)| test_storage_idle(self.try_entry::<Terrain>(id)).is_some())
	}

	fn water(&self, id: Id) -> Option<Arc<Water>> {
		test_storage_idle(self.try_entry::<Water>(id)).map(|entry| entry.value)
	}

	fn water_version(&self, id: Id) -> Option<Version> {
		test_storage_idle(self.try_entry::<Water>(id)).map(|entry| entry.version)
	}

	fn base_noise(&self) -> Option<Arc<BaseTerrainNoise>> {
		test_storage_idle(self.try_entry::<BaseTerrainNoise>(Id::Universal))
			.map(|entry| entry.value)
	}

	fn height_snapshot(&self) -> TerrainHeightSnapshot {
		let region = universal_bounds();
		let terrain = test_storage_idle(self.try_overlapping::<Terrain>(region))
			.into_iter()
			.filter_map(|id| {
				test_storage_idle(self.try_entry::<Terrain>(id))
					.map(|terrain| (id, Arc::clone(&terrain.value.sdf)))
			})
			.collect();
		TerrainHeightSnapshot { terrain: Arc::new(terrain) }
	}

	fn water_snapshot(&self) -> WaterSurfaceSnapshot {
		let region = universal_bounds();
		let water = test_storage_idle(self.try_overlapping::<Water>(region))
			.into_iter()
			.filter_map(|id| {
				test_storage_idle(self.try_entry::<Water>(id))
					.map(|water| (id, Arc::new(water.value.sdf.clone())))
			})
			.collect();
		WaterSurfaceSnapshot { water: Arc::new(water) }
	}

	fn water_column_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<WaterColumn> {
		origin_cell_ids_at(layout, x, z)
			.filter_map(|id| test_storage_idle(self.try_entry::<Water>(id)))
			.find_map(|entry| entry.value.column_at(x, z))
	}

	fn composed_height_at(&self, layout: &TerrainCellLayout, x: f32, z: f32) -> Option<f32> {
		origin_cell_ids_at(layout, x, z)
			.filter_map(|id| test_storage_idle(self.try_entry::<Terrain>(id)))
			.map(|entry| entry.value.sdf.terrain().height_at_with_all_modulations(x, z))
			.next()
	}

	fn geographic_features_overlapping(&self, region: Bounds2) -> Vec<GeographicFeature> {
		test_storage_idle(crate::terrain::geography::geographic_features_overlapping(self, region))
	}

	fn insert_base_terrain_for_test(
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

impl Terrain {
	/// Origin cell `(ix, iz)` of `layout` whose SDF is `base` with no jersey
	/// or hydro ops. Only for surface tests.
	#[doc(hidden)]
	pub fn base_cell_for_test(
		layout: &TerrainCellLayout,
		ix: i32,
		iz: i32,
		base: BaseTerrainNoise,
	) -> Self {
		let cell = cell_bounds(ix, iz, layout.cell_size, layout.vertical_half_extent);
		let sdf = Arc::new(Terrain::compose_sdf(&base, &[]));
		Terrain {
			cell,
			base,
			modulations: Vec::new(),
			jersey_leaves: Vec::new(),
			marazion_leaves: Vec::new(),
			marazion_fills: Vec::new(),
			sdf,
			material: Handle::default(),
			res_2: 0,
			stream_ring: None,
			wall_faces: render_item::sdf::cpu_shot::WallFaces::NONE,
		}
	}
}

#[cfg(test)]
pub(crate) fn insert_water_for_test(storage: &HcsgStorage, water: Water) {
	let bounds = water.cell;
	storage.publish(Id::from_cell(bounds), Arc::new(water), bounds);
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::config::TerrainConfig;

	#[test]
	fn clearing_durham_nodes_advances_versions_past_the_previous_epoch() -> anyhow::Result<()> {
		let storage = HcsgStorage::default();
		let layout = TerrainCellLayout::default();
		storage.insert_base_terrain_for_test(
			&layout,
			0,
			0,
			BaseTerrainNoise::from_config(&TerrainConfig::new(7)),
		);
		let id = storage.terrain_ids_overlapping(layout.request_region())[0];
		let previous = test_storage_idle(storage.try_entry::<Terrain>(id))
			.ok_or_else(|| anyhow::anyhow!("inserted"))?
			.version;
		storage.clear_derived();
		assert_eq!(storage.terrain_count(), 0);
		storage.insert_base_terrain_for_test(
			&layout,
			0,
			0,
			BaseTerrainNoise::from_config(&TerrainConfig::new(7)),
		);
		let rebuilt = test_storage_idle(storage.try_entry::<Terrain>(id))
			.ok_or_else(|| anyhow::anyhow!("reinserted"))?
			.version;
		assert!(rebuilt > previous, "restamp must not reuse a presented version");
		Ok(())
	}
}
