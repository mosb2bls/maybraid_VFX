//! Geneva on the shared HCSG runtime ([`lod::hcsg`]): large language
//! tiles and their region names generate on the worker within channel `C`'s
//! window, and the names of what stands near the viewer generate within a
//! naming window Geneva derives from it. The frame only presents storage
//! into the [`LanguageOverlay`].
//!
//! A session starts with [`lod::hcsg::request_hcsg_session_restart`].

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use durham::terrain::watersheds::{PocketWatersHighPass, PocketWatersLowPass};
use durham::terrain::{
	CanyonHighPassStampCell, CanyonLowPassStampCell, MassifHighPassStampCell,
	MassifLowPassStampCell, PlateauHighPassStampCell, PlateauLowPassStampCell,
	PocketWaterHighPassStampCell, PocketWaterLowPassStampCell, RollingHighPassStampCell,
	RollingLowPassStampCell, ValleyHighPassStampCell, ValleyLowPassStampCell,
};
use lod::gen::{Id, OriginalId};
use lod::hcsg::universal_bounds;
use lod::hcsg::{
	self, register_session_seed, Busy, GenerationContext, GenerationPlugin, GenerationScheme,
	HcsgClass, HcsgRegions, HcsgStorage, HcsgSystems, ViewerHcsgBounds,
};
use lod::LodViewer;
use richmond::DEVELOPMENT_CELL_SIZE;

use crate::named::{Forests, Groves, Named, Places, Regions, Stamp, Urban, Waters};
use crate::places::LanguageGround;
use crate::present::{present_language_overlay, LanguageOverlay, Nearby};
use crate::tiles::{
	large_tile_aabb, large_tile_index, large_tile_origin, LargeTile,
	LARGE_TILE,
};

/// Language tiles stay generated within this XZ radius of the viewer.
const LANGUAGE_GENERATE_RADIUS: f32 = 40_000.0;
/// Nearby entity naming. Language tiles keep the broad generate window.
pub(crate) const LANGUAGE_NAME_RADIUS: f32 = 400.0;
/// How far the viewer moves before the naming window recenters on it.
pub(crate) const LANGUAGE_NAME_HYSTERESIS: f32 = 80.0;
/// The naming window recenters on this grid.
pub const NAME_WINDOW_QUANT_M: f32 = 32.0;
/// Half-height of the naming window: every height a named source stands at.
const NAMING_COLUMN_Y: f32 = 10_000.0;

/// Configured world seed for language tiles and names: Geneva's session root.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageWorldSeed(pub u64);

lod::seeded_root!(LanguageWorldSeed);

impl Default for LanguageWorldSeed {
	fn default() -> Self {
		Self(LanguageConfig::DEFAULT_SEED)
	}
}

/// World/config contract for Geneva generation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LanguageConfig {
	pub seed: u64,
}

impl LanguageConfig {
	pub const DEFAULT_SEED: u64 = 0x6A7B_8A1D_E11E;

	pub fn world_defaults() -> Self {
		Self { seed: Self::DEFAULT_SEED }
	}
}

impl Default for LanguageConfig {
	fn default() -> Self {
		Self::world_defaults()
	}
}

impl hcsg::GenerationScheme for LargeTile {
	lod::hcsg_index_scale!(TILE_INDEX_SCALE);

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		crate::tiles::large_tile_original_ids_overlapping(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (ix, iz) = LargeTile::index_of(id)?;
		let seed = cx.get::<LanguageWorldSeed>(Id::Universal)?;
		Some((LargeTile::generate(seed.0, ix, iz), large_tile_aabb(ix, iz)))
	}
}

/// The large tiles within [`LANGUAGE_GENERATE_RADIUS`] of the [`LodViewer`],
/// as one box on the tile grid: it changes only when the window's edge
/// crosses a tile line.
pub struct LanguageNeighborhood;

impl LanguageNeighborhood {
	fn around(viewer: Vec3) -> Aabb3d {
		let r = LANGUAGE_GENERATE_RADIUS;
		let (min_x, min_z) =
			large_tile_origin(large_tile_index(viewer.x - r), large_tile_index(viewer.z - r));
		let (max_x, max_z) = large_tile_origin(
			large_tile_index(viewer.x + r) + 1,
			large_tile_index(viewer.z + r) + 1,
		);
		Aabb3d::from_min_max(Vec3::new(min_x, -1.0, min_z), Vec3::new(max_x, 1.0, max_z))
	}
}

