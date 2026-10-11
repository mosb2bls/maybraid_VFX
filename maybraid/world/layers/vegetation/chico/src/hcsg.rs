//! Chico on the shared HCSG runtime ([`lod::hcsg`]): forests and
//! groves select on the worker, and each grove grows there on the ground's
//! surface cells into a [`GrownGrove`] presented as an [`HcsgNode`] host.
//! Beyond the groves, [`BumpedOut`] canopy proxies displace the surface.
//!
//! A session starts with [`lod::hcsg::request_hcsg_session_restart`].

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use durham::{TerrainCellLayout, TerrainMeshBuilder};
use lod::gen::{Id, OriginalId};
use lod::hcsg::universal_bounds;
use lod::hcsg::{
	self, register_session_seed, GenerationContext, HcsgClass, PresentationPlugin, ViewerHcsgBounds,
};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use procedural_common::union_aabb3_iter;
use render_item::mesh::{IdentifiedMesh, MeshBuilder};
use render_item::NormalizeChunk;
use richmond::{PaddedTerrain, Richmond, RichmondGround};
use terrain_chunk_ref::TerrainChunkRef;
use urbanization_layer_model::Urbanization;
use vegetation_components::scene_children;
use vegetation_groves::{GroveExtent, GroveWorldSample};

use crate::ground::overlay_chunk_ref;
use crate::grove::{grove_from_id, grove_id};
use crate::plugin::register_vegetation_view;
use crate::{
	presenting_recipes, ChicoForest, ChicoGrove, ChicoGroveHost, ForestExtent, ForestGroveTile,
	ForestLayer, ForestSelection, NeighborLayers, DEFAULT_FOREST_EXTENT_XZ,
	DEFAULT_FOREST_GROVE_TILE_XZ, GROVE_PRESENT_RADIUS_M,
};

mod bump_outs;

pub use bump_outs::{BumpOutPresentationPlugin, BumpOutRing, BumpedOut, CanopyProxy};

/// The ground forests grow on: the cells that make up its whole surface.
pub trait ForestGround: Send + Sync + 'static {
	type Surface: hcsg::GenerationScheme;
	type Mesh: MeshBuilder + IdentifiedMesh + NormalizeChunk + Clone + Send + Sync + 'static;

	/// The cell's footprint.
	fn footprint(surface: &Self::Surface) -> Aabb3d;

	/// Surface height at `(x, z)` within the cell.
	fn height_at(surface: &Self::Surface, x: f32, z: f32) -> f32;

	/// The cell's mesh, keyed as the cell's own host presents it.
	fn chunk_ref(surface: &Self::Surface) -> TerrainChunkRef<Self::Mesh>;

	/// Surface ids that may carry proxy `cell`.
	///
	/// Defaults to the surface scheme's own discovery (the finest lattice on a
	/// streamed layout). Grounds with several cell sizes must also include each
	/// coarser lattice that covers `cell`, so a far-grid proxy can generate the
	/// 320 m surface the far ring presents.
	fn surface_ids_for(cx: &mut GenerationContext, cell: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<Self::Surface>(cell)
	}
}

/// Padded terrain is the urbanized ground's whole surface.
impl<T: RichmondGround> ForestGround for Urbanization<Richmond<T>> {
	type Surface = PaddedTerrain<T>;
	type Mesh = TerrainMeshBuilder;

	fn footprint(surface: &Self::Surface) -> Aabb3d {
		surface.surface.cell
	}

	fn height_at(surface: &Self::Surface, x: f32, z: f32) -> f32 {
		surface.surface.sdf.terrain().height_at_with_all_modulations(x, z)
	}

	fn chunk_ref(surface: &Self::Surface) -> TerrainChunkRef<TerrainMeshBuilder> {
		overlay_chunk_ref(&surface.surface)
	}

	fn surface_ids_for(cx: &mut GenerationContext, cell: Aabb3d) -> Vec<OriginalId> {
		let mut ids = cx.original_ids_for::<Self::Surface>(cell);
		let Some(layout) = cx.get_or_generate::<TerrainCellLayout>(Id::Universal) else {
			return ids;
		};
		// Finest-lattice ids already come from `original_ids_for`. Adding them
		// again would invent unbounded fine cells outside a windowed layout.
		let finest = layout
			.stream_rings
			.iter()
			.map(|ring| ring.cell_size)
			.min_by(|a, b| a.total_cmp(b))
			.unwrap_or(layout.cell_size);
		let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
		for id in layout.origin_ids_covering_xz(center.x, center.z) {
			let Some(width) = id.0.origin_cell_bounds().map(|bounds| bounds.max.x - bounds.min.x)
			else {
				continue;
			};
			if width <= finest + 1e-2 || ids.contains(&id) {
				continue;
			}
			ids.push(id);
		}
		ids
	}
}

