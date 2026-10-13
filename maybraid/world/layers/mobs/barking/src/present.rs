//! Placed scenes, group roots, and the High LOD pulse.

use std::time::Duration;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::time::common_conditions::on_timer;
use lod::lod_ref::LodRef;
use lod::scene::{LodRefreshRegions, LodRefreshRegionsStatus};
use lod::{
	update_lod_host_levels, LodNode, LodNodePose, LodRefreshDomain, LodRefreshSystems,
	LodSceneRefreshAabb, LodSceneRefreshRegion, LodSceneRefreshRegionPlugin, LodViewer,
};
use lod_gimme::GimmeLodSceneRefreshPlugin;
use mob_scenes::{MobLodRefreshMode, MobScene};

use crate::plugin::MobGroupsPlugin;

/// Half-extent of the High produce cube. Sized a margin past the 200 m High sphere.
pub const MOB_HIGH_LOD_REFRESH_RADIUS: f32 = 250.0;
pub const MOB_HIGH_LOD_REFRESH_INTERVAL: Duration = Duration::from_millis(250);
pub const MOB_HIGH_LOD_RECONCILE_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Clone, Copy, Debug, Default)]
pub struct MobHighLodChan;

#[derive(Resource, Default)]
pub struct MobHighLodRegion;

impl MobHighLodRegion {
	pub fn region_at(center: Vec3) -> Aabb3d {
		let half = Vec3::splat(MOB_HIGH_LOD_REFRESH_RADIUS);
		Aabb3d::from_min_max(center - half, center + half)
	}
}

impl LodRefreshRegions for MobHighLodRegion {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		if lod_ref.previous_transform.translation == lod_ref.current_transform.translation {
			LodRefreshRegionsStatus::Unchanged
		} else {
			LodRefreshRegionsStatus::Changed(Self::region_at(lod_ref.current_transform.translation))
		}
	}
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MobCellRoot;

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct MobGroupRoot;

pub fn pulse_mob_high_lod(
	nodes: Query<&LodNodePose, (With<LodNode>, With<LodViewer>)>,
	mut refresh: MessageWriter<LodSceneRefreshRegion<MobHighLodChan>>,
	mut bus: MessageWriter<LodSceneRefreshAabb>,
) {
	let regions = nodes.iter().map(|pose| MobHighLodRegion::region_at(pose.current.translation));
	let union = regions.reduce(|a, b| Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max)));
	if let Some(region) = union {
		refresh.write(LodSceneRefreshRegion::new(region));
		bus.write(LodSceneRefreshAabb { region, domain: LodRefreshDomain::of::<MobHighLodChan>() });
	}
}

/// Mob scenes, with their High band refreshed around the [`LodViewer`].
pub(crate) fn install_mob_scenes(app: &mut App) {
	// Indexed must be visible when MobScenesPlugin builds. Guarding
	// MobGroupsPlugin would hide an assembler that already added groups
	// under FullScan.
	app.insert_resource(MobLodRefreshMode::Indexed);
	app.add_plugins(MobGroupsPlugin);
	app.add_plugins(
		LodSceneRefreshRegionPlugin::<MobHighLodRegion, With<LodViewer>, MobHighLodChan>::default(),
	)
	.add_plugins(GimmeLodSceneRefreshPlugin::<MobScene, MobHighLodChan, With<LodViewer>>::default())
	.add_systems(
		Update,
		pulse_mob_high_lod
			.run_if(on_timer(MOB_HIGH_LOD_REFRESH_INTERVAL))
			.in_set(LodRefreshSystems::ProduceRegions),
	)
	.add_systems(
		Update,
		update_lod_host_levels::<MobScene, (), With<LodViewer>>
			.run_if(on_timer(MOB_HIGH_LOD_RECONCILE_INTERVAL))
			.in_set(LodRefreshSystems::UpdateLevels),
	);
}
