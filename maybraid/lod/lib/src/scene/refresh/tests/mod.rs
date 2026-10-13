//! App tests for the message-driven LOD refresh pipeline.

mod test_utils;

use bevy::prelude::*;

use bevy::math::bounding::Aabb3d;
use bevy::scene::ScenePlugin;

use crate::lod_ref::{LodNode, LodNodePose};
use crate::scene::host::LodLevelSpawnRequest;
use crate::scene::level::LodSceneLevel;
use crate::scene::refresh::{
	LodChunkFulfillBudget, LodCullProduceCache, LodCullProduceCadence, LodCullRegionCursor,
	LodHostBounds, LodHostHasCullableRoots, LodLevelRootPending, LodProduceCache, LodRefreshDomain,
	LodRefreshMembership, LodSceneCullAabb, LodSceneCullProduceFillPlugin, LodSceneRefreshAabb,
	LodSceneRefreshChunkPlugin, LodSceneRefreshLevel, LodSceneRefreshLevelsFillPlugin,
	LodSceneRefreshPlugin, LodSceneRegionCullPlugin, LodViewer,
};

use test_utils::{
	app_bullseye_regions, app_core, app_cull_enqueue, app_dual_channel_levels, app_entities_only,
	app_open_lattice, app_spotlight_levels, app_spotlight_regions, host_level, move_viewer, pose,
	spawn_host, spawn_host_with_roots, spawn_nested_pair, spawn_viewer, BullChan, CullChan,
	NewCullRegions, NewRegions, Probe, ScanHostIndex, SpotChan,
};

#[test]
fn track_updates_pose_on_transform_change() -> anyhow::Result<()> {
	let mut app = app_core();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert_eq!(pose(&app, viewer), (Vec3::ZERO, Vec3::ZERO));

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(pose(&app, viewer), (Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)));

	app.update();
	assert_eq!(
		pose(&app, viewer),
		(Vec3::ZERO, Vec3::new(10.0, 0.0, 0.0)),
		"still frame must not collapse previous/current"
	);
	assert!(app.world().entity(viewer).get::<LodNodePose>().is_some());
	Ok(())
}

#[test]
fn spotlight_silent_until_translation_changes() -> anyhow::Result<()> {
	let mut app = app_spotlight_regions();
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert!(app.world().resource::<NewRegions<SpotChan>>().regions.is_empty());

	app.update();
	assert!(app.world().resource::<NewRegions<SpotChan>>().regions.is_empty());
	Ok(())
}

#[test]
fn spotlight_emits_cube_on_translation() -> anyhow::Result<()> {
	let mut app = app_spotlight_regions();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	let regions = &app.world().resource::<NewRegions<SpotChan>>().regions;
	assert_eq!(regions.len(), 1);
	assert_eq!(regions[0].min.x, -15.0);
	assert_eq!(regions[0].max.x, 35.0);
	Ok(())
}

#[test]
fn bullseye_silent_inside_inner_cell() -> anyhow::Result<()> {
	let mut app = app_bullseye_regions();
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(10.0, 10.0, 10.0));
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(20.0, 20.0, 20.0));
	app.update();
	assert!(app.world().resource::<NewRegions<BullChan>>().regions.is_empty());
	Ok(())
}

#[test]
fn bullseye_emits_outer_cube_on_cell_cross() -> anyhow::Result<()> {
	let mut app = app_bullseye_regions();
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(-25.0, 0.0, 0.0));
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 10.0, 10.0));
	app.update();
	let regions = &app.world().resource::<NewRegions<BullChan>>().regions;
	assert_eq!(regions.len(), 1);
	// cell (0,0,0) for 50m → center 25; outer 500 → [-225, 275]
	assert_eq!(regions[0].min.x, -225.0);
	assert_eq!(regions[0].max.x, 275.0);
	Ok(())
}

#[test]
fn spotlight_writes_level_for_host_in_region() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::UltraLow);

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn spotlight_skips_host_outside_region() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::new(1000.0, 0.0, 0.0), LodSceneLevel::UltraLow);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::UltraLow);
	Ok(())
}

