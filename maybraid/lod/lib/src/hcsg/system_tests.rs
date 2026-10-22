//! App tests for [`super::GenerationPlugin`] and [`super::PresentationPlugin`],
//! with the real worker.

use bevy::ecs::system::SystemParamItem;
use bevy::math::bounding::{Aabb3d, IntersectsVolume};
use bevy::math::{DVec3, Vec3};
use bevy::prelude::*;
use bevy::scene::ScenePlugin;

use crate::gen::tests::test_utils::{cell, stub_scene, Terrain, Vegetation};
use crate::gen::{Id, OriginalId, Version};
use crate::lod_ref::{LodNodePose, LodRef};
use crate::scene::host::LodLevelSpawnRequest;
use crate::scene::refresh::LodSceneRefreshChunkPlugin;
use crate::scene::{LodHostBounds, LodSceneHost, LodViewer};
use crate::scene::{LodScene, LodSceneLevel};

use super::{
	GenerationContext, GenerationPlugin, GenerationScheme, HcsgBounds, HcsgBoundsPlugin, HcsgClass,
	HcsgDemand, HcsgNode, HcsgRestartRequest, HcsgStorage, PresentationPlugin, RetiredHost,
	SettlePresentation,
};

#[derive(Resource, Default)]
struct Window(Vec<Aabb3d>);

struct WindowBounds;

impl HcsgBounds for WindowBounds {
	const CLASS: HcsgClass = HcsgClass::Near;

	type Param = Res<'static, Window>;

	fn regions(window: &SystemParamItem<Self::Param>) -> Vec<Aabb3d> {
		window.0.clone()
	}
}

fn span(x: f32, width: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + width, 1.0, 1.0))
}

fn ids(xs: &[f32]) -> Vec<Id> {
	let mut ids: Vec<Id> = xs.iter().map(|&x| Id::from_cell(cell(x))).collect();
	ids.sort();
	ids
}

fn app(plugin: impl Plugin, regions: Vec<Aabb3d>) -> App {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins((AssetPlugin::default(), ScenePlugin))
		.insert_resource(Window(regions))
		.add_plugins((HcsgBoundsPlugin::<WindowBounds>::default(), plugin));
	let storage = app.world().resource::<HcsgStorage>().clone();
	storage.configure::<Terrain>(DVec3::splat(0.1));
	storage.configure::<Vegetation>(DVec3::splat(0.1));
	storage.configure::<Kept>(DVec3::splat(0.1));
	app
}

fn present_terrain(regions: Vec<Aabb3d>) -> App {
	app(PresentationPlugin::<WindowBounds, Terrain>::default(), regions)
}

#[test]
fn presentation_plugin_registers_chunk_refresh_by_default() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(PresentationPlugin::<WindowBounds, Terrain>::default());
	assert!(app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Terrain>>>());
}

#[test]
fn presentation_plugin_without_chunk_refresh_skips_chunk_refresh() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(PresentationPlugin::<WindowBounds, Terrain>::without_chunk_refresh());
	assert!(!app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Terrain>>>());
}

struct OtherPresentationChannel;

#[test]
fn presentation_plugin_chunk_refresh_is_shared_across_channels() {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins);
	app.add_plugins(PresentationPlugin::<WindowBounds, Terrain>::default());
	app.add_plugins(PresentationPlugin::<OtherPresentationChannel, Terrain>::default());
	assert!(app.is_plugin_added::<LodSceneRefreshChunkPlugin<HcsgNode<Terrain>>>());
}

fn set_window(app: &mut App, regions: Vec<Aabb3d>) {
	app.world_mut().resource_mut::<Window>().0 = regions;
}

fn settle(app: &mut App) -> anyhow::Result<()> {
	SettlePresentation::PLUGIN_TESTS.settle(app).map_err(Into::into)
}

fn host_versions(hosts: &[(Id, Version, Entity)]) -> Vec<(Id, Version)> {
	hosts.iter().map(|(id, version, _)| (*id, *version)).collect()
}

fn hosts(app: &mut App) -> Vec<(Id, Version, Entity)> {
	let mut hosts: Vec<_> = app
		.world_mut()
		.query_filtered::<(Entity, &HcsgNode<Terrain>), Without<RetiredHost>>()
		.iter(app.world())
		.map(|(entity, node)| (node.id, node.version, entity))
		.collect();
	hosts.sort();
	hosts
}