impl ViewerHcsgBounds for LanguageNeighborhood {
	const CLASS: HcsgClass = HcsgClass::Ambient;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		vec![Self::around(viewer)]
	}
}

pub(crate) const TILE_INDEX_SCALE: DVec3 = DVec3::new(LARGE_TILE as f64, 2.0, LARGE_TILE as f64);
/// One development cell per bucket, one naming column tall.
pub(crate) const PLACES_INDEX_SCALE: DVec3 = DVec3::new(
	DEVELOPMENT_CELL_SIZE as f64,
	2.0 * NAMING_COLUMN_Y as f64,
	DEVELOPMENT_CELL_SIZE as f64,
);

/// From one list of the sources named near the viewer, over ground `$W`:
/// the naming window's generation and the overlay's reads.
macro_rules! geneva_nodes {
	(<$W:ident> $($S:ty),* $(,)?) => {
		/// Generates each source's names over ground `W` within the naming
		/// window over `C`.
		fn register_naming<C: Send + Sync + 'static, $W: LanguageGround>(app: &mut App) {
			$(app.add_plugins(GenerationPlugin::<Naming<C>, Named<$S>>::default());)*
		}

		/// Latest membership change among the sources named over ground `W`.
		pub(crate) fn naming_revision<$W: LanguageGround>(
			storage: &HcsgStorage,
		) -> Result<u64, Busy> {
			let mut latest = 0;
			$(latest = latest.max(storage.try_membership_revision::<Named<$S>>()?);)*
			Ok(latest)
		}

		/// Reads the names of each source over ground `W` the window reaches.
		pub(crate) fn read_nearby<$W: LanguageGround>(nearby: &mut Nearby) -> Result<(), Busy> {
			$(nearby.read::<$S>()?;)*
			Ok(())
		}
	};
}

geneva_nodes!(<W>
	Forests,
	Groves<W>,
	Urban,
	Places<W>,
	Stamp<MassifHighPassStampCell>,
	Stamp<MassifLowPassStampCell>,
	Stamp<PlateauHighPassStampCell>,
	Stamp<PlateauLowPassStampCell>,
	Stamp<CanyonHighPassStampCell>,
	Stamp<CanyonLowPassStampCell>,
	Stamp<RollingHighPassStampCell>,
	Stamp<RollingLowPassStampCell>,
	Stamp<ValleyHighPassStampCell>,
	Stamp<ValleyLowPassStampCell>,
	Stamp<PocketWaterHighPassStampCell>,
	Stamp<PocketWaterLowPassStampCell>,
	Waters<PocketWatersHighPass>,
	Waters<PocketWatersLowPass>,
);

/// Geneva's root resource: the language world seed.
#[derive(SystemParam)]
pub struct GenevaRoots<'w> {
	seed: Res<'w, LanguageWorldSeed>,
}

impl GenevaRoots<'_> {
	/// Seeds Geneva's world seed root over ground `W`.
	pub fn seed<W: LanguageGround>(&self, storage: &HcsgStorage) {
		storage.seed(*self.seed, universal_bounds());
	}
}

/// Seeds [`GenevaRoots`] during an HCSG session restart.
pub fn seed_geneva_hcsg_roots<W: LanguageGround>(roots: GenevaRoots, storage: Res<HcsgStorage>) {
	roots.seed::<W>(storage.as_ref());
}

/// Geneva's naming window over channel `C`: [`LANGUAGE_NAME_RADIUS`] around
/// the viewer while `C` has regions.
struct Naming<C>(PhantomData<fn() -> C>);

/// Channel `C`'s latest regions, and the naming window's center.
#[derive(Resource, Default)]
pub(crate) struct LanguageWindow {
	pub(crate) boxes: Vec<Aabb3d>,
	focus: Option<Vec3>,
	naming: Option<Vec2>,
}

