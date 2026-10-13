//! Barking on the shared HCSG runtime ([`lod::hcsg`]): each occupied
//! mob cell places its group on the worker and presents its mobs in a
//! [`PlacedMobCell`] host's scene.
//!
//! A group samples the forest and urbanization selections, the urban hosts
//! built over the ground, and the ground's surface, all from the generation
//! context. Mobs retire with their cell, and their members with them.
//!
//! A session starts with [`lod::hcsg::request_hcsg_session_restart`].

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::DVec3;
use bevy::prelude::*;
use chico::{ForestGround, GroundSurface};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext, HcsgClass, PresentationPlugin, ViewerHcsgBounds};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use mob_intelligence::{MemberOf, Mob};
use richmond::{column_bounds, Built, DevelopmentHosts, Richmond, RichmondGround};
use urbanization_cells::{
	SelectedUrbanization, UrbanDevelopmentKind, UrbanizationKind, UrbanizationSelection,
};
use urbanization_layer_model::Urbanization;

use crate::generation::{
	MobEnvironmentSample, MobGroup, MobPlantHost, MobWorldHosts, MobWorldSample,
};
use crate::index::{group_kind, hosts_near, urbanization_weight, MOB_CELL_EXTENT};
use crate::present::{install_mob_scenes, MobCellRoot, MobGroupRoot};
use crate::sample::{chico_layers_at, host_at, richmond_kind_at};
use crate::{MobCell, MobCellExtent};

/// Present / generate rings the world stream used (1 km / 3 km).
pub const MOB_PRESENT_RADIUS: f32 = 1_000.0;

/// The widest arrival radius a plant host has.
const HOST_REACH: f32 = 128.0;

/// The ground mobs stand on, and where urban families gather on it.
pub trait MobGround: ForestGround {
	/// Plant hosts over `region`: urban leaves, development settings, and the
	/// places their buildings mark.
	fn plant_hosts(cx: &mut GenerationContext, region: Aabb3d) -> Vec<MobPlantHost>;
}

impl<T: RichmondGround> MobGround for Urbanization<Richmond<T>>
where
	Self: ForestGround,
{
	fn plant_hosts(cx: &mut GenerationContext, region: Aabb3d) -> Vec<MobPlantHost> {
		let mut hosts = Vec::new();
		for OriginalId(id) in cx.original_ids_for::<SelectedUrbanization>(region) {
			let Some(selected) = cx.get_or_generate::<SelectedUrbanization>(id) else {
				continue;
			};
			hosts.extend(
				selected
					.leaves
					.iter()
					.filter(|leaf| {
						leaf.kind != UrbanDevelopmentKind::Empty && region.intersects(&leaf.bounds)
					})
					.map(|leaf| host_at(leaf.bounds)),
			);
		}
		for OriginalId(id) in cx.original_ids_for::<Built<T>>(region) {
			let Some(built) = cx.get_or_generate::<Built<T>>(id) else {
				continue;
			};
			hosts.push(MobPlantHost {
				xz: built.setting_at.xz(),
				arrival_radius: built.setting.arrival_radius,
			});
			hosts.extend(built.development.hosts().iter().filter_map(|host| {
				let place = host.discoverable_place()?;
				Some(MobPlantHost {
					xz: host.transform().translation.xz(),
					arrival_radius: place.arrival_radius,
				})
			}));
		}
		hosts
	}
}

/// What one cell's group samples.
struct CellWorld<G: ForestGround> {
	forest: chico::ForestSelection,
	urban: Arc<UrbanizationSelection>,
	surface: GroundSurface<G>,
	hosts: Vec<MobPlantHost>,
}

impl<G: ForestGround> CellWorld<G> {
	fn kind_at(&self, xz: Vec2) -> UrbanizationKind {
		richmond_kind_at(self.urban.noise, self.urban.kind, xz)
	}
}

/// Off the surface there is no elevation, so no mob stands there.
impl<G: ForestGround> MobWorldSample for CellWorld<G> {
	fn sample_mobs(&self, xz: Vec2) -> MobEnvironmentSample {
		MobEnvironmentSample {
			elevation: self.surface.height(xz.x, xz.y),
			urbanization: urbanization_weight(self.kind_at(xz)),
			vegetation: chico_layers_at(self.forest.noise, self.forest.layering, xz) as f32 / 4.0,
		}
	}
}