#[test]
fn spotlight_bands_follow_viewer_distance() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);

	move_viewer(&mut app, viewer, Vec3::new(50.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::Medium);

	move_viewer(&mut app, viewer, Vec3::new(100.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::Low);
	Ok(())
}

#[test]
fn idle_frame_does_not_rewrite_level() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();
	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	assert_eq!(app.world().resource::<NewRegions<SpotChan>>().regions.len(), 1);

	app.update();
	assert!(app.world().resource::<NewRegions<SpotChan>>().regions.is_empty());
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn dual_channel_small_move_is_spotlight_only() -> anyhow::Result<()> {
	let mut app = app_dual_channel_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(10.0, 10.0, 10.0));
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(20.0, 10.0, 10.0));
	app.update();
	assert_eq!(app.world().resource::<NewRegions<SpotChan>>().regions.len(), 1);
	assert!(app.world().resource::<NewRegions<BullChan>>().regions.is_empty());
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn stamped_hosts_only_follow_their_domain_aabb() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	let vegetation = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	let mob = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);
	app.world_mut()
		.entity_mut(vegetation)
		.insert(LodRefreshMembership(LodRefreshDomain::of::<SpotChan>()));
	app.world_mut()
		.entity_mut(mob)
		.insert(LodRefreshMembership(LodRefreshDomain::of::<BullChan>()));
	app.update();

	let region = Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0));
	app.world_mut()
		.write_message(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<BullChan>() });
	app.update();
	assert_eq!(host_level(&app, vegetation), LodSceneLevel::UltraLow);
	assert_eq!(host_level(&app, mob), LodSceneLevel::High);

	app.world_mut()
		.write_message(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<SpotChan>() });
	app.update();
	assert_eq!(host_level(&app, vegetation), LodSceneLevel::High);
	assert_eq!(host_level(&app, mob), LodSceneLevel::High);
	Ok(())
}

#[test]
fn camera_and_viewer_refresh_plugins_share_one_produce_fill() -> anyhow::Result<()> {
	#[derive(Component)]
	struct CameraLike;

	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(LodSceneRefreshPlugin::<
			Probe,
			SpotChan,
			ScanHostIndex,
			With<LodViewer>,
		>::without_full_scan_cull())
		.add_plugins(LodSceneRefreshPlugin::<
			Probe,
			BullChan,
			ScanHostIndex,
			With<CameraLike>,
		>::without_full_scan_cull());
	assert!(
		app.is_plugin_added::<LodSceneRefreshLevelsFillPlugin<ScanHostIndex>>(),
		"fill is once per host index, not once per node filter"
	);

	app.world_mut().spawn((LodNode, Transform::from_xyz(8.0, 0.0, 0.0)));
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	let region = Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0));
	app.world_mut()
		.write_message(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<SpotChan>() });
	app.update();

	assert_eq!(
		app.world().resource::<LodProduceCache>().snapshots.len(),
		2,
		"one fill must snapshot every LodNode, not a Camera- or LodViewer-only subset"
	);
	Ok(())
}

#[test]
fn camera_and_viewer_cull_plugins_share_one_cull_fill() -> anyhow::Result<()> {
	#[derive(Component)]
	struct CameraLike;

	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins(
			LodSceneRegionCullPlugin::<ScanHostIndex, CullChan, Probe, With<LodViewer>>::default(),
		)
		.add_plugins(
			LodSceneRegionCullPlugin::<ScanHostIndex, SpotChan, Probe, With<CameraLike>>::default(),
		);
	assert!(
		app.is_plugin_added::<LodSceneCullProduceFillPlugin<ScanHostIndex>>(),
		"cull fill is once per host index, not once per node filter"
	);

	app.world_mut().spawn((LodNode, Transform::from_xyz(8.0, 0.0, 0.0)));
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.world_mut().write_message(world_cull_aabb());
	app.update();

	assert_eq!(
		app.world().resource::<LodCullProduceCache>().snapshots.len(),
		2,
		"one cull fill must snapshot every LodNode, not a Camera- or LodViewer-only subset"
	);
	Ok(())
}