impl LanguageWindow {
	/// What the naming window covers: the name radius, plus the hysteresis
	/// the viewer may move before it recenters.
	pub(crate) fn naming_box(&self) -> Option<Aabb3d> {
		let center = self.naming?;
		let r = LANGUAGE_NAME_RADIUS + LANGUAGE_NAME_HYSTERESIS;
		Some(Aabb3d::from_min_max(
			Vec3::new(center.x - r, -NAMING_COLUMN_Y, center.y - r),
			Vec3::new(center.x + r, NAMING_COLUMN_Y, center.y + r),
		))
	}
}

/// Follows `C`, and recenters the naming window on the viewer once it has
/// moved [`LANGUAGE_NAME_HYSTERESIS`] from the center.
fn track_language<C: Send + Sync + 'static>(
	mut regions: MessageReader<HcsgRegions<C>>,
	viewers: Query<&Transform, With<LodViewer>>,
	mut window: ResMut<LanguageWindow>,
	mut naming: MessageWriter<HcsgRegions<Naming<C>>>,
) {
	if let Some(latest) = regions.read().last() {
		window.boxes = latest.boxes.clone();
		window.focus = latest.focus;
	}
	let viewer = viewers.iter().next().map(|viewer| viewer.translation).or(window.focus);
	let center = match viewer {
		Some(at) if !window.boxes.is_empty() => Some(match window.naming {
			Some(center) if (at.xz() - center).abs().max_element() <= LANGUAGE_NAME_HYSTERESIS => {
				center
			}
			_ => (at.xz() / NAME_WINDOW_QUANT_M).round() * NAME_WINDOW_QUANT_M,
		}),
		_ => None,
	};
	if center != window.naming {
		window.naming = center;
		naming.write(HcsgRegions::new(
			window.naming_box().into_iter().collect(),
			viewer,
			HcsgClass::Ambient,
		));
	}
}

/// The stored value for `id` and its published bounds after generate.
pub(crate) fn generated<T: GenerationScheme>(
	cx: &mut GenerationContext,
	id: Id,
) -> Option<(Arc<T>, Aabb3d)> {
	let value = cx.get_or_generate::<T>(id)?;
	let bounds = cx.stored_bounds::<T>(id)?;
	Some((value, bounds))
}

/// Geneva over ground `W`: language tiles and region names within channel
/// `C`'s regions, names for the forests, groves, urbanization, Durham
/// geography and Richmond places near the [`LodViewer`], and the
/// [`LanguageOverlay`] they present to.
///
/// `W` is the ground groves grow on, as for Chico's presentation: the
/// Discovery world passes `Urbanization<Richmond<OnTerrain<Durham>>>`. An
/// empty region set on `C` idles naming and empties the overlay. The world
/// seed comes from [`LanguageWorldSeed`], [`LanguageConfig::world_defaults`]
/// unless inserted first.
pub struct GenevaPlugin<C, W>(PhantomData<fn() -> (C, W)>);