impl<G: ForestGround> MobWorldHosts for CellWorld<G> {
	fn plant_hosts(&self, origin: Vec2, extent: f32) -> Vec<MobPlantHost> {
		hosts_near(&self.hosts, origin, extent)
	}
}

/// One occupied cell's mobs, placed on ground `G`.
pub struct PlacedMobCell<G> {
	pub cell: MobCell,
	_ground: PhantomData<fn() -> G>,
}

impl<G: MobGround> hcsg::GenerationScheme for PlacedMobCell<G> {
	lod::hcsg_index_scale!(MOB_CELL_SCALE);

	fn original_ids_for(_cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		MobCellExtent::occupied_original_ids_overlapping(region)
	}

	/// `None` off the ground.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let extent = MobCellExtent::from_id(id)?;
		let seed = extent.group_seed()?;
		let forest = *cx.get::<chico::ForestSelection>(Id::Universal)?;
		let urban = cx.get::<UrbanizationSelection>(Id::Universal)?;
		let column = column_bounds(extent.aabb());
		let surface = GroundSurface::<G>::generate(cx, column);
		let footprint = surface.footprint()?;
		let reach = Aabb3d::from_min_max(
			Vec3::from(column.min) - Vec3::new(HOST_REACH, 0.0, HOST_REACH),
			Vec3::from(column.max) + Vec3::new(HOST_REACH, 0.0, HOST_REACH),
		);
		let world = CellWorld { forest, urban, surface, hosts: G::plant_hosts(cx, reach) };
		let origin = extent.center().xz();
		let kind = group_kind(world.kind_at(origin), seed);
		let group = MobGroup::generate(kind, seed, origin, &world);
		let bounds = Aabb3d::from_min_max(
			Vec3::new(column.min.x, footprint.min.y, column.min.z),
			Vec3::new(column.max.x, footprint.max.y, column.max.z),
		);
		let cell = MobCell { extent, groups: vec![group] };
		Some((Self { cell, _ground: PhantomData }, bounds))
	}
}

/// One level: a root per group holding a mob host per placed mob, each
/// streaming its own High band.
impl<G: MobGround> LodScene for PlacedMobCell<G> {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		let (ix, iz) = self.cell.extent.index();
		let groups: Vec<Box<dyn Scene>> = self
			.cell
			.groups
			.iter()
			.map(|group| {
				let mobs: Vec<Box<dyn Scene>> = group
					.mobs
					.iter()
					.map(|placed| Box::new(placed.scene.scene(placed.transform)) as Box<dyn Scene>)
					.collect();
				let name = Name::new(format!("{:?} mob group", group.kind));
				Box::new(bsn! {
					template_value(name)
					MobGroupRoot
					Transform::default()
					Visibility::default()
					Children [ {mobs} ]
				}) as Box<dyn Scene>
			})
			.collect();
		let name = Name::new(format!("mob-cell {ix},{iz}"));
		bsn! {
			template_value(name)
			MobCellRoot
			Transform::default()
			Visibility::default()
			Children [ {groups} ]
		}
	}
}

/// Half-height of a mob neighborhood: every surface a mob stands on.
const MOB_COLUMN_Y: f32 = 10_000.0;

/// The mob cells around the [`LodViewer`]: within [`MOB_PRESENT_RADIUS`] of
/// its cell.
pub struct MobNeighborhood;

impl MobNeighborhood {
	fn around(viewer: Vec3, radius: f32) -> Aabb3d {
		let (ix, iz) = MobCellExtent::cell_index_containing(viewer);
		let center = MobCellExtent::from_cell_index(ix, iz).center();
		Aabb3d::from_min_max(
			Vec3::new(center.x - radius, -MOB_COLUMN_Y, center.z - radius),
			Vec3::new(center.x + radius, MOB_COLUMN_Y, center.z + radius),
		)
	}
}

impl ViewerHcsgBounds for MobNeighborhood {
	const CLASS: HcsgClass = HcsgClass::Near;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		vec![Self::around(viewer, MOB_PRESENT_RADIUS)]
	}
}

const MOB_CELL_SCALE: DVec3 = DVec3::new(MOB_CELL_EXTENT as f64, 1.0, MOB_CELL_EXTENT as f64);
/// Members are not children of their mob. When presentation despawns a mob
/// host with its scene, its members go with it; despawn the host in `Last`
/// so commands queued on either through `PostUpdate` still land.
pub fn retire_members_with_their_mob(
	despawned: On<Despawn, Mob>,
	members: Query<(Entity, &MemberOf)>,
	mut commands: Commands,
) {
	for (entity, member) in &members {
		if member.mob == despawned.entity {
			commands.entity(entity).try_despawn();
		}
	}
}