#[test]
fn untyped_level_bus_keeps_max_across_messages() -> anyhow::Result<()> {
	let mut app = app_entities_only();
	let host = spawn_host(app.world_mut(), Vec3::ZERO, LodSceneLevel::UltraLow);

	app.world_mut()
		.write_message(LodSceneRefreshLevel { entity: host, level: LodSceneLevel::Low });
	app.world_mut()
		.write_message(LodSceneRefreshLevel { entity: host, level: LodSceneLevel::High });
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);
	Ok(())
}

#[test]
fn open_lattice_emits_while_camera_still() -> anyhow::Result<()> {
	let mut app = app_open_lattice();
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert_eq!(app.world().resource::<NewCullRegions<CullChan>>().regions.len(), 1);
	let next_after_first = app.world().resource::<LodCullRegionCursor>().next;
	assert_eq!(next_after_first, 1);

	app.update();
	assert_eq!(app.world().resource::<NewCullRegions<CullChan>>().regions.len(), 1);
	assert_eq!(app.world().resource::<LodCullRegionCursor>().next, 2);
	assert_eq!(app.world().resource::<LodCullRegionCursor>().anchor_cell, Some(IVec3::ZERO));
	Ok(())
}

#[test]
fn open_lattice_respects_produce_cadence() -> anyhow::Result<()> {
	let mut app = app_open_lattice();
	app.insert_resource(LodCullProduceCadence::every_n_frames(4));
	spawn_viewer(app.world_mut(), Vec3::ZERO);
	app.update();
	assert_eq!(app.world().resource::<NewCullRegions<CullChan>>().regions.len(), 1);
	app.update();
	assert!(app.world().resource::<NewCullRegions<CullChan>>().regions.is_empty());
	app.update();
	assert!(app.world().resource::<NewCullRegions<CullChan>>().regions.is_empty());
	app.update();
	assert!(app.world().resource::<NewCullRegions<CullChan>>().regions.is_empty());
	app.update();
	assert_eq!(app.world().resource::<NewCullRegions<CullChan>>().regions.len(), 1);
	Ok(())
}

#[test]
fn nested_host_under_hidden_root_is_not_refreshed() -> anyhow::Result<()> {
	let mut app = app_spotlight_levels();
	let viewer = spawn_viewer(app.world_mut(), Vec3::ZERO);
	let (_parent, allowed, blocked) = spawn_nested_pair(app.world_mut());
	app.update();

	move_viewer(&mut app, viewer, Vec3::new(10.0, 0.0, 0.0));
	app.update();
	assert_eq!(host_level(&app, allowed), LodSceneLevel::High);
	assert_eq!(host_level(&app, blocked), LodSceneLevel::UltraLow);
	Ok(())
}

fn world_cull_aabb() -> LodSceneCullAabb {
	LodSceneCullAabb { region: Aabb3d::from_min_max(Vec3::splat(-50.0), Vec3::splat(50.0)) }
}

#[test]
fn cull_produce_lowers_stale_desired_and_enqueues_high() -> anyhow::Result<()> {
	let mut app = app_cull_enqueue();
	spawn_viewer(app.world_mut(), Vec3::new(100.0, 0.0, 0.0));
	let (host, roots) = spawn_host_with_roots(
		app.world_mut(),
		Vec3::ZERO,
		LodSceneLevel::High,
		&[LodSceneLevel::High, LodSceneLevel::Medium],
	);
	app.update();
	assert_eq!(host_level(&app, host), LodSceneLevel::High);

	app.world_mut().write_message(world_cull_aabb());
	app.update();

	assert_eq!(host_level(&app, host), LodSceneLevel::Low);
	assert!(
		app.world().get_entity(roots[0]).is_err(),
		"High root must be culled once desired drops"
	);
	assert!(app.world().get_entity(roots[1]).is_ok(), "early Low keeps Medium warm");
	Ok(())
}

