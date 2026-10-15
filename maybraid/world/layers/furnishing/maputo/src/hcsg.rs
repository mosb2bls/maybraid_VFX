//! Maputo on the shared HCSG runtime ([`lod::hcsg`]): furniture cells
//! generate on the worker from the ground's developments and present as
//! [`HcsgNode`] hosts.
//!
//! Maputo derives everything from the ground, so a session only clears it
//! through the shared HCSG session restart.

use std::collections::BTreeSet;
use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::math::DVec3;
use bevy::prelude::*;
use building_components::{FurnitureNode, FurnitureWireframePlugin};
use furniture_assemblies::FurnitureAssembliesPlugin;
use furniture_shaders::FurnitureShadersPlugin;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext, HcsgClass, PresentationPlugin, ViewerHcsgBounds};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;
use procedural_common::union_aabb3_iter;

use crate::cell::{
	intersects_xz, xz_radius_aabb, FurnitureCellExtent, FURNITURE_CELL_SIZE,
	FURNITURE_PRESENT_RADIUS, FURNITURE_REGION_Y_HALF,
};
use crate::host::FurnitureCell;
use crate::slots::FurnitureSlots;

/// The world-space High slots of one of `U`'s developments, under its id.
pub struct DevelopmentSlots<U> {
	pub slots: Vec<FurnitureNode>,
	/// Furniture cells holding at least one of [`slots`], computed at build time.
	furnished_cell_ids: Vec<Id>,
	_ground: PhantomData<fn() -> U>,
}

fn furnished_cell_ids_for_slots(slots: &[FurnitureNode]) -> Vec<Id> {
	let mut ids = BTreeSet::new();
	for slot in slots {
		let (ix, iz) = FurnitureCellExtent::cell_index_containing(slot.placement.translation);
		ids.insert(FurnitureCellExtent::from_cell_index(ix, iz).id());
	}
	ids.into_iter().collect()
}

impl<U: FurnitureSlots> hcsg::GenerationScheme for DevelopmentSlots<U> {
	lod::hcsg_index_scale!(CELL_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<U::Development>(region)
	}

	/// `None` for a development with nothing to furnish.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let development = cx.get_or_generate::<U::Development>(id)?;
		let slots = U::development_slots(&development);
		let bounds = union_aabb3_iter(slots.iter().map(|slot| slot.scene_bounds()))?;
		let furnished_cell_ids = furnished_cell_ids_for_slots(&slots);
		Some((Self { slots, furnished_cell_ids, _ground: PhantomData }, bounds))
	}
}

/// One 50 m [`FurnitureCell`] holding the slots of `U`'s developments that
/// fall inside it.
pub struct Furnished<U> {
	pub cell: FurnitureCell,
	_ground: PhantomData<fn() -> U>,
}

impl<U: FurnitureSlots> Furnished<U> {
	/// Empty host value for tests and debug spawns.
	pub fn empty_host(extent: FurnitureCellExtent) -> (Self, Aabb3d) {
		let bounds = extent.aabb();
		(Self { cell: FurnitureCell::new(extent, Vec::new()), _ground: PhantomData }, bounds)
	}

	fn developments(cx: &mut GenerationContext, region: Aabb3d) -> Vec<Arc<DevelopmentSlots<U>>> {
		cx.get_or_generate_in::<DevelopmentSlots<U>>(region)
	}

	/// Furniture cells in `region` that hold a slot from one of `U`'s developments.
	fn furnished_cell_ids(cx: &mut GenerationContext, region: Aabb3d) -> BTreeSet<Id> {
		let mut ids = BTreeSet::new();
		for development in Self::developments(cx, region) {
			for cell_id in &development.furnished_cell_ids {
				let Some(extent) = FurnitureCellExtent::from_id(*cell_id) else {
					continue;
				};
				if intersects_xz(region, extent.aabb()) {
					ids.insert(*cell_id);
				}
			}
		}
		ids
	}
}

impl<U: FurnitureSlots> hcsg::GenerationScheme for Furnished<U> {
	lod::hcsg_index_scale!(CELL_SCALE);

	/// The cells overlapping `region` that hold a slot.
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		Self::furnished_cell_ids(cx, region).into_iter().map(OriginalId).collect()
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = FurnitureCellExtent::from_id(id)?;
		let column = xz_radius_aabb(extent.center(), FURNITURE_CELL_SIZE * 0.5);
		let slots: Vec<FurnitureNode> = Self::developments(cx, column)
			.iter()
			.flat_map(|development| development.slots.iter())
			.filter(|slot| extent.contains_xz(slot.placement.translation))
			.cloned()
			.collect();
		if slots.is_empty() {
			return None;
		}
		let cell = FurnitureCell::new(extent, slots);
		let bounds = cell.bounds();
		Some((Self { cell, _ground: PhantomData }, bounds))
	}
}

