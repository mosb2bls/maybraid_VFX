//! Richmond on the shared HCSG runtime ([`lod::hcsg`]): padded terrain
//! and built developments generate on the worker and present as [`HcsgNode`]
//! hosts.
//!
//! A session starts with [`lod::hcsg::request_hcsg_session_restart`].

use std::marker::PhantomData;

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::hcsg::universal_bounds;
use lod::hcsg::{self, register_session_seed, HcsgClass, PresentationPlugin, ViewerHcsgBounds};
use urbanization_cells::{UrbanizationExtent, UrbanizationSelection};

use crate::built::Built;
use crate::config::DevelopmentConfig;
use crate::developments::site::AuthoredDevelopments;
use crate::ground::RichmondGround;
use crate::layer_config::stream_radii_m;
use crate::layer_config::UrbanizationStreamSpec;
use crate::padded::PaddedTerrain;
use crate::plugin::register_richmond_plugin;

/// The urbanization cells within the default stream's present radius of the
/// [`LodViewer`], one box each.
pub struct DevelopmentNeighborhood;

impl ViewerHcsgBounds for DevelopmentNeighborhood {
	const CLASS: HcsgClass = HcsgClass::Near;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		let (present, _) = stream_radii_m(UrbanizationStreamSpec::default().stream_radius);
		let around = UrbanizationExtent::xz_radius_aabb(viewer, present);
		UrbanizationExtent::cells_overlapping(around)
			.into_iter()
			.map(|cell| cell.aabb())
			.collect()
	}
}

/// Presents built developments over ground `G` within channel `C`'s regions
/// from the shared storage.
pub struct BuiltPresentationPlugin<C, G>(PhantomData<fn() -> (C, G)>);

impl<C, G> Default for BuiltPresentationPlugin<C, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, G: RichmondGround> Plugin for BuiltPresentationPlugin<C, G> {
	fn build(&self, app: &mut App) {
		register_richmond_plugin(app);
		register_session_seed(app, seed_richmond_hcsg_roots::<G>);
		app.add_plugins(PresentationPlugin::<C, Built<G>>::default());
	}
}

/// Presents padded terrain and built developments over ground `G` within
/// channel `C`'s regions from the shared storage.
///
/// Padded terrain is the ground's whole surface, so the ground's own cells
/// are not presented beside it.
pub struct RichmondPresentationPlugin<C, G>(PhantomData<fn() -> (C, G)>);

impl<C, G> Default for RichmondPresentationPlugin<C, G> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C: Send + Sync + 'static, G: RichmondGround> Plugin for RichmondPresentationPlugin<C, G> {
	fn build(&self, app: &mut App) {
		app.add_plugins((
			BuiltPresentationPlugin::<C, G>::default(),
			PresentationPlugin::<C, PaddedTerrain<G>>::default(),
		));
	}
}

/// Richmond's root resources: the development config, the authored
/// developments and the urbanization selection.
#[derive(SystemParam)]
pub struct RichmondRoots<'w> {
	config: Res<'w, DevelopmentConfig>,
	authored: Res<'w, AuthoredDevelopments>,
	selection: Res<'w, UrbanizationSelection>,
}

impl RichmondRoots<'_> {
	/// Seeds Richmond and urbanization session roots over ground `G`.
	pub fn seed<G: RichmondGround>(&self, storage: &hcsg::HcsgStorage) {
		storage.seed(self.config.clone(), universal_bounds());
		storage.seed(self.authored.clone(), universal_bounds());
		storage.seed(self.selection.clone(), universal_bounds());
	}
}

/// Seeds [`RichmondRoots`] during an HCSG session restart.
pub fn seed_richmond_hcsg_roots<G: RichmondGround>(
	roots: RichmondRoots,
	storage: Res<hcsg::HcsgStorage>,
) {
	roots.seed::<G>(storage.as_ref());
}

#[cfg(test)]
mod tests {
	use std::time::Duration;

