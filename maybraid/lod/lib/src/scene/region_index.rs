//! Region queries for LOD refresh (hosts overlapping an AABB).
//!
//! Separate from HCSG storage: refresh only needs “which [`LodScene`] hosts
//! overlap this AABB?”, not generated values or their `Id`s.

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::math::bounding::Aabb3d;

use crate::scene::SemanticLodScene;

/// Lookup of **any** [`crate::LodSceneHost`] whose host volume hits `region`.
///
/// Level and cull production fill a frame cache from this, then each host
/// type `T` filters with `Query<&T>`.
pub trait LodSceneHostIndex {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a;
}

/// Lookup of [`LodScene`] hosts of type `T` whose **host** volumes hit `region`.
///
/// Typed overlap lookup. Prefer [`LodSceneHostIndex`] plus a `Query<&T>` filter
/// so the Avian AABB query runs once per unique region, not once per `T`.
///
/// How hosts entered the world (authored, generated, etc.) is out of scope.
/// Volumes are typically stamped by [`crate::PatchSceneBounds`] from
/// [`crate::LodScene::scene_bounds`].
pub trait LodSceneRegionIndex<T: Component + SemanticLodScene> {
	fn hosts_in_region<'a>(
		&'a mut self,
		region: Aabb3d,
	) -> impl Iterator<Item = (Entity, &'a T)> + 'a;
}