impl<U: FurnitureSlots> LodScene for Furnished<U> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.cell.scene_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.cell.scene_lod_status(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		self.cell.scene_lod_culls(lod_ref, current)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.cell.scene_with_level(lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		self.cell.scene_chunks_with_level(lod_ref, level)
	}
}

/// The furniture cells around the [`LodViewer`]: within
/// [`FURNITURE_PRESENT_RADIUS`] of its cell.
pub struct FurnitureNeighborhood;

impl FurnitureNeighborhood {
	fn around(viewer: Vec3, radius: f32) -> Aabb3d {
		let (ix, iz) = FurnitureCellExtent::cell_index_containing(viewer);
		xz_radius_aabb(FurnitureCellExtent::from_cell_index(ix, iz).center(), radius)
	}
}

impl ViewerHcsgBounds for FurnitureNeighborhood {
	const CLASS: HcsgClass = HcsgClass::Near;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		vec![Self::around(viewer, FURNITURE_PRESENT_RADIUS)]
	}
}

/// Index scale of furniture values: one furniture cell per bucket, over every
/// elevation a furniture region spans.
const CELL_SCALE: DVec3 = DVec3::new(
	FURNITURE_CELL_SIZE as f64,
	2.0 * FURNITURE_REGION_Y_HALF as f64,
	FURNITURE_CELL_SIZE as f64,
);

/// Presents furniture over ground `U` within channel `C`'s regions from the
/// shared storage.
pub struct MaputoPresentationPlugin<C, U>(PhantomData<fn() -> (C, U)>);

impl<C, U> Default for MaputoPresentationPlugin<C, U> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, U: FurnitureSlots> Plugin for MaputoPresentationPlugin<C, U> {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<FurnitureShadersPlugin>() {
			app.add_plugins(FurnitureShadersPlugin);
		}
		if !app.is_plugin_added::<FurnitureAssembliesPlugin>() {
			app.add_plugins(FurnitureAssembliesPlugin);
		}
		if !app.is_plugin_added::<FurnitureWireframePlugin>() {
			app.add_plugins(FurnitureWireframePlugin);
		}
		app.add_plugins(PresentationPlugin::<C, Furnished<U>>::default());
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::entity_disabling::Disabled;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use durham::Durham;
	use lod::hcsg::{
		hcsg_hosts, settle_presentation, wait_replaced_hosts, HcsgNode, HcsgRestartRequest,
		RetiredHost, SettlePresentation,
	};
	use lod::lod_ref::LodNodePose;
	use lod::LodViewer;
	use richmond::{
		AuthoredDevelopment, AuthoredDevelopments, Built, DevelopmentConfig, DevelopmentKind,
		DevelopmentSites, Richmond,
	};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::UrbanizationSelection;
	use urbanization_layer_model::Urbanization;

	use super::*;
	use crate::colliders::FurnitureWalkCollider;

	type Ground = OnTerrain<Durham>;
	type Urban = Urbanization<Richmond<Ground>>;

	const IDLE: Duration = Duration::from_secs(120);

	/// One Les Halles authored at `height`. Authored sites plan on their own
	/// level, so no ground is generated.
	fn authored(height: f32) -> AuthoredDevelopment {
		AuthoredDevelopment {
			cell: Aabb3d::from_min_max(Vec3::new(10.0, 0.0, 10.0), Vec3::new(150.0, 1.0, 150.0)),
			kinds: vec![DevelopmentKind::LesHalles],
			height,
			config: DevelopmentConfig::default(),
			courtyard: None,
		}
	}

	fn seed_resources(app: &mut App, height: f32) {
		app.insert_resource(DevelopmentConfig {
			sites: DevelopmentSites::Authored,
			..DevelopmentConfig::default()
		})
		.insert_resource(AuthoredDevelopments(vec![authored(height)]))
		.insert_resource(HcsgRestartRequest::queued());
	}

	/// Furniture presented around a viewer at `at`.
	fn app(at: Vec3) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.init_resource::<UrbanizationSelection>()
			.add_plugins((
				hcsg::HcsgBoundsPlugin::<FurnitureNeighborhood>::default(),
				richmond::BuiltPresentationPlugin::<FurnitureNeighborhood, Ground>::default(),
				MaputoPresentationPlugin::<FurnitureNeighborhood, Urban>::default(),
			));
		seed_resources(&mut app, 12.0);
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