	use bevy::ecs::entity_disabling::Disabled;
	use bevy::math::bounding::Aabb3d;
	use bevy::scene::ScenePlugin;
	use bevy::state::app::StatesPlugin;
	use building_physics::BuildingWalkCollider;
	use durham::{
		fine_patch_cell_layout, CellTiling, Durham, DurhamWindow, Terrain, TerrainCellLayout,
		TerrainColliderMeshSource, TerrainConfig, TerrainMeshAssets, TerrainMeshLodBand,
		TerrainStampConfigs, WaterMeshAssets, WaterPresentationPlugin, WatershedConfigs,
	};
	use lod::gen::{Id, OriginalId};
	use lod::hcsg::{
		hcsg_hosts, settle_presentation, wait_replaced_hosts, HcsgDemand, HcsgRestartRequest,
		SettlePresentation, SettleTimedOut,
	};
	use lod::lod_ref::LodNodePose;
	use lod::{LodChunkFulfillment, LodLevelSpawnRequest, LodViewer};
	use terrain_layer_model::OnTerrain;
	use urbanization_layer_model::UrbanSetting;

	use super::*;
	use crate::config::DevelopmentSites;
	use crate::developments::site::{AuthoredDevelopment, DevelopmentKind};

	type Ground = OnTerrain<Durham>;

	const IDLE: Duration = Duration::from_secs(120);