impl hcsg::GenerationScheme for ChicoForest {
	lod::hcsg_index_scale!(FOREST_SCALE);

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		ForestExtent::original_ids_overlapping(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = ForestExtent::from_id(id)?;
		let selection = cx.get::<ForestSelection>(Id::Universal)?;
		Some((Self { extent, layers: selection.layers_for(extent) }, extent.aabb()))
	}
}

/// The forest cell holding `tile`.
fn forest_of(tile: GroveExtent) -> ForestExtent {
	let (ix, iz) = ForestExtent::cell_index_containing((tile.min() + tile.max()) * 0.5);
	ForestExtent::from_cell_index(ix, iz)
}

fn push_grove_ids_for_tile(
	ids: &mut Vec<OriginalId>,
	tile: GroveExtent,
	forest: &ChicoForest,
	neighbors: &NeighborLayers,
) {
	for layer in ForestLayer::ALL {
		if layer.kind(forest.layers).is_some() || neighbors.any_kind(layer) {
			ids.push(OriginalId(grove_id(tile, layer)));
		}
	}
}

/// The forest cell over `forest` and its four cardinal neighbors' layers.
fn forest_with_neighbors(
	cx: &mut GenerationContext,
	forest: ForestExtent,
) -> Option<(Arc<ChicoForest>, NeighborLayers)> {
	let selected = cx.get_or_generate::<ChicoForest>(forest.id())?;
	let (ix, iz) = ForestExtent::cell_index_containing(forest.center());
	let mut layers = |ix, iz| {
		cx.get_or_generate::<ChicoForest>(ForestExtent::from_cell_index(ix, iz).id())
			.map(|neighbor| neighbor.layers)
	};
	let neighbors = NeighborLayers {
		north: layers(ix, iz + 1),
		east: layers(ix + 1, iz),
		south: layers(ix, iz - 1),
		west: layers(ix - 1, iz),
	};
	Some((selected, neighbors))
}

impl hcsg::GenerationScheme for ChicoGrove {
	lod::hcsg_index_scale!(GROVE_SCALE);

	/// One grove per tile and layer that the tile's forest or a neighbor
	/// selects, so blends reach across forest faces.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = Vec::new();
		let mut tiles_by_forest: HashMap<Id, Vec<GroveExtent>> = HashMap::new();
		for tile in ForestExtent::grove_tiles_overlapping(region) {
			tiles_by_forest.entry(forest_of(tile).id()).or_default().push(tile);
		}
		for tiles in tiles_by_forest.values() {
			let forest_extent = forest_of(tiles[0]);
			let Some((forest, neighbors)) = forest_with_neighbors(cx, forest_extent) else {
				continue;
			};
			for &tile in tiles {
				push_grove_ids_for_tile(&mut ids, tile, &forest, &neighbors);
			}
		}
		ids
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (extent, layer) = grove_from_id(id)?;
		let forest_extent = forest_of(extent);
		let (forest, neighbors) = forest_with_neighbors(cx, forest_extent)?;
		let kind = layer.kind(forest.layers);
		let recipes = presenting_recipes(kind, extent, forest_extent, &neighbors, layer);
		let grove = Self::selected(extent, layer, recipes);
		let bounds = grove.aabb();
		Some((grove, bounds))
	}
}

/// The surface cells of ground `G` over a region, finest first. Groves place
/// plants only where a cell covers them.
pub struct GroundSurface<G: ForestGround> {
	cells: Vec<Arc<G::Surface>>,
}

impl<G: ForestGround> GroundSurface<G> {
	pub fn generate(cx: &mut GenerationContext, region: Aabb3d) -> Self {
		let mut cells: Vec<Arc<G::Surface>> = cx
			.original_ids_for::<G::Surface>(region)
			.into_iter()
			.filter_map(|OriginalId(id)| cx.get_or_generate::<G::Surface>(id))
			.collect();
		cells.sort_by(|a, b| span_x(G::footprint(a)).total_cmp(&span_x(G::footprint(b))));
		Self { cells }
	}

	pub fn is_empty(&self) -> bool {
		self.cells.is_empty()
	}

	/// The union of the cells' footprints.
	pub fn footprint(&self) -> Option<Aabb3d> {
		union_aabb3_iter(self.cells.iter().map(|cell| G::footprint(cell)))
	}

	/// Surface height at `(x, z)`; `None` off the surface.
	pub fn height(&self, x: f32, z: f32) -> Option<f32> {
		self.cells
			.iter()
			.find(|cell| {
				let bounds = G::footprint(cell);
				x >= bounds.min.x && x <= bounds.max.x && z >= bounds.min.z && z <= bounds.max.z
			})
			.map(|cell| G::height_at(cell, x, z))
	}
}

fn span_x(bounds: Aabb3d) -> f32 {
	bounds.max.x - bounds.min.x
}