#[test]
fn cull_produce_lowers_and_walks_when_cullable_marker_is_absent() -> anyhow::Result<()> {
	let mut app = app_cull_enqueue();
	spawn_viewer(app.world_mut(), Vec3::new(100.0, 0.0, 0.0));
	let (host, roots) = spawn_host_with_roots(
		app.world_mut(),
		Vec3::ZERO,
		LodSceneLevel::High,
		&[LodSceneLevel::High],
	);
	app.update();
	assert!(
		app.world().get::<LodHostHasCullableRoots>(host).is_none(),
		"only the current root is present, so the marker stays off"
	);

	app.world_mut().write_message(world_cull_aabb());
	app.update();

	assert_eq!(host_level(&app, host), LodSceneLevel::Low);
	assert!(
		app.world().get_entity(roots[0]).is_err(),
		"lowering must still walk roots when the marker was stale"
	);
	Ok(())
}

#[test]
fn cull_produce_enqueues_non_desired_even_if_sibling_is_pending() -> anyhow::Result<()> {
	let mut app = app_cull_enqueue();
	spawn_viewer(app.world_mut(), Vec3::new(300.0, 0.0, 0.0));
	let (host, roots) = spawn_host_with_roots(
		app.world_mut(),
		Vec3::ZERO,
		LodSceneLevel::High,
		&[LodSceneLevel::High, LodSceneLevel::Medium],
	);
	app.world_mut().entity_mut(roots[0]).insert(LodLevelRootPending);
	app.update();

	app.world_mut().write_message(world_cull_aabb());
	app.update();

	assert_eq!(host_level(&app, host), LodSceneLevel::Low);
	assert!(app.world().get_entity(roots[0]).is_err(), "pending High must still be culled");
	assert!(
		app.world().get_entity(roots[1]).is_err(),
		"Medium sibling must be culled even if High is pending"
	);
	Ok(())
}

#[test]
fn cull_produce_skips_hidden_leaving_host() -> anyhow::Result<()> {
	let mut app = app_cull_enqueue();
	spawn_viewer(app.world_mut(), Vec3::new(100.0, 0.0, 0.0));
	let (host, roots) = spawn_host_with_roots(
		app.world_mut(),
		Vec3::ZERO,
		LodSceneLevel::High,
		&[LodSceneLevel::High, LodSceneLevel::Medium],
	);
	app.world_mut().entity_mut(host).insert(Visibility::Hidden);
	app.update();

	app.world_mut().write_message(world_cull_aabb());
	app.update();

	assert_eq!(
		host_level(&app, host),
		LodSceneLevel::High,
		"present-hidden host must not drop desired"
	);
	assert!(app.world().get_entity(roots[0]).is_ok(), "High root must not be scene-GC'd");
	assert!(app.world().get_entity(roots[1]).is_ok());
	assert!(app.world().get::<crate::LodCullInFlight>(roots[0]).is_none());
	Ok(())
}

#[test]
fn begin_skips_cull_worthy_desired() -> anyhow::Result<()> {
	let mut app = app_cull_enqueue();
	app.add_plugins(LodSceneRefreshChunkPlugin::<Probe>::default());
	spawn_viewer(app.world_mut(), Vec3::new(300.0, 0.0, 0.0));
	let (host, _) = spawn_host_with_roots(
		app.world_mut(),
		Vec3::ZERO,
		LodSceneLevel::High,
		&[LodSceneLevel::Medium],
	);
	app.world_mut()
		.entity_mut(host)
		.insert(LodLevelSpawnRequest { level: LodSceneLevel::High });
	app.update();

	assert_ne!(
		app.world().entity(host).get::<LodLevelSpawnRequest>().map(|r| r.level),
		Some(LodSceneLevel::High),
		"stale High must retarget, not stay queued"
	);
	let bag_children: Vec<Entity> = app
		.world()
		.entity(host)
		.get::<Children>()
		.into_iter()
		.flat_map(|c| c.iter())
		.collect();
	let mut pending_high = 0u32;
	for bag in bag_children {
		let Some(children) = app.world().entity(bag).get::<Children>() else {
			continue;
		};
		for child in children.iter() {
			let entity = app.world().entity(child);
			if entity.get::<crate::LodLevelRoot>().is_some_and(|r| r.0 == LodSceneLevel::High)
				&& entity.get::<LodLevelRootPending>().is_some()
			{
				pending_high += 1;
			}
		}
	}
	assert_eq!(pending_high, 0, "begin must not start a High root distance would cull");
	Ok(())
}