impl<C, W> Default for GenevaPlugin<C, W> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, W: LanguageGround> Plugin for GenevaPlugin<C, W> {
	fn build(&self, app: &mut App) {
		register_session_seed(app, seed_geneva_hcsg_roots::<W>);
		app.init_resource::<LanguageWorldSeed>()
			.init_resource::<LanguageOverlay>()
			.init_resource::<LanguageWindow>()
			.add_message::<HcsgRegions<Naming<C>>>()
			.add_plugins(GenerationPlugin::<C, Named<Regions>>::default())
			.add_systems(Update, track_language::<C>.before(HcsgSystems))
			.add_systems(Update, present_language_overlay::<W>.after(HcsgSystems));
		register_naming::<C, W>(app);
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::system::SystemParamItem;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use chico::{ChicoPresentationPlugin, ForestSelection, GroveNeighborhood};
	use durham::{
		fine_patch_cell_layout, Durham, DurhamWindow, TerrainConfig, TerrainMeshAssets,
		TerrainMeshLodBand, TerrainStampConfigs, WaterMeshAssets, WatershedConfigs,
	};
	use lod::hcsg::{
		Gated, HcsgBoundsPlugin, HcsgGate, HcsgRestartRequest, SettlePresentation,
		settle_presentation,
	};
	use lod::lod_ref::LodNodePose;
	use richmond::{
		AuthoredDevelopment, AuthoredDevelopments, DevelopmentConfig, DevelopmentKind,
		DevelopmentSites, Richmond,
	};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::UrbanizationSelection;
	use urbanization_layer_model::Urbanization;

	use super::*;
	use crate::places::DevelopmentPlaces;
	use crate::NameKey;

	type Ground = OnTerrain<Durham>;
	type Urban = Urbanization<Richmond<Ground>>;

	const IDLE: Duration = Duration::from_secs(300);

	/// Whether the test's language channel is open.
	#[derive(Resource)]
	struct Open(bool);

	struct Opened;

	impl HcsgGate for Opened {
		type Param = Res<'static, Open>;

		fn open(open: &SystemParamItem<Self::Param>) -> bool {
			open.0
		}
	}

	type Window = Gated<Opened, LanguageNeighborhood>;

	fn spawn_viewer(app: &mut App, at: Vec3) {
		let at = Transform::from_translation(at);
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
	}

	/// Geneva alone over an empty world, viewed from `at`.
	fn language_app(at: Vec3) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.insert_resource(Open(true))
			.insert_resource(HcsgRestartRequest::queued())
			.add_plugins(HcsgBoundsPlugin::<Window>::default())
			.add_plugins(GenevaPlugin::<Window, Urban>::default());
		app.finish();
		app.cleanup();
		spawn_viewer(&mut app, at);
		app
	}

	fn settle(app: &mut App) -> anyhow::Result<()> {
		settle_presentation(app, SettlePresentation::geneva_integration(IDLE)).map_err(Into::into)
	}

	fn region_names(app: &App) -> Vec<(i32, i32, String)> {
		let mut names: Vec<_> = app
			.world()
			.resource::<LanguageOverlay>()
			.names
			.iter()
			.filter_map(|name| match name.key {
				NameKey::Region { ix, iz } => Some((ix, iz, name.surface.clone())),
				_ => None,
			})
			.collect();
		names.sort();
		names
	}

	fn storage_busy(busy: lod::hcsg::Busy) -> anyhow::Error {
		anyhow::anyhow!("HcsgStorage busy: {busy:?}")
	}

	/// The tiles within the generate radius of the origin.
	fn origin_window() -> Vec<(i32, i32)> {
		(-2..=1).flat_map(|ix| (-2..=1).map(move |iz| (ix, iz))).collect()
	}

	#[test]
	fn the_window_generates_tiles_and_names_their_regions() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;

		let storage = app.world().resource::<HcsgStorage>().clone();
		for (ix, iz) in origin_window() {
			let tile = storage
				.try_entry::<LargeTile>(LargeTile::id(ix, iz))
				.map_err(storage_busy)?
				.map(|entry| entry.value)
				.ok_or_else(|| anyhow::anyhow!("tile ({ix}, {iz}) was not generated"))?;
			assert_eq!(*tile, LargeTile::generate(LanguageWorldSeed::default().0, ix, iz));
			assert!(storage
				.try_entry::<Named<Regions>>(LargeTile::id(ix, iz))
				.map_err(storage_busy)?
				.is_some());
		}
		assert!(
			storage
				.try_entry::<LargeTile>(LargeTile::id(3, 0))
				.map_err(storage_busy)?
				.is_none(),
			"beyond the window"
		);

