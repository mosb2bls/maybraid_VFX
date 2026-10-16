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
	_ground: PhantomData<fn() -> U>,
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
		Some((Self { slots, _ground: PhantomData }, bounds))
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
		hcsg_hosts, settle_presentation, wait_replaced_hosts, HcsgDemand, HcsgNode,
		HcsgRestartRequest, RetiredHost, SettlePresentation, SettleTimedOut,
	};
	use lod::lod_ref::LodNodePose;
	use lod::{LodChunkFulfillment, LodLevelSpawnRequest, LodViewer};
	use richmond::{
		AuthoredDevelopment, AuthoredDevelopments, Built, DevelopmentConfig, DevelopmentKind,
		DevelopmentSites, Richmond,
	};
	use terrain_layer_model::OnTerrain;
	use urbanization_cells::UrbanizationSelection;
	use urbanization_layer_model::Urbanization;

	use super::*;
	use crate::colliders::{walk_collider, FurnitureWalkCollider};

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
		let mut cells: Vec<_> = furnished_hosts(app)
			.into_iter()
			.map(|(_, node)| (node.id, node.value.cell.slots.len()))
			.collect();
		cells.sort();
		cells
	}

	fn furnished_hosts(app: &mut App) -> Vec<(Entity, HcsgNode<Furnished<Urban>>)> {
		let mut cells: Vec<_> = app
			.world_mut()
			.query_filtered::<(Entity, &HcsgNode<Furnished<Urban>>), Without<RetiredHost>>()
			.iter(app.world())
			.map(|(entity, node)| (entity, node.clone()))
			.collect();
		cells.sort_by_key(|(_, node)| node.id);
		cells
	}

	fn subtree_has_walk_collider(world: &World, entity: Entity) -> bool {
		if world.get::<FurnitureWalkCollider>(entity).is_some() {
			return true;
		}
		let Some(children) = world.get::<Children>(entity) else {
			return false;
		};
		children.iter().any(|child| subtree_has_walk_collider(world, child))
	}

	fn cell_needs_walk_collider(node: &HcsgNode<Furnished<Urban>>) -> bool {
		walk_collider(&node.value.cell.slots).is_some()
	}

	/// Hosts exist before chunk drain; 8 reconcile frames does not guarantee
	/// the first furniture primitive has spawned under Richmond's shared 2 ms
	/// budget. Wait until every expected cell is a current host and every
	/// collider-bearing cell has a [`FurnitureWalkCollider`] descendant.
	fn furnished_walk_colliders_ready(app: &mut App, expected: &[(Id, usize)]) -> bool {
		if furnished(app) != expected {
			return false;
		}
		let hosts = furnished_hosts(app);
		let world = app.world();
		hosts.iter().all(|(entity, node)| {
			!cell_needs_walk_collider(node) || subtree_has_walk_collider(world, *entity)
		})
	}

	fn host_fulfill_state(app: &mut App, host: Entity) -> String {
		let spawn = app
			.world()
			.get::<LodLevelSpawnRequest>(host)
			.map(|request| format!("{:?}", request.level));
		let fulfillment = {
			let mut jobs =
				app.world_mut().query_filtered::<&LodChunkFulfillment, Allow<Disabled>>();
			jobs.iter(app.world()).find(|job| job.host == host).map(|job| {
				format!("spawned={}/{} queue={}", job.spawned, job.expected, job.queue.len())
			})
		};
		format!(
			"spawn_request={spawn} fulfillment={fulfillment}",
			spawn = spawn.as_deref().unwrap_or("none"),
			fulfillment = fulfillment.as_deref().unwrap_or("none"),
		)
	}

	fn wait_furnished_walk_colliders(
		app: &mut App,
		expected: &[(Id, usize)],
		max_frames: u32,
	) -> anyhow::Result<()> {
		for _ in 0..max_frames {
			if furnished_walk_colliders_ready(app, expected) {
				return Ok(());
			}
			app.update();
			let demand = app.world().resource::<HcsgDemand>().clone();
			if !demand.wait_idle(IDLE) {
				return Err(SettleTimedOut { timeout: IDLE }.into());
			}
		}
		let hosts = furnished_hosts(app);
		let by_id: std::collections::BTreeMap<_, _> =
			hosts.iter().map(|(entity, node)| (node.id, (*entity, node.clone()))).collect();
		let mut missing = Vec::new();
		for (id, slots) in expected {
			match by_id.get(id) {
				None => missing.push(format!("{id:?} host missing (expected {slots} slots)")),
				Some((entity, node)) => {
					if node.value.cell.slots.len() != *slots {
						missing.push(format!(
							"{id:?} slots {} != {slots}",
							node.value.cell.slots.len()
						));
					}
					if cell_needs_walk_collider(node)
						&& !subtree_has_walk_collider(app.world(), *entity)
					{
						missing.push(format!(
							"{id:?} furniture bears weight; {}",
							host_fulfill_state(app, *entity)
						));
					}
				}
			}
		}
		panic!(
			"furniture walk colliders missing after {max_frames} frames: {}",
			missing.join("; ")
		);
	}

	#[test]
	fn the_neighborhood_presents_every_furnished_cell_with_walk_colliders() -> anyhow::Result<()> {
		let mut app = app(center());
		settle(&mut app)?;

		let expected = expected_cells(&mut app)?;
		assert!(!expected.is_empty(), "Les Halles furnishes the neighborhood");
		wait_furnished_walk_colliders(&mut app, &expected, 64)?;
		assert_eq!(furnished(&mut app), expected, "one host per furnished cell, with its slots");
		let mut collider_cells = 0usize;
		for (entity, node) in furnished_hosts(&mut app) {
			if !cell_needs_walk_collider(&node) {
				continue;
			}
			collider_cells += 1;
			assert!(
				subtree_has_walk_collider(app.world(), entity),
				"{:?} furniture bears weight",
				node.id
			);
		}
		assert!(collider_cells > 0, "Les Halles furniture blocks walking");
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