	fn center() -> Vec3 {
		let cell = authored(0.0).cell;
		(Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5
	}

	/// Cells holding the development's slots within the viewer's
	/// neighborhood, with how many slots each holds.
	fn expected_cells(app: &mut App) -> anyhow::Result<Vec<(Id, usize)>> {
		let mut viewers = app.world_mut().query_filtered::<&Transform, With<LodViewer>>();
		let viewer = *viewers.single(app.world())?;
		let (ix, iz) = FurnitureCellExtent::cell_index_containing(viewer.translation);
		let inner = xz_radius_aabb(
			FurnitureCellExtent::from_cell_index(ix, iz).center(),
			FURNITURE_PRESENT_RADIUS,
		);
		let storage = app.world().resource::<hcsg::HcsgStorage>();
		let built = storage
			.try_entry::<Built<Ground>>(authored(0.0).id())
			.map_err(|busy| anyhow::anyhow!("HcsgStorage busy: {busy:?}"))?
			.map(|entry| entry.value)
			.ok_or_else(|| anyhow::anyhow!("the development was not built"))?;
		let mut cells = std::collections::BTreeMap::<Id, usize>::new();
		for slot in Urban::development_slots(&built) {
			let (ix, iz) = FurnitureCellExtent::cell_index_containing(slot.placement.translation);
			let extent = FurnitureCellExtent::from_cell_index(ix, iz);
			if intersects_xz(inner, extent.aabb()) {
				*cells.entry(extent.id()).or_default() += 1;
			}
		}
		Ok(cells.into_iter().collect())
	}

	fn furnished(app: &mut App) -> Vec<(Id, usize)> {
		let mut cells: Vec<_> = app
			.world_mut()
			.query_filtered::<&HcsgNode<Furnished<Urban>>, Without<RetiredHost>>()
			.iter(app.world())
			.map(|node| (node.id, node.value.cell.slots.len()))
			.collect();
		cells.sort();
		cells
	}

	#[test]
	fn the_neighborhood_presents_every_furnished_cell_with_walk_colliders() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;

		let expected = expected_cells(&mut app)?;
		assert!(!expected.is_empty(), "Les Halles furnishes the neighborhood");
		assert_eq!(furnished(&mut app), expected, "one host per furnished cell, with its slots");
		let colliders = app
			.world_mut()
			.query_filtered::<(), (With<FurnitureWalkCollider>, Allow<Disabled>)>()
			.iter(app.world())
			.count();
		assert!(colliders > 0, "furniture bears weight");
		Ok(())
	}

	#[test]
	fn leaving_the_neighborhood_retires_its_hosts() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;
		assert!(!hcsg_hosts::<Furnished<Urban>>(&mut app).is_empty());