/// The mob hosts' runtime: their High band refreshed around the
/// [`LodViewer`], their members bound by intelligence, and those members
/// retired with their mob.
pub struct MobScenePresentationPlugin;

impl Plugin for MobScenePresentationPlugin {
	fn build(&self, app: &mut App) {
		install_mob_scenes(app);
		app.add_observer(retire_members_with_their_mob);
	}
}

/// Presents placed mob cells over ground `G` within channel `C`'s regions
/// from the shared storage. Their mob hosts stream under [`MobScenePresentationPlugin`].
pub struct BarkingPresentationPlugin<C, G>(PhantomData<fn() -> (C, G)>);

impl<C, G> Default for BarkingPresentationPlugin<C, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, G: MobGround> Plugin for BarkingPresentationPlugin<C, G> {
	fn build(&self, app: &mut App) {
		app.add_plugins(PresentationPlugin::<C, PlacedMobCell<G>>::default());
	}
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::entity_disabling::Disabled;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use chico::ChicoPresentationPlugin;
	use durham::{
		fine_patch_cell_layout, Durham, DurhamWindow, TerrainConfig, TerrainMeshAssets,
		TerrainMeshLodBand, TerrainStampConfigs, WaterMeshAssets, WaterPresentationPlugin,
		WatershedConfigs,
	};
	use lod::hcsg::{settle_presentation, HcsgNode, HcsgRestartRequest, SettlePresentation};
	use lod::lod_ref::LodNodePose;
	use lod::LodViewer;
	use mob_scenes::{MobKind, MobScene};
	use richmond::BuiltPresentationPlugin;
	use richmond::{
		AuthoredDevelopment, AuthoredDevelopments, DevelopmentConfig, DevelopmentKind,
		DevelopmentSites,
	};
	use terrain_layer_model::OnTerrain;

	use super::*;
	use crate::GroupKind;

	type Ground = OnTerrain<Durham>;
	type Urban = Urbanization<Richmond<Ground>>;

	const IDLE: Duration = Duration::from_secs(300);

	/// The 2×2 fine patch's footprint.
	const PATCH: f32 = 160.0;

	fn pin(app: &mut App, kind: UrbanizationKind) {
		app.insert_resource(UrbanizationSelection { kind: Some(kind), ..default() })
			.world_mut()
			.resource_mut::<HcsgRestartRequest>()
			.request();
	}

	/// One Les Halles inside one cell of the patch.
	fn les_halles() -> AuthoredDevelopment {
		AuthoredDevelopment {
			cell: Aabb3d::from_min_max(Vec3::new(10.0, 0.0, 10.0), Vec3::new(150.0, 1.0, 150.0)),
			kinds: vec![DevelopmentKind::LesHalles],
			height: 12.0,
			config: DevelopmentConfig::default(),
			courtyard: None,
		}
	}

	/// Mobs on a 2×2 fine patch at the origin, viewed from the origin.
	fn app(kind: UrbanizationKind) -> App {
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
			.init_resource::<chico::ForestSelection>()
			.insert_resource(HcsgRestartRequest::queued())
			.add_plugins((
				hcsg::HcsgBoundsPlugin::<MobNeighborhood>::default(),
				WaterPresentationPlugin::<DurhamWindow>::default(),
				BuiltPresentationPlugin::<MobNeighborhood, Ground>::default(),
				ChicoPresentationPlugin::<MobNeighborhood, Urban>::default(),
				BarkingPresentationPlugin::<MobNeighborhood, Urban>::default(),
			));
		pin(&mut app, kind);
		durham::register_durham_hcsg_session(&mut app);
		app.finish();
		app.cleanup();
		let at = Transform::IDENTITY;
		app.world_mut()
			.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
		app
	}

	fn settle(app: &mut App) -> anyhow::Result<()> {
		settle_presentation(app, SettlePresentation::layer_integration(IDLE)).map_err(Into::into)
	}

	fn placed(app: &mut App) -> Vec<HcsgNode<PlacedMobCell<Urban>>> {
		app.world_mut()
			.query::<&HcsgNode<PlacedMobCell<Urban>>>()
			.iter(app.world())
			.cloned()
			.collect()
	}

