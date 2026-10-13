//! Avian physics layers for Maybraid LOD / terrain / motion.
//!
//! Scene-host refresh and cull live in [`lod-gimme`](lod_gimme). Call sites use
//! `gimme_host!` (`GimmeLodSceneRefreshPlugin` / `GimmeLodSceneCullPlugin`).
//! This crate still re-exports those plugins as [`AvianLodSceneRefreshPlugin`] /
//! [`AvianLodSceneCullPlugin`] for the unused `avian_host!` wrappers.
//!
//! Generate and present id lookup stays on [`lod::hcsg::HcsgStorage`].

mod layers;

pub use layers::{AvianLodHostVolume, AvianLodQueryVolume, PhysicsInteractionLayer};
pub use lod_gimme::{
	GimmeLodSceneCullPlugin as AvianLodSceneCullPlugin,
	GimmeLodSceneRefreshPlugin as AvianLodSceneRefreshPlugin,
};

use avian3d::prelude::{Collider, SpatialQuery, SpatialQueryFilter};
use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::scene::SemanticLodScene;
use lod::{LodSceneHost, LodSceneHostIndex, LodSceneRegionIndex};

/// [`lod::LodSceneBoundsMarshaller`] for scene-host volumes ([`PhysicsInteractionLayer::Host`]).
///
/// Refresh plugins do not install this marshaller; they use
/// [`lod_gimme::GimmeLodHostMarshaller`]. Kept so an Avian host query remains
/// possible without reintroducing Generate / Present volumes.
#[derive(Debug, Clone, Copy, Default)]
pub struct AvianLodSceneBoundsMarshaller;

fn region_hits(
	spatial: &SpatialQuery,
	region: Aabb3d,
	layer: PhysicsInteractionLayer,
) -> Vec<Entity> {
	let min = Vec3::from(region.min);
	let max = Vec3::from(region.max);
	let center = (min + max) * 0.5;
	let size = (max - min).max(Vec3::splat(1e-3));
	spatial.shape_intersections(
		&Collider::cuboid(size.x, size.y, size.z),
		center,
		Quat::IDENTITY,
		&SpatialQueryFilter::from_mask(layer),
	)
}

/// Untyped Avian host lookup.
///
/// Hits are already restricted to [`PhysicsInteractionLayer::Host`]. Refresh
/// and cull plugins use [`lod_gimme::GimmeLodSceneHostIndex`] instead.
#[derive(SystemParam)]
pub struct AvianLodSceneHostIndex<'w, 's> {
	spatial: SpatialQuery<'w, 's>,
}

impl LodSceneHostIndex for AvianLodSceneHostIndex<'_, '_> {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a {
		region_hits(&self.spatial, region, PhysicsInteractionLayer::Host).into_iter()
	}
}

/// [`SystemParam`] Avian implementation of [`LodSceneRegionIndex`] for host type `T`.
///
/// Layer-masked to [`PhysicsInteractionLayer::Host`], then resolved as `T`.
#[derive(SystemParam)]
pub struct AvianLodSceneRegionIndex<'w, 's, T: Component + SemanticLodScene + 'static> {
	spatial: SpatialQuery<'w, 's>,
	hosts: Query<'w, 's, &'static T, With<LodSceneHost>>,
}

impl<T: Component + SemanticLodScene + 'static> LodSceneRegionIndex<T>
	for AvianLodSceneRegionIndex<'_, '_, T>
{
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a {
		region_hits(&self.spatial, region, PhysicsInteractionLayer::Host)
			.into_iter()
			.filter_map(|entity| self.hosts.get(entity).ok().map(|scene| (entity, scene)))
	}
}