/// Steepness is a 1 m forward difference, as on the legacy ground sample; a
/// neighbor off the surface reads level.
impl<G: ForestGround> GroveWorldSample for GroundSurface<G> {
	fn height_at(&self, position: Vec3) -> f32 {
		self.height(position.x, position.z).unwrap_or(0.0)
	}

	fn steepness_at(&self, position: Vec3) -> f32 {
		const EPS: f32 = 1.0;
		let h = self.height_at(position);
		let hx = self.height(position.x + EPS, position.z).unwrap_or(h);
		let hz = self.height(position.x, position.z + EPS).unwrap_or(h);
		let (dx, dz) = ((hx - h) / EPS, (hz - h) / EPS);
		(dx * dx + dz * dz).sqrt()
	}

	fn allows_placement_at(&self, position: Vec3) -> bool {
		self.height(position.x, position.z).is_some()
	}
}

/// A [`ChicoGrove`] grown on ground `G`'s surface, under the grove's id.
pub struct GrownGrove<G> {
	pub layer: ForestLayer,
	pub tiles: Vec<ForestGroveTile>,
	_ground: PhantomData<fn() -> G>,
}

impl<G: ForestGround> hcsg::GenerationScheme for GrownGrove<G> {
	lod::hcsg_index_scale!(GROWN_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<ChicoGrove>(region)
	}

	/// `None` off the ground or where nothing grows.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let grove = cx.get_or_generate::<ChicoGrove>(id)?;
		let extent = Aabb3d::from_min_max(grove.extent.min(), grove.extent.max());
		let surface = GroundSurface::<G>::generate(cx, extent);
		if surface.is_empty() {
			return None;
		}
		let tiles = grove.grow(&surface);
		let bounds = union_aabb3_iter(
			tiles
				.iter()
				.map(ForestGroveTile::scene_bounds)
				.filter(|bounds| bounds.min.is_finite() && bounds.max.is_finite()),
		)?;
		Some((Self { layer: grove.layer, tiles, _ground: PhantomData }, bounds))
	}
}

/// One level holding a nested [`ChicoGroveHost`] per tile; each tile keeps
/// its own levels.
impl<G: ForestGround> LodScene for GrownGrove<G> {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		scene_children(
			self.tiles
				.iter()
				.map(|tile| {
					let host = ChicoGroveHost::new(tile.clone(), self.layer);
					Box::new(host.scene(lod_ref)) as Box<dyn Scene>
				})
				.collect(),
		)
	}
}

/// Half-height of a grove neighborhood: every surface a grove grows on.
const GROVE_COLUMN_Y: f32 = 10_000.0;

/// The groves around the [`LodViewer`]: within [`GROVE_PRESENT_RADIUS_M`] of
/// its grove tile.
pub struct GroveNeighborhood;

impl GroveNeighborhood {
	fn around(viewer: Vec3, radius: f32) -> Aabb3d {
		let s = DEFAULT_FOREST_GROVE_TILE_XZ;
		let origin = -DEFAULT_FOREST_EXTENT_XZ * 0.5;
		let snap = |v: f32| origin + (((v - origin) / s).floor() + 0.5) * s;
		let center = Vec3::new(snap(viewer.x), 0.0, snap(viewer.z));
		Aabb3d::from_min_max(
			Vec3::new(center.x - radius, -GROVE_COLUMN_Y, center.z - radius),
			Vec3::new(center.x + radius, GROVE_COLUMN_Y, center.z + radius),
		)
	}
}

impl ViewerHcsgBounds for GroveNeighborhood {
	const CLASS: HcsgClass = HcsgClass::Near;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		vec![Self::around(viewer, GROVE_PRESENT_RADIUS_M)]
	}
}

const FOREST_SCALE: DVec3 =
	DVec3::new(DEFAULT_FOREST_EXTENT_XZ as f64, 1.0, DEFAULT_FOREST_EXTENT_XZ as f64);

/// Grove ids stack the four layers on Y.
const GROVE_SCALE: DVec3 =
	DVec3::new(DEFAULT_FOREST_GROVE_TILE_XZ as f64, 4.0, DEFAULT_FOREST_GROVE_TILE_XZ as f64);

const GROWN_SCALE: DVec3 =
	DVec3::new(DEFAULT_FOREST_GROVE_TILE_XZ as f64, 256.0, DEFAULT_FOREST_GROVE_TILE_XZ as f64);

/// Chico's root resource: the forest selection.
#[derive(SystemParam)]
pub struct ChicoRoots<'w> {
	selection: Res<'w, ForestSelection>,
}

impl ChicoRoots<'_> {
	/// Seeds Chico's forest selection root over ground `G`.
	pub fn seed<G: ForestGround>(&self, storage: &hcsg::HcsgStorage) {
		storage.seed(*self.selection, universal_bounds());
	}
}