fn hosted_ids(app: &mut App) -> Vec<Id> {
	hosts(app).into_iter().map(|(id, _, _)| id).collect()
}

#[test]
fn presentation_spawns_one_host_per_published_value() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	assert_eq!(hosted_ids(&mut app), ids(&[0.0, 1.0, 2.0]));

	for (id, _, entity) in hosts(&mut app) {
		let bounds = app.world().get::<LodHostBounds>(entity).map(|bounds| bounds.0);
		assert_eq!(bounds, id.origin_cell_bounds(), "hosts carry their stored world bounds");
	}
	Ok(())
}

#[test]
fn generation_follows_changed_bounds() -> anyhow::Result<()> {
	let mut app =
		app(GenerationPlugin::<WindowBounds, Vegetation>::default(), vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	let storage = app.world().resource::<HcsgStorage>().clone();
	for id in ids(&[0.0, 1.0, 2.0]) {
		anyhow::ensure!(
			storage.contains::<Vegetation>(id),
			"initial window should warm vegetation at {id:?}"
		);
	}

	set_window(&mut app, vec![span(1.2, 2.6)]);
	settle(&mut app)?;
	for id in ids(&[1.0, 2.0, 3.0]) {
		anyhow::ensure!(
			storage.contains::<Vegetation>(id),
			"moved window should warm vegetation at {id:?}"
		);
	}
	anyhow::ensure!(
		!storage.contains::<Vegetation>(Id::from_cell(cell(0.0))),
		"a cell that left the window is evicted with the old subscription"
	);
	Ok(())
}

#[test]
fn presentation_follows_the_regions_it_is_sent() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	let kept = hosts(&mut app).into_iter().find(|(id, _, _)| *id == Id::from_cell(cell(1.0)));

	set_window(&mut app, vec![span(1.2, 2.6)]);
	settle(&mut app)?;
	assert_eq!(hosted_ids(&mut app), ids(&[1.0, 2.0, 3.0]));
	let still = hosts(&mut app).into_iter().find(|(id, _, _)| *id == Id::from_cell(cell(1.0)));
	assert_eq!(kept, still, "a host in both sets is kept, not respawned");

	let storage = app.world().resource::<HcsgStorage>().clone();
	assert!(
		!storage.contains::<Terrain>(Id::from_cell(cell(0.0))),
		"a host that left the window is evicted with it"
	);
	assert!(
		storage.contains::<Terrain>(Id::from_cell(cell(1.0))),
		"a presented value is never evicted while its host is kept"
	);
	Ok(())
}

#[test]
fn presentation_covers_every_region_in_a_set() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 0.6), span(3.2, 0.6), span(3.4, 0.2)]);
	settle(&mut app)?;
	assert_eq!(hosted_ids(&mut app), ids(&[0.0, 3.0]), "one host per cell across the set");
	Ok(())
}

/// Hosts alive in `PostUpdate`, where systems still queue commands on them.
#[derive(Resource, Default)]
struct PostUpdateHosts(usize);

fn count_post_update_hosts(
	hosts: Query<(), With<HcsgNode<Terrain>>>,
	mut seen: ResMut<PostUpdateHosts>,
) {
	seen.0 = hosts.iter().count();
}

#[test]
fn a_retired_host_lasts_the_frame() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	app.init_resource::<PostUpdateHosts>()
		.add_systems(PostUpdate, count_post_update_hosts);
	settle(&mut app)?;

	set_window(&mut app, Vec::new());
	app.update();
	assert_eq!(app.world().resource::<PostUpdateHosts>().0, 3);
	assert!(
		hosts(&mut app).is_empty(),
		"presented queries drop RetiredHost after Last even if leftovers linger"
	);
	let leftover: Vec<Entity> = app
		.world_mut()
		.query_filtered::<Entity, (With<RetiredHost>, With<HcsgNode<Terrain>>)>()
		.iter(app.world())
		.collect();
	assert!(
		leftover.len() <= 1,
		"cull-root cap (2) leaves at most one of the three hosts for a later Last: {}",
		leftover.len()
	);
	for entity in &leftover {
		assert!(
			app.world()
				.get::<Visibility>(*entity)
				.is_some_and(|v| matches!(*v, Visibility::Hidden)),
			"leftover retired hosts stay Hidden so they do not extract"
		);
	}
	for _ in 0..leftover.len() {
		app.update();
	}
	assert_eq!(
		app.world_mut()
			.query_filtered::<Entity, With<RetiredHost>>()
			.iter(app.world())
			.count(),
		0,
		"further Last ticks finish the leftover"
	);
	Ok(())
}