		let away = Transform::from_xyz(5_000.0, 0.0, 5_000.0);
		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = away;
		settle(&mut app)?;
		assert!(hcsg_hosts::<Furnished<Urban>>(&mut app).is_empty());
		Ok(())
	}

	mod scheme_perf_tests {
		use super::*;
		use crate::cell::FURNITURE_GENERATE_RADIUS;
		use crate::host::FurnitureCell;
		use lod::hcsg::{universal_bounds, GenerationContext, GenerationScheme, HcsgStorage};
		use std::time::Instant;

		type Urban = Urbanization<Richmond<Ground>>;

		fn seed_les_halles(storage: &HcsgStorage, height: f32) {
			storage.seed(
				DevelopmentConfig {
					sites: DevelopmentSites::Authored,
					..DevelopmentConfig::default()
				},
				universal_bounds(),
			);
			storage.seed(
				AuthoredDevelopments(vec![authored(height)]),
				universal_bounds(),
			);
		}

		fn furnished_cell_ids_via_slots(
			cx: &mut GenerationContext,
			region: Aabb3d,
		) -> BTreeSet<Id> {
			let mut ids = BTreeSet::new();
			for development in cx.get_or_generate_in::<DevelopmentSlots<Urban>>(region) {
				for slot in &development.slots {
					let (ix, iz) =
						FurnitureCellExtent::cell_index_containing(slot.placement.translation);
					let extent = FurnitureCellExtent::from_cell_index(ix, iz);
					if intersects_xz(region, extent.aabb()) {
						ids.insert(extent.id());
					}
				}
			}
			ids
		}

		fn furnished_original_ids(cx: &mut GenerationContext, region: Aabb3d) -> BTreeSet<Id> {
			cx.original_ids_for::<Furnished<Urban>>(region)
				.into_iter()
				.map(|OriginalId(id)| id)
				.collect()
		}

		fn furnished_snapshot(cell: &FurnitureCell) -> (usize, Vec<(f32, f32, f32)>) {
			let positions = cell
				.slots
				.iter()
				.map(|slot| {
					let t = slot.placement.translation;
					(t.x, t.y, t.z)
				})
				.collect();
			(cell.slots.len(), positions)
		}

		#[test]
		fn furnished_discovery_matches_slot_scan() -> anyhow::Result<()> {
			let storage = HcsgStorage::default();
			seed_les_halles(&storage, 12.0);
			let region = FurnitureNeighborhood::around(center(), FURNITURE_PRESENT_RADIUS);
			let mut legacy_cx = GenerationContext::new(&storage);
			let mut optimized_cx = GenerationContext::new(&storage);
			let legacy = furnished_cell_ids_via_slots(&mut legacy_cx, region);
			let optimized = furnished_original_ids(&mut optimized_cx, region);
			assert_eq!(legacy, optimized);
			Ok(())
		}

		#[test]
		fn furnished_cells_match_slot_scan_across_heights() -> anyhow::Result<()> {
			for height in [0.0f32, 12.0, 20.0, 48.0] {
				let storage = HcsgStorage::default();
				seed_les_halles(&storage, height);
				let region = FurnitureNeighborhood::around(center(), FURNITURE_GENERATE_RADIUS);
				let mut legacy_cx = GenerationContext::new(&storage);
				let mut optimized_cx = GenerationContext::new(&storage);
				let legacy_ids = furnished_cell_ids_via_slots(&mut legacy_cx, region);
				let optimized_ids = furnished_original_ids(&mut optimized_cx, region);
				assert_eq!(legacy_ids, optimized_ids, "height {height}");

				for cell_id in optimized_ids {
					let legacy = Furnished::<Urban>::build_with_id(&mut legacy_cx, cell_id)
						.ok_or_else(|| anyhow::anyhow!("legacy cell {cell_id:?}"))?
						.0
						.cell;
					let optimized = Furnished::<Urban>::build_with_id(&mut optimized_cx, cell_id)
						.ok_or_else(|| anyhow::anyhow!("optimized cell {cell_id:?}"))?
						.0
						.cell;
					assert_eq!(
						furnished_snapshot(&legacy),
						furnished_snapshot(&optimized),
						"height {height} cell {cell_id:?}"
					);
				}
			}
			Ok(())
		}

		fn warm_discover_time(slot_scan: bool, runs: usize) -> std::time::Duration {
			let storage = HcsgStorage::default();
			seed_les_halles(&storage, 12.0);
			let region = FurnitureNeighborhood::around(center(), FURNITURE_GENERATE_RADIUS);
			let mut warm = GenerationContext::new(&storage);
			let _ = warm.get_or_generate_in::<DevelopmentSlots<Urban>>(region);
			let start = Instant::now();
			for _ in 0..runs {
				let mut cx = GenerationContext::new(&storage);
				if slot_scan {
					let _ = furnished_cell_ids_via_slots(&mut cx, region);
				} else {
					let _ = furnished_original_ids(&mut cx, region);
				}
			}
			start.elapsed()
		}

		/// `cargo test -p maputo furnished_discovery_scheme_microbench --release -- --ignored --nocapture`
		#[test]
		#[ignore = "microbench"]
		fn furnished_discovery_scheme_microbench() {
			let runs = 200;
			let slot_scan = warm_discover_time(true, runs);
			let indexed = warm_discover_time(false, runs);
			eprintln!(
				"furnished_discovery_scheme_microbench (Les Halles, {runs} warm discovers, \
				 generate radius {FURNITURE_GENERATE_RADIUS} m): slot-scan {:?} indexed {:?} ({:.2}x)",
				slot_scan,
				indexed,
				slot_scan.as_secs_f64() / indexed.as_secs_f64().max(1e-9)
			);
		}
	}

	#[test]
	fn a_restart_replaces_every_furnished_host() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;
		let before = hcsg_hosts::<Furnished<Urban>>(&mut app);

		seed_resources(&mut app, 20.0);
		settle(&mut app)?;
		// Presentation keeps the previous session until new versions publish.
		// `settle` only runs a fixed reconcile budget; wait on versions.
		let after = wait_replaced_hosts::<Furnished<Urban>>(&mut app, &before, IDLE, 64)?;

		assert_eq!(after.len(), before.len());
		for ((id, old), (new_id, new)) in before.iter().zip(&after) {
			assert_eq!(id, new_id);
			assert!(new > old, "{id:?} still shows the previous session");
		}
		Ok(())
	}
}