/// Seeds [`ChicoRoots`] during an HCSG session restart.
pub fn seed_chico_hcsg_roots<G: ForestGround>(roots: ChicoRoots, storage: Res<hcsg::HcsgStorage>) {
	roots.seed::<G>(storage.as_ref());
}

/// Presents groves grown on ground `G` within channel `C`'s regions from the
/// shared storage.
pub struct ChicoPresentationPlugin<C, G>(PhantomData<fn() -> (C, G)>);

impl<C, G> Default for ChicoPresentationPlugin<C, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, G: ForestGround> Plugin for ChicoPresentationPlugin<C, G> {
	fn build(&self, app: &mut App) {
		register_vegetation_view(app);
		app.init_resource::<ForestSelection>();
		register_session_seed(app, seed_chico_hcsg_roots::<G>);
		app.add_plugins(PresentationPlugin::<C, GrownGrove<G>>::default());
	}
}

#[cfg(test)]
mod tests {
	use std::collections::{BTreeMap, BTreeSet};
	use std::time::Duration;

	use bevy::ecs::entity_disabling::Disabled;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use durham::{
		fine_patch_cell_layout, mesh_assets, playable_world_cell_layout, Durham, DurhamWindow,
		TerrainConfig, TerrainCoverage, TerrainMeshAssets, TerrainMeshLodBand, TerrainStampConfigs,
		WaterMeshAssets, WaterPresentationPlugin, WatershedConfigs, WORLD_FINE_HALF_EXTENT_CELLS,
	};
	use lod::gen::Version;
	use lod::hcsg::{
		GenerationScheme, HcsgClass, HcsgNode, HcsgRestartRequest, SettlePresentation,
		ViewerHcsgBounds, settle_presentation,
	};
	use lod::lod_ref::LodNodePose;
	use lod::LodViewer;
	use richmond::BuiltPresentationPlugin;
	use richmond::{AuthoredDevelopments, DevelopmentConfig, DevelopmentSites, PaddedTerrain};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::UrbanizationSelection;

	use super::*;
	use crate::{
		CanopyBumpOut, LayeringKind, MediumCanopyBumpOut, BUMP_OUT_CELL_XZ, MEDIUM_BUMP_OUT_CELL_XZ,
	};

	type Ground = OnTerrain<Durham>;
	type Urban = Urbanization<Richmond<Ground>>;

	const IDLE: Duration = Duration::from_secs(300);

	/// The 2×2 fine patch's footprint.
	const PATCH: f32 = 160.0;

	fn select(app: &mut App, layering: LayeringKind) {
		app.insert_resource(ForestSelection {
			layering: Some(layering),
			..ForestSelection::default()
		})
		.world_mut()
		.resource_mut::<HcsgRestartRequest>()
		.request();
	}