#[test]
fn presentation_without_regions_unsubscribes_and_retires() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	set_window(&mut app, Vec::new());
	settle(&mut app)?;
	assert!(hosts(&mut app).is_empty());
	Ok(())
}

#[test]
fn presentation_keeps_hosts_when_the_subscription_dies() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	let before = hosts(&mut app);
	let before_versions = host_versions(&before);
	anyhow::ensure!(!before.is_empty(), "need hosts before the subscription dies");

	app.world().resource::<HcsgDemand>().advance_epoch();
	app.update();
	let mid = hosts(&mut app);
	anyhow::ensure!(
		SettlePresentation::hosts_cover(&before_versions, &host_versions(&mid)),
		"a dead subscription must not drop hosts before replacements exist; before={before_versions:?} mid={:?}",
		host_versions(&mid)
	);

	SettlePresentation::PLUGIN_TESTS.wait_replaced::<Terrain>(
		&mut app,
		&before_versions,
		SettlePresentation::PLUGIN_TEST_REPLACE_FRAMES,
	)?;
	let after = hosts(&mut app);
	assert_eq!(after.len(), before.len());
	for ((id, version, entity), (before_id, before_version, before_entity)) in
		after.iter().zip(&before)
	{
		assert_eq!(id, before_id);
		assert!(
			version > before_version,
			"the worker republishes a newer version after resubscribe"
		);
		assert_ne!(entity, before_entity, "replacement hosts wait until the new publish is ready");
	}
	Ok(())
}

#[test]
fn presentation_keeps_hosts_until_world_session_restart_replacements_are_ready(
) -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	let before = hosts(&mut app);
	anyhow::ensure!(!before.is_empty(), "need hosts before the world session restarts");

	app.world_mut().resource_mut::<HcsgRestartRequest>().request();
	app.update();
	let mid = hosts(&mut app);
	anyhow::ensure!(
		SettlePresentation::hosts_cover(&host_versions(&before), &host_versions(&mid)),
		"old presentation stays up until the new session publishes replacements; before={:?} mid={:?}",
		host_versions(&before),
		host_versions(&mid)
	);

	SettlePresentation::PLUGIN_TESTS.wait_replaced::<Terrain>(
		&mut app,
		&host_versions(&before),
		SettlePresentation::PLUGIN_TEST_REPLACE_FRAMES,
	)?;
	let after = hosts(&mut app);
	assert_eq!(after.len(), before.len());
	for ((id, version, entity), (before_id, before_version, before_entity)) in
		after.iter().zip(&before)
	{
		assert_eq!(id, before_id);
		assert!(version > before_version, "restart rebuilds a newer stored version");
		assert_ne!(entity, before_entity, "replacement hosts are new entities");
	}
	Ok(())
}

#[test]
fn presentation_replaces_hosts_whose_value_changed_across_an_epoch() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	let before = hosts(&mut app);

	app.world().resource::<HcsgDemand>().advance_epoch();
	app.world().resource::<HcsgStorage>().clear::<Terrain>();
	SettlePresentation::PLUGIN_TESTS.wait_replaced::<Terrain>(
		&mut app,
		&host_versions(&before),
		SettlePresentation::PLUGIN_TEST_REPLACE_FRAMES,
	)?;

	let after = hosts(&mut app);
	assert_eq!(after.len(), before.len());
	for ((id, version, entity), (before_id, before_version, before_entity)) in
		after.iter().zip(&before)
	{
		assert_eq!(id, before_id);
		assert!(version > before_version);
		assert_ne!(entity, before_entity);
		assert!(
			app.world().get_entity(*before_entity).is_err()
				|| app.world().get::<RetiredHost>(*before_entity).is_some(),
			"stale host is retired or already despawned"
		);
	}
	Ok(())
}

impl GenerationScheme for Terrain {
	const INDEX_SCALE: DVec3 = DVec3::splat(0.1);

	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		(region.min.x.floor() as i32..=region.max.x.ceil() as i32)
			.map(|x| OriginalId(Id::from_cell(cell(x as f32))))
			.filter(|OriginalId(id)| id.origin_cell_bounds().is_some_and(|b| region.intersects(&b)))
			.collect()
	}

	fn build_with_id(_: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		Some((Self { cell: bounds }, bounds))
	}
}