	/// One Les Halles authored inside one cell of the patch at `height`; no
	/// pad reaches the other three.
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
		app.insert_resource(TerrainStampConfigs::from_world_seed(1))
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
			.insert_resource(DevelopmentConfig {
				sites: DevelopmentSites::Authored,
				..DevelopmentConfig::default()
			})
			.insert_resource(AuthoredDevelopments(vec![authored(height)]))
			.insert_resource(HcsgRestartRequest::queued());
	}

	/// A 2×2 fine patch at the origin with Richmond presented over it.
	fn app(height: f32) -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, StatesPlugin))
			.add_plugins((AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.insert_resource(fine_patch_cell_layout(1, IVec2::new(-1, -1)))
			.insert_resource(WaterMeshAssets { material: Handle::default() })
			.init_resource::<UrbanizationSelection>()
			.add_plugins((
				hcsg::HcsgBoundsPlugin::<DurhamWindow>::default(),
				WaterPresentationPlugin::<DurhamWindow>::default(),
				RichmondPresentationPlugin::<DurhamWindow, Ground>::default(),
			));
		seed_resources(&mut app, height);
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

	fn count<C: Component>(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<(), (With<C>, Allow<Disabled>)>()
			.iter(app.world())
			.count()
	}

	/// Padded height at the authored development's center.
	fn pad_height(app: &App) -> anyhow::Result<f32> {
		let cell = authored(0.0).cell;
		let center = (Vec3::from(cell.min) + Vec3::from(cell.max)) * 0.5;
		let storage = app.world().resource::<hcsg::HcsgStorage>();
		let region = Aabb3d::new(center, Vec3::splat(0.5));
		let mut padded = None;
		for id in storage
			.try_overlapping::<PaddedTerrain<Ground>>(region)
			.map_err(|busy| anyhow::anyhow!("HcsgStorage busy: {busy:?}"))?
		{
			if let Some(entry) = storage
				.try_entry::<PaddedTerrain<Ground>>(id)
				.map_err(|busy| anyhow::anyhow!("HcsgStorage busy: {busy:?}"))?
			{
				padded = Some(entry.value);
				break;
			}
		}
		let padded =
			padded.ok_or_else(|| anyhow::anyhow!("no padded cell under the development"))?;
		Ok(padded.surface.sdf.terrain().height_at_with_all_modulations(center.x, center.z))
	}

	#[test]
	fn padded_terrain_is_the_whole_ground_surface() -> anyhow::Result<()> {
		let mut app = app(12.0);
		settle(&mut app)?;

		let layout = app.world().resource::<TerrainCellLayout>().clone();
		let mut cells: Vec<Id> = layout
			.cell_ids(layout.request_region())
			.into_iter()
			.map(|OriginalId(id)| id)
			.collect();
		cells.sort();
		let padded: Vec<Id> = hcsg_hosts::<PaddedTerrain<Ground>>(&mut app)
			.into_iter()
			.map(|(id, _)| id)
			.collect();
		assert_eq!(padded, cells, "one padded host per ground cell, padded or not");
		assert!(hcsg_hosts::<Terrain>(&mut app).is_empty(), "raw ground is never presented");
		assert_eq!(count::<TerrainColliderMeshSource>(&mut app), cells.len());
		assert!((pad_height(&app)? - 12.0).abs() < 1e-3, "the pad flattens to its terrace");
		Ok(())
	}

	#[test]
	fn a_built_development_presents_its_setting_and_buildings() -> anyhow::Result<()> {
		let mut app = app(12.0);
		settle(&mut app)?;

		let id = authored(12.0).id();
		let built: Vec<Id> =
			hcsg_hosts::<Built<Ground>>(&mut app).into_iter().map(|(id, _)| id).collect();
		assert_eq!(built, vec![id]);
		let settings: Vec<UrbanSetting> = app
			.world_mut()
			.query_filtered::<&UrbanSetting, Allow<Disabled>>()
			.iter(app.world())
			.copied()
			.collect();
		assert_eq!(settings.len(), 1);
		assert_eq!(settings[0].id, id);
		wait_building_walk_colliders(&mut app, 64)?;
		assert!(count::<BuildingWalkCollider>(&mut app) > 0, "buildings bear weight");
		Ok(())
	}

	/// Building colliders attach after nested host scenes fulfill. Worker-idle
	/// plus eight frames does not guarantee that under the shared 2 ms drain.
	fn wait_building_walk_colliders(app: &mut App, max_frames: u32) -> anyhow::Result<()> {
		for _ in 0..max_frames {
			if count::<BuildingWalkCollider>(app) > 0 {
				return Ok(());
			}
			app.update();
			let demand = app.world().resource::<HcsgDemand>().clone();
			if !demand.wait_idle(IDLE) {
				return Err(SettleTimedOut { timeout: IDLE }.into());
			}
		}
		let built = hcsg_hosts::<Built<Ground>>(app);
		let mut states = Vec::new();
		for (id, _) in &built {
			let Some(host) = app
				.world_mut()
				.query_filtered::<(Entity, &lod::hcsg::HcsgNode<Built<Ground>>), Allow<Disabled>>()
				.iter(app.world())
				.find_map(|(entity, node)| (node.id == *id).then_some(entity))
			else {
				states.push(format!("{id:?} host missing"));
				continue;
			};
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
			states.push(format!(
				"{id:?} spawn_request={spawn} fulfillment={fulfillment}",
				spawn = spawn.as_deref().unwrap_or("none"),
				fulfillment = fulfillment.as_deref().unwrap_or("none"),
			));
		}
		panic!(
			"buildings bear weight: missing BuildingWalkCollider after {max_frames} frames; {}",
			states.join("; ")
		);
	}

	#[test]
	fn a_restart_replaces_padded_and_built_hosts() -> anyhow::Result<()> {
		let mut app = app(12.0);
		settle(&mut app)?;
		let padded = hcsg_hosts::<PaddedTerrain<Ground>>(&mut app);
		let built = hcsg_hosts::<Built<Ground>>(&mut app);

		seed_resources(&mut app, 20.0);
		settle(&mut app)?;
		let padded_after =
			wait_replaced_hosts::<PaddedTerrain<Ground>>(&mut app, &padded, IDLE, 64)?;
		let built_after = wait_replaced_hosts::<Built<Ground>>(&mut app, &built, IDLE, 64)?;

		for (before, after) in [(padded, padded_after), (built, built_after)] {
			assert_eq!(after.len(), before.len());
			for ((id, old), (new_id, new)) in before.iter().zip(&after) {
				assert_eq!(id, new_id);
				assert!(new > old, "{id:?} still shows the previous session");
			}
		}
		assert!((pad_height(&app)? - 20.0).abs() < 1e-3, "the new terrace height");
		Ok(())
	}
}