#[test]
fn begin_retargets_stale_high_to_camera_band() -> anyhow::Result<()> {
	let mut app = app_cull_enqueue();
	app.add_plugins(LodSceneRefreshChunkPlugin::<Probe>::default());
	spawn_viewer(app.world_mut(), Vec3::new(300.0, 0.0, 0.0));
	let (host, _) = spawn_host_with_roots(
		app.world_mut(),
		Vec3::ZERO,
		LodSceneLevel::High,
		&[LodSceneLevel::Medium],
	);
	app.world_mut()
		.entity_mut(host)
		.insert(LodLevelSpawnRequest { level: LodSceneLevel::High });
	app.update();

	assert_eq!(host_level(&app, host), LodSceneLevel::Low);
	assert_eq!(
		app.world().entity(host).get::<LodLevelSpawnRequest>().map(|r| r.level),
		Some(LodSceneLevel::Low)
	);
	let bag_children: Vec<Entity> = app
		.world()
		.entity(host)
		.get::<Children>()
		.into_iter()
		.flat_map(|c| c.iter())
		.collect();
	let mut pending_high = 0u32;
	for bag in bag_children {
		let Some(children) = app.world().entity(bag).get::<Children>() else {
			continue;
		};
		for child in children.iter() {
			let entity = app.world().entity(child);
			if entity.get::<crate::LodLevelRoot>().is_some_and(|r| r.0 == LodSceneLevel::High)
				&& entity.get::<LodLevelRootPending>().is_some()
			{
				pending_high += 1;
			}
		}
	}
	assert_eq!(pending_high, 0);
	Ok(())
}

/// Identity-transform hosts (forest groves) must rank by footprint center, not
/// ECS spawn order. Budget 1 begin: the nearer Medium request starts first.
#[test]
fn begin_admits_nearer_identity_host_first() -> anyhow::Result<()> {
	let mut app = App::new();
	app.add_plugins(MinimalPlugins)
		.add_plugins((AssetPlugin::default(), ScenePlugin))
		.add_plugins(LodSceneRefreshChunkPlugin::<Probe>::default())
		.insert_resource(LodChunkFulfillBudget { begins_per_frame: 1, ..Default::default() });
	let viewer = spawn_viewer(app.world_mut(), Vec3::new(40.0, 0.0, 0.0));
	app.world_mut().entity_mut(viewer).insert(LodNodePose {
		previous: Transform::from_translation(Vec3::new(40.0, 0.0, 0.0)),
		current: Transform::from_translation(Vec3::new(40.0, 0.0, 0.0)),
	});

	let (far, _) = spawn_host_with_roots(app.world_mut(), Vec3::ZERO, LodSceneLevel::Medium, &[]);
	app.world_mut().entity_mut(far).insert((
		Transform::IDENTITY,
		LodHostBounds(Aabb3d::from_min_max(
			Vec3::new(399.0, -1.0, -1.0),
			Vec3::new(401.0, 1.0, 1.0),
		)),
		LodLevelSpawnRequest { level: LodSceneLevel::Medium },
	));
	let (near, _) = spawn_host_with_roots(app.world_mut(), Vec3::ZERO, LodSceneLevel::Medium, &[]);
	app.world_mut().entity_mut(near).insert((
		Transform::IDENTITY,
		LodHostBounds(Aabb3d::from_min_max(Vec3::new(44.0, -1.0, -1.0), Vec3::new(46.0, 1.0, 1.0))),
		LodLevelSpawnRequest { level: LodSceneLevel::Medium },
	));

	app.update();

	assert_eq!(
		app.world().entity(near).get::<LodLevelSpawnRequest>().map(|r| r.level),
		None,
		"nearer host must take the only begin slot"
	);
	assert_eq!(
		app.world().entity(far).get::<LodLevelSpawnRequest>().map(|r| r.level),
		Some(LodSceneLevel::Medium),
		"farther host must stay queued at Medium"
	);
	assert_eq!(host_level(&app, near), LodSceneLevel::Medium);
	assert_eq!(host_level(&app, far), LodSceneLevel::Medium);
	Ok(())
}