/// Stands on the [`Terrain`] at its own id.
impl GenerationScheme for Vegetation {
	const INDEX_SCALE: DVec3 = DVec3::splat(0.1);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<Terrain>(region)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let terrain = cx.get_or_generate::<Terrain>(id)?;
		Some((Self { cell: terrain.cell }, terrain.cell))
	}
}

/// The cells a session keeps.
struct Keep(Vec<f32>);

/// One unit cell, only where the session's [`Keep`] lists it.
struct Kept;

impl GenerationScheme for Kept {
	const INDEX_SCALE: DVec3 = DVec3::splat(0.1);

	fn original_ids_for(_: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let (start, end) = (region.min.x.floor() as i32, region.max.x.ceil() as i32);
		(start..end).map(|x| OriginalId::new(Id::from_cell(cell(x as f32)))).collect()
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let bounds = id.origin_cell_bounds()?;
		let keep = cx.get::<Keep>(Id::Universal)?;
		keep.0.contains(&bounds.min.x).then_some((Self, bounds))
	}
}

impl LodScene for Kept {
	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		stub_scene()
	}
}

#[test]
fn a_new_session_retires_hosts_it_does_not_publish() -> anyhow::Result<()> {
	let mut app = app(PresentationPlugin::<WindowBounds, Kept>::default(), vec![span(0.2, 2.6)]);
	let storage = app.world().resource::<HcsgStorage>().clone();
	storage.seed(Keep(vec![0.0, 1.0, 2.0]), span(-10.0, 30.0));
	settle(&mut app)?;
	let kept = |app: &mut App| {
		let mut ids: Vec<Id> = app
			.world_mut()
			.query_filtered::<&HcsgNode<Kept>, Without<RetiredHost>>()
			.iter(app.world())
			.map(|n| n.id)
			.collect();
		ids.sort();
		ids
	};
	assert_eq!(kept(&mut app), ids(&[0.0, 1.0, 2.0]));

	app.world().resource::<HcsgDemand>().advance_epoch();
	storage.clear::<Kept>();
	storage.seed(Keep(vec![1.0]), span(-10.0, 30.0));
	let expected = ids(&[1.0]);
	SettlePresentation::PLUGIN_TESTS.wait_until(
		&mut app,
		SettlePresentation::PLUGIN_TEST_REPLACE_FRAMES,
		|app| kept(app) == expected,
	)?;
	assert_eq!(kept(&mut app), expected, "cells the session dropped are retired");
	Ok(())
}

#[test]
fn presented_hosts_fulfill_through_the_lod_pipeline() -> anyhow::Result<()> {
	let mut app = present_terrain(vec![span(0.2, 2.6)]);
	let at = Transform::from_translation(Vec3::new(1.0, 0.0, 0.0));
	app.world_mut()
		.spawn((LodViewer, at, LodNodePose { previous: at, current: at }));
	settle(&mut app)?;
	for _ in 0..4 {
		app.update();
	}

	let hosts = hosts(&mut app);
	assert_eq!(hosts.len(), 3);
	for (_, _, host) in hosts {
		assert!(app.world().get::<LodSceneHost>(host).is_some());
		assert!(
			app.world().get::<LodLevelSpawnRequest>(host).is_none(),
			"the forwarded scene was fulfilled"
		);
	}
	Ok(())
}

#[test]
fn generation_keeps_values_warm_without_hosts() -> anyhow::Result<()> {
	let mut app =
		app(GenerationPlugin::<WindowBounds, Vegetation>::default(), vec![span(0.2, 2.6)]);
	settle(&mut app)?;
	let storage = app.world().resource::<HcsgStorage>().clone();
	for id in ids(&[0.0, 1.0, 2.0]) {
		assert!(storage.contains::<Vegetation>(id) && storage.contains::<Terrain>(id));
	}
	assert!(app
		.world_mut()
		.query::<&HcsgNode<Vegetation>>()
		.iter(app.world())
		.next()
		.is_none());

	app.world().resource::<HcsgDemand>().advance_epoch();
	storage.clear::<Vegetation>();
	let id = Id::from_cell(cell(1.0));
	SettlePresentation::PLUGIN_TESTS.wait_until(
		&mut app,
		SettlePresentation::PLUGIN_TEST_REPLACE_FRAMES,
		|app| app.world().resource::<HcsgStorage>().contains::<Vegetation>(id),
	)?;
	assert!(storage.contains::<Vegetation>(id), "resubscribed after the epoch");
	Ok(())
}