		let overlay = app.world().resource::<LanguageOverlay>();
		assert_eq!(overlay.large_tiles.len(), origin_window().len());
		let named: Vec<_> = region_names(&app).into_iter().map(|(ix, iz, _)| (ix, iz)).collect();
		assert_eq!(named, origin_window(), "every tile in the window names its region");
		assert!(region_names(&app).iter().all(|(_, _, surface)| !surface.is_empty()));
		Ok(())
	}

	#[test]
	fn a_closed_channel_idles_naming_and_empties_the_overlay() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;
		assert!(!region_names(&app).is_empty());
		assert!(app.world().resource::<LanguageWindow>().naming_box().is_some());

		app.world_mut().resource_mut::<Open>().0 = false;
		settle(&mut app)?;
		let epoch = app.world().resource::<LanguageOverlay>().epoch;
		assert_eq!(
			*app.world().resource::<LanguageOverlay>(),
			LanguageOverlay { epoch, ..LanguageOverlay::default() }
		);
		assert!(app.world().resource::<LanguageWindow>().naming_box().is_none());
		app.update();
		assert_eq!(app.world().resource::<LanguageOverlay>().epoch, epoch, "idle stays idle");

		app.world_mut().resource_mut::<Open>().0 = true;
		settle(&mut app)?;
		assert_eq!(region_names(&app).len(), origin_window().len());
		Ok(())
	}

	#[test]
	fn a_restart_with_a_new_seed_renames_every_region() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;
		let before = region_names(&app);

		app.insert_resource(LanguageWorldSeed(99));
		app.world_mut().resource_mut::<HcsgRestartRequest>().request();
		settle(&mut app)?;
		let after = region_names(&app);

		assert_eq!(after.len(), before.len());
		assert_ne!(after, before, "the new seed composes new regions");
		let storage = app.world().resource::<HcsgStorage>().clone();
		let tile = storage
			.try_entry::<LargeTile>(LargeTile::id(0, 0))
			.map_err(storage_busy)?
			.map(|entry| entry.value)
			.ok_or_else(|| anyhow::anyhow!("tile"))?;
		assert_eq!(*tile, LargeTile::generate(99, 0, 0));
		Ok(())
	}

	#[test]
	fn crossing_a_tile_line_moves_the_window() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::new(9_000.0, 0.0, 0.0));
		settle(&mut app)?;
		let named: Vec<_> = region_names(&app).into_iter().map(|(ix, iz, _)| (ix, iz)).collect();
		assert_eq!(named, origin_window());

		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = Transform::from_xyz(16_000.0, 0.0, 0.0);
		settle(&mut app)?;
		let named: Vec<_> = region_names(&app).into_iter().map(|(ix, iz, _)| (ix, iz)).collect();
		let moved: Vec<_> = (-1..=2).flat_map(|ix| (-2..=1).map(move |iz| (ix, iz))).collect();
		assert_eq!(named, moved, "regions follow the window");
		assert_eq!(app.world().resource::<LanguageOverlay>().large_tiles.len(), moved.len());
		let storage = app.world().resource::<HcsgStorage>().clone();
		assert!(
			storage
				.try_entry::<LargeTile>(LargeTile::id(2, 0))
				.map_err(storage_busy)?
				.is_some(),
			"the tile the window crossed into generated"
		);
		Ok(())
	}

	#[test]
	fn the_naming_window_recenters_past_the_hysteresis() -> anyhow::Result<()> {
		let mut app = language_app(Vec3::ZERO);
		settle(&mut app)?;
		let first = app.world().resource::<LanguageWindow>().naming;
		assert_eq!(first, Some(Vec2::ZERO));

		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? =
			Transform::from_xyz(LANGUAGE_NAME_HYSTERESIS - 1.0, 0.0, 0.0);
		app.update();
		assert_eq!(app.world().resource::<LanguageWindow>().naming, first, "within hysteresis");

		*viewers.single_mut(app.world_mut())? = Transform::from_xyz(200.0, 0.0, 0.0);
		app.update();
		let moved = (200.0 / NAME_WINDOW_QUANT_M).round() * NAME_WINDOW_QUANT_M;
		assert_eq!(app.world().resource::<LanguageWindow>().naming, Some(Vec2::new(moved, 0.0)));
		Ok(())
	}

	/// One Les Halles authored inside one cell of the patch.
	fn les_halles() -> AuthoredDevelopment {
		AuthoredDevelopment {
			cell: Aabb3d::from_min_max(Vec3::new(10.0, 0.0, 10.0), Vec3::new(150.0, 1.0, 150.0)),
			kinds: vec![DevelopmentKind::LesHalles],
			height: 12.0,
			config: DevelopmentConfig::default(),
			courtyard: None,
		}
	}

	/// Groves on Chico's 2×2 fine patch at the origin and a Les Halles in
	/// one of its cells, named by Geneva.
	fn forested_app() -> App {
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
			.insert_resource(AuthoredDevelopments(vec![les_halles()]))
			.init_resource::<UrbanizationSelection>()
			.insert_resource(ForestSelection {
				layering: Some(chico::LayeringKind::LushJungle),
				..ForestSelection::default()
			})
			.insert_resource(Open(true))
			.insert_resource(HcsgRestartRequest::queued())
			.add_plugins((
				HcsgBoundsPlugin::<GroveNeighborhood>::default(),
				HcsgBoundsPlugin::<Window>::default(),
				durham::WaterPresentationPlugin::<DurhamWindow>::default(),
				richmond::BuiltPresentationPlugin::<GroveNeighborhood, Ground>::default(),
			))
			.add_plugins(ChicoPresentationPlugin::<GroveNeighborhood, Urban>::default())
			.add_plugins(GenevaPlugin::<Window, Urban>::default());
		durham::register_durham_hcsg_session(&mut app);
		app.finish();
		app.cleanup();
		spawn_viewer(&mut app, Vec3::ZERO);
		app
	}

	fn naming_window(app: &App) -> anyhow::Result<Aabb3d> {
		let window = app.world().resource::<LanguageWindow>();
		window.naming_box().ok_or_else(|| anyhow::anyhow!("naming is idle"))
	}

	#[test]
	fn nearby_groves_are_named_in_their_tiles_languages() -> anyhow::Result<()> {
		let mut app = forested_app();
		settle(&mut app)?;

		let overlay = app.world().resource::<LanguageOverlay>().clone();
		let groves: Vec<_> = overlay
			.names
			.iter()
			.filter(|name| matches!(name.key, NameKey::Forest(_) | NameKey::Grove(_)))
			.collect();
		assert!(!groves.is_empty(), "the patch's forests and groves are named");
		let window = naming_window(&app)?;
		let reach = Rect::from_corners(
			Vec2::new(window.min.x, window.min.z),
			Vec2::new(window.max.x, window.max.z),
		);
		for name in &groves {
			assert!(!name.surface.is_empty());
			assert!(!reach.intersect(name.extent).is_empty(), "{:?} named outside", name.key);
		}

		let storage = app.world().resource::<HcsgStorage>().clone();
		let grown = storage
			.try_overlapping::<chico::GrownGrove<Urban>>(window)
			.map_err(storage_busy)?;
		let named_groves =
			groves.iter().filter(|name| matches!(name.key, NameKey::Grove(_))).count();
		assert!(named_groves > 0 && named_groves <= grown.len(), "only grown groves are named");
		Ok(())
	}

	#[test]
	fn built_places_are_named_and_rooms_speak_their_buildings_language() -> anyhow::Result<()> {
		let mut app = forested_app();
		settle(&mut app)?;

		let development = les_halles().id();
		let storage = app.world().resource::<HcsgStorage>().clone();
		let places = storage
			.try_entry::<DevelopmentPlaces<Urban>>(development)
			.map_err(storage_busy)?
			.map(|entry| entry.value)
			.ok_or_else(|| anyhow::anyhow!("the development's places were not generated"))?;
		let building = places
			.places
			.iter()
			.position(|place| place.building.is_none() && place.place.host == Some(development))
			.ok_or_else(|| anyhow::anyhow!("no building place"))?;
		let rooms: Vec<_> =
			places.places.iter().filter(|place| place.building == Some(building)).collect();
		assert!(!rooms.is_empty(), "Les Halles authors usage-area rooms");

		let named = storage
			.try_entry::<Named<Places<Urban>>>(development)
			.map_err(storage_busy)?
			.map(|entry| entry.value)
			.ok_or_else(|| anyhow::anyhow!("the places were not named"))?;
		let name = |key: NameKey| named.names.iter().find(|entry| entry.key == key);
		let host = name(places.places[building].key)
			.ok_or_else(|| anyhow::anyhow!("the building has no name"))?;
		for room in &rooms {
			let room = name(room.key).ok_or_else(|| anyhow::anyhow!("{:?} unnamed", room.key))?;
			assert_eq!(room.name.language_seed, host.name.language_seed);
		}

		let overlay = app.world().resource::<LanguageOverlay>();
		assert!(
			overlay.names.iter().any(|name| name.key == places.places[building].key),
			"the building's name reaches the overlay"
		);
		Ok(())
	}
}