	/// Groves on a 2×2 fine patch at the origin, viewed from `at`.
	fn app(at: Vec3) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
			.insert_resource(TerrainStampConfigs::from_world_seed(1))
			.insert_resource(WatershedConfigs::default().with_seed(1))
			.insert_resource(TerrainMeshAssets {
				config: TerrainConfig::new(1),
				material: Handle::default(),
				lod_bands: vec![TerrainMeshLodBand { max_radius_cells: 1, res_2: 2 }],
				outer_add_walls: false,
				fine_grid_max_radius: Some(1),
				macro_seam_half_extents: Vec::new(),
				macro_cell_min_size: None,
				macro_res_2: None,
			})
			.insert_resource(WaterMeshAssets { material: Handle::default() })
			.insert_resource(DevelopmentConfig {
				sites: DevelopmentSites::Authored,
				..DevelopmentConfig::default()
			})
			.init_resource::<AuthoredDevelopments>()
			.init_resource::<UrbanizationSelection>()
			.insert_resource(HcsgRestartRequest::queued())
			.add_plugins((
				hcsg::HcsgBoundsPlugin::<GroveNeighborhood>::default(),
				hcsg::HcsgBoundsPlugin::<BumpOutRing<CanopyBumpOut>>::default(),
				WaterPresentationPlugin::<DurhamWindow>::default(),
				BuiltPresentationPlugin::<GroveNeighborhood, Ground>::default(),
			))
			.add_plugins(ChicoPresentationPlugin::<GroveNeighborhood, Urban>::default())
			.add_plugins(BumpOutPresentationPlugin::<
				BumpOutRing<CanopyBumpOut>,
				CanopyBumpOut,
				Urban,
			>::default());
		select(&mut app, LayeringKind::LushJungle);
		durham::register_durham_hcsg_session(&mut app);
		app.finish();
		app.cleanup();
		let at = Transform::from_translation(at);
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
		app
	}

	fn settle(app: &mut App) -> anyhow::Result<()> {
		settle_presentation(app, SettlePresentation::layer_integration(IDLE)).map_err(Into::into)
	}

	fn nested_hosts(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<(), (With<ChicoGroveHost>, Allow<Disabled>)>()
			.iter(app.world())
			.count()
	}

	fn grown(app: &mut App) -> Vec<HcsgNode<GrownGrove<Urban>>> {
		let mut hosts: Vec<_> = app
			.world_mut()
			.query::<&HcsgNode<GrownGrove<Urban>>>()
			.iter(app.world())
			.cloned()
			.collect();
		hosts.sort_by_key(|node| node.id);
		hosts
	}

	fn versions(app: &mut App) -> Vec<(Id, Version)> {
		grown(app).iter().map(|node| (node.id, node.version)).collect()
	}

	fn within_patch(tile: GroveExtent) -> bool {
		tile.min().x >= -PATCH
			&& tile.max().x <= PATCH
			&& tile.min().z >= -PATCH
			&& tile.max().z <= PATCH
	}

	/// The layers grown on each tile the patch wholly covers.
	fn covered_layers(app: &mut App) -> anyhow::Result<BTreeMap<Id, BTreeSet<u8>>> {
		let mut tiles = BTreeMap::<Id, BTreeSet<u8>>::new();
		for node in grown(app) {
			let (tile, layer) =
				grove_from_id(node.id).ok_or_else(|| anyhow::anyhow!("grove id"))?;
			if within_patch(tile) {
				let key = grove_id(tile, ForestLayer::Tufts);
				tiles.entry(key).or_default().insert(layer.id_y() as u8);
			}
		}
		Ok(tiles)
	}

	fn selected_layers(layering: LayeringKind) -> BTreeSet<u8> {
		let layers = layering.layering().typical_layers();
		ForestLayer::ALL
			.into_iter()
			.filter(|layer| layer.kind(layers).is_some())
			.map(|layer| layer.id_y() as u8)
			.collect()
	}

	#[test]
	fn the_neighborhood_grows_the_selected_layers_on_the_ground() -> anyhow::Result<()> {
		let mut app = app(Vec3::ZERO);
		settle(&mut app)?;

		let hosts = grown(&mut app);
		assert!(!hosts.is_empty(), "the patch grows groves");
		for node in &hosts {
			let (tile, _) = grove_from_id(node.id).ok_or_else(|| anyhow::anyhow!("grove id"))?;
			assert!(
				tile.min().x < PATCH
					&& tile.max().x > -PATCH
					&& tile.min().z < PATCH
					&& tile.max().z > -PATCH,
				"{tile:?} grew off the ground"
			);
		}

		let covered = covered_layers(&mut app)?;
		assert_eq!(covered.len(), 4, "every tile the patch covers grows");
		for layers in covered.values() {
			assert_eq!(*layers, selected_layers(LayeringKind::LushJungle));
		}

		let tiles: usize = hosts.iter().map(|node| node.value.tiles.len()).sum();
		for _ in 0..200 {
			if nested_hosts(&mut app) >= tiles {
				break;
			}
			app.update();
		}
		assert_eq!(nested_hosts(&mut app), tiles, "one nested host per grown tile");
		Ok(())
	}

	#[test]
	fn groves_stand_on_the_surface() -> anyhow::Result<()> {
		let mut app = app(Vec3::ZERO);
		settle(&mut app)?;

		let storage = app.world().resource::<hcsg::HcsgStorage>().clone();
		let mut checked = 0;
		for node in grown(&mut app) {
			let mut cx = GenerationContext::new(&storage);
			let (tile, _) = grove_from_id(node.id).ok_or_else(|| anyhow::anyhow!("grove id"))?;
			let region = Aabb3d::from_min_max(tile.min(), tile.max());
			let surface = GroundSurface::<Urban>::generate(&mut cx, region);
			for grown in &node.value.tiles {
				let ForestGroveTile::TradeWinds(grove) = grown else {
					continue;
				};
				for plant in grove.plants.iter() {
					let at = plant.placement.translation;
					let ground = surface
						.height(at.x, at.z)
						.ok_or_else(|| anyhow::anyhow!("{at} grew off the ground"))?;
					assert!((at.y - ground).abs() < 0.5, "{at} stands off the ground at {ground}");
					checked += 1;
				}
			}
		}
		assert!(checked > 0, "the upper canopy grows trade winds");
		Ok(())
	}

	/// Grove High is hundreds of metres; plant High is ~10× tree radius. Fulfill
	/// at the tile High edge must still pose High canopy kits — colliders already
	/// stamp High IR from the ancestor grove band.
	#[test]
	fn high_band_trees_present_full_canopy() -> anyhow::Result<()> {
		let at = Vec3::new(280.0, 20.0, 0.0);
		let mut app = app(at);
		settle(&mut app)?;

		let tiles: usize = grown(&mut app).iter().map(|node| node.value.tiles.len()).sum();
		update_until(&mut app, |app| nested_hosts(app) >= tiles);
		assert!(nested_hosts(&mut app) > 0, "the patch presents grove hosts");
		for _ in 0..240 {
			app.update();
		}

		let compounds = app
			.world_mut()
			.query::<&crate::stick_physics::StickPhysicsCompound>()
			.iter(app.world())
			.count();
		assert!(compounds > 0, "High/Medium groves still stamp stick colliders");

		let mut high_canopy = 0usize;
		let mut thinned_canopy = 0usize;
		for merge in app.world_mut().query::<&scene_ref::MultiSceneMergeRoot>().iter(app.world()) {
			let Some(path) = merge.0.parts.first().map(|part| part.scene.path.as_str()) else {
				continue;
			};
			if !path.contains("cheap_ball") {
				continue;
			}
			if path.contains("high_res") {
				high_canopy += 1;
			} else {
				thinned_canopy += 1;
			}
		}
		assert!(
			high_canopy > 0,
			"High-band trees should pose High cheap-ball canopy kits; got {high_canopy} high / {thinned_canopy} thinned"
		);
		Ok(())
	}

	#[test]
	fn leaving_the_neighborhood_retires_its_hosts() -> anyhow::Result<()> {
		let mut app = app(Vec3::ZERO);
		settle(&mut app)?;
		assert!(!grown(&mut app).is_empty());

		let away = Transform::from_xyz(5_000.0, 0.0, 5_000.0);
		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = away;
		settle(&mut app)?;
		assert!(grown(&mut app).is_empty());
		Ok(())
	}

	#[test]
	fn a_restart_reselects_every_grove() -> anyhow::Result<()> {
		let mut app = app(Vec3::ZERO);
		settle(&mut app)?;
		let before: BTreeMap<Id, Version> = versions(&mut app).into_iter().collect();

		select(&mut app, LayeringKind::Meadowland);
		settle(&mut app)?;

		for layers in covered_layers(&mut app)?.values() {
			assert_eq!(*layers, selected_layers(LayeringKind::Meadowland));
		}
		for (id, version) in versions(&mut app) {
			if let Some(old) = before.get(&id) {
				assert!(version > *old, "{id:?} still shows the previous session");
			}
		}
		Ok(())
	}

	type Fine = BumpedOut<CanopyBumpOut, Urban>;

	/// Beyond the grove hole, east of the patch.
	const BEYOND: Vec3 = Vec3::new(1_500.0, 0.0, 0.0);

	fn bumped_out(app: &mut App) -> Vec<HcsgNode<Fine>> {
		app.world_mut().query::<&HcsgNode<Fine>>().iter(app.world()).cloned().collect()
	}

	/// Shown bump-outs whose surface mesh has resolved.
	fn shown_bump_outs(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<(), (With<vegetation_bumpout::BumpOut>, With<terrain_chunk_ref::TerrainChunkRefResolved>)>()
			.iter(app.world())
			.count()
	}

	fn update_until(app: &mut App, done: impl Fn(&mut App) -> bool) {
		for _ in 0..200 {
			if done(app) {
				return;
			}
			app.update();
		}
	}

	fn move_viewer(app: &mut App, to: Vec3) -> anyhow::Result<()> {
		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = Transform::from_translation(to);
		Ok(())
	}

	#[test]
	fn bump_outs_beyond_the_groves_displace_the_surface_mesh() -> anyhow::Result<()> {
		let mut app = app(BEYOND);
		settle(&mut app)?;

		let nodes = bumped_out(&mut app);
		assert_eq!(nodes.len(), 4, "one bump-out per fine cell on the patch");
		let storage = app.world().resource::<hcsg::HcsgStorage>().clone();
		for node in &nodes {
			let mut cx = GenerationContext::new(&storage);
			let cell = node.value.cell;
			let under: Vec<_> = cx
				.original_ids_for::<PaddedTerrain<Ground>>(cell)
				.into_iter()
				.filter_map(|OriginalId(id)| cx.get_or_generate::<PaddedTerrain<Ground>>(id))
				.filter(|surface| {
					let footprint = Urban::footprint(surface);
					footprint.min.xz() == cell.min.xz() && footprint.max.xz() == cell.max.xz()
				})
				.collect();
			let [surface] = under.as_slice() else {
				anyhow::bail!("{cell:?} lies on one fine surface cell");
			};
			assert_eq!(node.value.terrain.key(), Urban::chunk_ref(surface).key());
		}

		update_until(&mut app, |app| shown_bump_outs(app) >= 4);
		assert_eq!(shown_bump_outs(&mut app), 4, "every bump-out resolves its surface");
		Ok(())
	}

	#[test]
	fn the_grove_hole_hides_bump_outs() -> anyhow::Result<()> {
		let mut app = app(Vec3::ZERO);
		settle(&mut app)?;
		assert_eq!(bumped_out(&mut app).len(), 4, "the hole keeps its bump-outs warm");
		assert_eq!(shown_bump_outs(&mut app), 0);

		move_viewer(&mut app, BEYOND)?;
		settle(&mut app)?;
		update_until(&mut app, |app| shown_bump_outs(app) >= 4);
		assert_eq!(shown_bump_outs(&mut app), 4);

		move_viewer(&mut app, Vec3::ZERO)?;
		settle(&mut app)?;
		update_until(&mut app, |app| shown_bump_outs(app) == 0);
		assert_eq!(shown_bump_outs(&mut app), 0);
		Ok(())
	}

	type Medium = BumpedOut<MediumCanopyBumpOut, Urban>;

	/// One 320 m far-ring cell, 1.6–1.92 km east of the origin.
	fn far_grid_cell() -> Aabb3d {
		MediumCanopyBumpOut::cell_bounds(5, 0)
	}

	/// Discover only [`far_grid_cell`] so the worker generates one far bump-out.
	struct FarRingBumpOutProbe;

	impl ViewerHcsgBounds for FarRingBumpOutProbe {
		const CLASS: HcsgClass = HcsgClass::Near;

		fn regions_around(_viewer: Vec3) -> Vec<Aabb3d> {
			let cell = far_grid_cell();
			vec![Aabb3d::from_min_max(
				Vec3::new(cell.min.x + 1.0, -10.0, cell.min.z + 1.0),
				Vec3::new(cell.max.x - 1.0, 10.0, cell.max.z - 1.0),
			)]
		}
	}

	fn seed_playable_bump_out_storage() -> HcsgStorage {
		let storage = HcsgStorage::default();
		storage.seed(playable_world_cell_layout(), universal_bounds());
		storage.seed(TerrainStampConfigs::from_world_seed(42), universal_bounds());
		storage.seed(WatershedConfigs::default().with_seed(42), universal_bounds());
		storage.seed(
			mesh_assets(
				TerrainConfig::new(42),
				Handle::default(),
				TerrainCoverage::PlayableWorld,
				WORLD_FINE_HALF_EXTENT_CELLS,
			),
			universal_bounds(),
		);
		storage.seed(
			ForestSelection {
				layering: Some(LayeringKind::LushJungle),
				..ForestSelection::default()
			},
			universal_bounds(),
		);
		storage
	}

	fn playable_far_bump_out_app(at: Vec3) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.insert_resource(playable_world_cell_layout())
			.insert_resource(TerrainStampConfigs::from_world_seed(1))
			.insert_resource(WatershedConfigs::default().with_seed(1))
			.insert_resource(mesh_assets(
				TerrainConfig::new(1),
				Handle::default(),
				TerrainCoverage::PlayableWorld,
				WORLD_FINE_HALF_EXTENT_CELLS,
			))
			.insert_resource(WaterMeshAssets { material: Handle::default() })
			.insert_resource(DevelopmentConfig {
				sites: DevelopmentSites::Authored,
				..DevelopmentConfig::default()
			})
			.init_resource::<AuthoredDevelopments>()
			.init_resource::<UrbanizationSelection>()
			.insert_resource(HcsgRestartRequest::queued());
		register_vegetation_view(&mut app);
		register_session_seed(&mut app, seed_chico_hcsg_roots::<Urban>);
		app.add_plugins((
			hcsg::HcsgBoundsPlugin::<FarRingBumpOutProbe>::default(),
			BumpOutPresentationPlugin::<FarRingBumpOutProbe, MediumCanopyBumpOut, Urban>::default(),
		));
		select(&mut app, LayeringKind::LushJungle);
		durham::register_durham_hcsg_session(&mut app);
		app.finish();
		app.cleanup();
		let at = Transform::from_translation(at);
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
		app
	}

	fn medium_bumped_out(app: &mut App) -> Vec<HcsgNode<Medium>> {
		app.world_mut()
			.query::<&HcsgNode<Medium>>()
			.iter(app.world())
			.cloned()
			.collect()
	}

	/// Finest-lattice `original_ids_for` never yields the 320 m far cell.
	#[test]
	fn finest_lattice_discovery_misses_far_grid_surfaces() -> anyhow::Result<()> {
		let storage = seed_playable_bump_out_storage();
		let mut cx = GenerationContext::new(&storage);
		let cell = far_grid_cell();
		let finest = cx.original_ids_for::<PaddedTerrain<Ground>>(cell);
		let covering = Urban::surface_ids_for(&mut cx, cell);
		let width_of = |id: Id| id.origin_cell_bounds().map(|bounds| bounds.max.x - bounds.min.x);
		assert!(
			finest.iter().all(|OriginalId(id)| {
				width_of(*id).is_some_and(|width| (width - BUMP_OUT_CELL_XZ).abs() < 1e-2)
			}),
			"Terrain origin_ids stay on the near lattice"
		);
		assert!(
			covering.iter().any(|OriginalId(id)| {
				width_of(*id).is_some_and(|width| (width - MEDIUM_BUMP_OUT_CELL_XZ).abs() < 1e-2)
			}),
			"surface_ids_for must include the 320 m far cell"
		);
		Ok(())
	}

	#[test]
	fn far_grid_bump_outs_generate_on_320m_surfaces() -> anyhow::Result<()> {
		let storage = seed_playable_bump_out_storage();
		let mut cx = GenerationContext::new(&storage);
		let cell = far_grid_cell();
		let id = Id::from_cell(cell);
		let (built, bounds) = Medium::build_with_id(&mut cx, id)
			.ok_or_else(|| anyhow::anyhow!("far-grid bump-out at {cell:?}"))?;
		assert_eq!(built.cell.min.xz(), cell.min.xz());
		assert_eq!(built.cell.max.xz(), cell.max.xz());
		assert!(
			(bounds.max.x - bounds.min.x - MEDIUM_BUMP_OUT_CELL_XZ).abs() < 1e-2,
			"attached surface must be the 320 m far cell, got width {}",
			bounds.max.x - bounds.min.x
		);
		Ok(())
	}

	#[test]
	fn far_ring_bump_outs_are_presented_at_far_ring_distance() -> anyhow::Result<()> {
		let mut app = playable_far_bump_out_app(Vec3::ZERO);
		settle(&mut app)?;

		let nodes = medium_bumped_out(&mut app);
		assert_eq!(nodes.len(), 1, "the far-ring probe presents its 320 m bump-out");
		update_until(&mut app, |app| shown_bump_outs(app) >= 1);
		assert_eq!(shown_bump_outs(&mut app), 1, "the far-ring bump-out shows at 1.76 km");
		Ok(())
	}

	use lod::gen::OriginalId;
	use lod::hcsg::{universal_bounds, GenerationContext, HcsgStorage};

	use super::{
		forest_of, forest_with_neighbors, push_grove_ids_for_tile, ChicoGrove, ForestSelection,
		GroveNeighborhood, GROVE_PRESENT_RADIUS_M,
	};

	/// Per-tile `forest_with_neighbors` (pre-optimization discovery).
	fn naive_grove_original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let mut ids = Vec::new();
		for tile in ForestExtent::grove_tiles_overlapping(region) {
			let Some((forest, neighbors)) = forest_with_neighbors(cx, forest_of(tile)) else {
				continue;
			};
			push_grove_ids_for_tile(&mut ids, tile, &forest, &neighbors);
		}
		ids
	}

	#[test]
	fn grove_discovery_matches_per_tile_forest_lookup() {
		let region = GroveNeighborhood::around(Vec3::ZERO, GROVE_PRESENT_RADIUS_M);
		let selection = ForestSelection {
			layering: Some(LayeringKind::LushJungle),
			..ForestSelection::default()
		};
		let storage = HcsgStorage::default();
		storage.seed(selection, universal_bounds());
		let mut naive_cx = GenerationContext::new(&storage);
		let mut grouped_cx = GenerationContext::new(&storage);
		let mut naive = naive_grove_original_ids_for(&mut naive_cx, region);
		let mut grouped = grouped_cx.original_ids_for::<ChicoGrove>(region);
		naive.sort();
		grouped.sort();
		naive.dedup();
		grouped.dedup();
		assert_eq!(naive, grouped);
		assert!(!grouped.is_empty());
	}

	/// `cargo test -p chico chico_grove_discovery_microbench --release -- --ignored --nocapture`
	#[test]
	#[ignore]
	fn chico_grove_discovery_microbench() {
		use std::time::Instant;

		let region = GroveNeighborhood::around(Vec3::ZERO, GROVE_PRESENT_RADIUS_M);
		let selection = ForestSelection {
			layering: Some(LayeringKind::LushJungle),
			..ForestSelection::default()
		};
		let runs = 30;

		let bench = |naive: bool| -> std::time::Duration {
			let start = Instant::now();
			for _ in 0..runs {
				let storage = HcsgStorage::default();
				storage.seed(selection, universal_bounds());
				let mut cx = GenerationContext::new(&storage);
				if naive {
					naive_grove_original_ids_for(&mut cx, region);
				} else {
					cx.original_ids_for::<ChicoGrove>(region);
				}
			}
			start.elapsed()
		};

		let naive = bench(true);
		let grouped = bench(false);
		let naive_ms = naive.as_secs_f64() * 1000.0 / runs as f64;
		let grouped_ms = grouped.as_secs_f64() * 1000.0 / runs as f64;
		eprintln!(
			"chico_grove_discovery_microbench (GroveNeighborhood, cold storage, seed=LushJungle): \
			 naive={naive_ms:.2} ms/discover grouped={grouped_ms:.2} ms/discover \
			 ({:.2}×)",
			naive_ms / grouped_ms
		);
		assert!(grouped < naive, "grouped discovery should beat per-tile forest lookups");
	}
}