	fn mob_hosts(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<(), (With<MobScene>, Allow<Disabled>)>()
			.iter(app.world())
			.count()
	}

	fn mobs(app: &mut App) -> Vec<crate::PlacedMob> {
		placed(app)
			.iter()
			.flat_map(|node| node.value.cell.groups.iter().flat_map(|group| group.mobs.clone()))
			.collect()
	}

	fn surface(app: &App) -> GroundSurface<Urban> {
		let storage = app.world().resource::<hcsg::HcsgStorage>().clone();
		let mut cx = GenerationContext::new(&storage);
		GroundSurface::<Urban>::generate(
			&mut cx,
			column_bounds(Aabb3d::new(Vec3::ZERO, Vec3::splat(PATCH))),
		)
	}

	#[test]
	fn the_origin_cell_stands_its_mobs_on_the_surface() -> anyhow::Result<()> {
		let mut app = app(UrbanizationKind::RuralLife);
		settle(&mut app)?;

		let mobs = mobs(&mut app);
		assert!(!mobs.is_empty(), "the origin cell is always occupied");
		let surface = surface(&app);
		for mob in &mobs {
			let at = mob.transform.translation;
			let ground = surface
				.height(at.x, at.z)
				.ok_or_else(|| anyhow::anyhow!("{at} stands off the ground"))?;
			assert!((at.y - ground).abs() < 1e-3, "{at} stands off the ground at {ground}");
		}

		for _ in 0..200 {
			if mob_hosts(&mut app) >= mobs.len() {
				break;
			}
			app.update();
		}
		assert_eq!(mob_hosts(&mut app), mobs.len(), "one mob host per placed mob");
		Ok(())
	}

	#[test]
	fn urban_families_gather_at_built_places() -> anyhow::Result<()> {
		let mut app = app(UrbanizationKind::Frontier);
		settle(&mut app)?;

		let storage = app.world().resource::<hcsg::HcsgStorage>().clone();
		let mut cx = GenerationContext::new(&storage);
		let hosts =
			Urban::plant_hosts(&mut cx, column_bounds(Aabb3d::new(Vec3::ZERO, Vec3::splat(400.0))));
		let setting = hosts
			.iter()
			.find(|host| host.xz == Vec2::splat(80.0))
			.ok_or_else(|| anyhow::anyhow!("the development's setting is a host"))?;
		assert!(setting.arrival_radius > 0.0);

		let families: Vec<_> = mobs(&mut app)
			.into_iter()
			.filter(|mob| {
				matches!(mob.scene.mob.kind, MobKind::Guard | MobKind::Brawler | MobKind::Pleb)
			})
			.collect();
		assert!(!families.is_empty(), "a frontier group has urban families");
		for mob in families {
			let xz = mob.transform.translation.xz();
			assert!(
				hosts.iter().any(|host| xz.distance(host.xz) <= host.arrival_radius + 1e-3),
				"{xz} gathers at no host"
			);
		}
		Ok(())
	}

	#[test]
	fn leaving_the_neighborhood_retires_its_mobs() -> anyhow::Result<()> {
		let mut app = app(UrbanizationKind::RuralLife);
		settle(&mut app)?;
		assert!(mob_hosts(&mut app) > 0);

		let away = Transform::from_xyz(5_000.0, 0.0, 5_000.0);
		let mut viewers = app.world_mut().query_filtered::<&mut Transform, With<LodViewer>>();
		*viewers.single_mut(app.world_mut())? = away;
		settle(&mut app)?;
		assert!(placed(&mut app).is_empty());
		assert_eq!(mob_hosts(&mut app), 0);
		Ok(())
	}

	#[test]
	fn a_restart_regroups_every_cell() -> anyhow::Result<()> {
		let mut app = app(UrbanizationKind::RuralLife);
		settle(&mut app)?;
		for node in placed(&mut app) {
			assert!(node.value.cell.groups.iter().all(|group| group.kind == GroupKind::Peaceful));
		}

		pin(&mut app, UrbanizationKind::None);
		settle(&mut app)?;
		let nodes = placed(&mut app);
		assert!(!nodes.is_empty());
		for node in nodes {
			assert!(node.value.cell.groups.iter().all(|group| group.kind == GroupKind::Wild));
		}
		Ok(())
	}
}
