//! Produce [`LodSceneRefreshLevel`] from region impulses and a spatial index.

use std::any::TypeId;
use std::marker::PhantomData;

use bevy::ecs::entity_disabling::Disabled;
use bevy::ecs::system::{StaticSystemParam, SystemParam};
use bevy::math::bounding::Aabb3d;
use bevy::platform::collections::{HashMap, HashSet};
use bevy::prelude::*;

use crate::lod_ref::{
	collect_node_snapshots_into, lod_refs_from_snapshots, LodNode, LodNodeBounds, LodNodePose,
	LodNodeSnapshot,
};
use crate::scene::host::{
	nested_host_parent_allows_refresh, nested_host_parent_allows_refresh_world, LodLevelRoot,
	LodLevelRoots, LodSceneHost,
};
use crate::scene::level::LodSceneLevel;
use crate::scene::region_index::LodSceneHostIndex;
use crate::scene::{LodSceneCulls, SceneChunk, SemanticLodScene};

use super::super::{ensure_refresh_core, LodLevelProduceSystems};

/// Impulse: set host `entity` toward `level` (folded by max in entity refresh).
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshLevel {
	pub entity: Entity,
	pub level: LodSceneLevel,
}

type ProduceLevelFn =
	for<'a> fn(&World, Entity, &[crate::lod_ref::LodRef<'a>]) -> Option<LodSceneLevel>;
type ProduceOneLevelFn =
	for<'a> fn(&World, Entity, &crate::lod_ref::LodRef<'a>) -> Option<LodSceneLevel>;
type ProduceCullsFn =
	for<'a> fn(&World, Entity, &crate::lod_ref::LodRef<'a>, LodSceneLevel) -> Option<LodSceneCulls>;
type ProduceChunkFn =
	for<'a> fn(&World, Entity, &crate::lod_ref::LodRef<'a>, LodSceneLevel) -> Option<SceneChunk>;

/// Type-erased level callback stamped when a semantic host component is added.
///
/// The shared producer uses this to visit each spatial hit once, independent of
/// the number of registered host types.
#[derive(Component, Clone, Copy)]
pub struct LodLevelProducer {
	level_from_all: ProduceLevelFn,
	level: ProduceOneLevelFn,
	culls: ProduceCullsFn,
	chunk: ProduceChunkFn,
}

impl LodLevelProducer {
	fn for_scene<T>() -> Self
	where
		T: Component + SemanticLodScene + 'static,
	{
		Self {
			level_from_all: |world, entity, refs| {
				world.get::<T>(entity).map(|scene| scene.scene_lod_level_from_levels(refs))
			},
			level: |world, entity, lod_ref| {
				world.get::<T>(entity).map(|scene| scene.scene_lod_level(lod_ref))
			},
			culls: |world, entity, lod_ref, current| {
				world.get::<T>(entity).map(|scene| scene.scene_lod_culls(lod_ref, current))
			},
			chunk: |world, entity, lod_ref, level| {
				world
					.get::<T>(entity)
					.map(|scene| scene.scene_chunks_with_level(lod_ref, level))
			},
		}
	}

	pub(crate) fn level_for(
		self,
		world: &World,
		entity: Entity,
		lod_ref: &crate::lod_ref::LodRef,
	) -> Option<LodSceneLevel> {
		(self.level)(world, entity, lod_ref)
	}

	pub(crate) fn culls_for(
		self,
		world: &World,
		entity: Entity,
		lod_ref: &crate::lod_ref::LodRef,
		current: LodSceneLevel,
	) -> Option<LodSceneCulls> {
		(self.culls)(world, entity, lod_ref, current)
	}

	pub(crate) fn chunk_for(
		self,
		world: &World,
		entity: Entity,
		lod_ref: &crate::lod_ref::LodRef,
		level: LodSceneLevel,
	) -> Option<SceneChunk> {
		(self.chunk)(world, entity, lod_ref, level)
	}
}

fn attach_lod_level_producer<T>(add: On<Add, T>, mut commands: Commands)
where
	T: Component + SemanticLodScene + 'static,
{
	if let Ok(mut entity) = commands.get_entity(add.entity) {
		entity.insert(LodLevelProducer::for_scene::<T>());
	}
}

pub(crate) fn attach_refresh_membership<T, D>(add: On<Add, T>, mut commands: Commands)
where
	T: Component,
	D: Send + Sync + 'static,
{
	if let Ok(mut entity) = commands.get_entity(add.entity) {
		entity.insert(LodRefreshMembership(LodRefreshDomain::of::<D>()));
	}
}

/// Semantic produce domain. Channels that share a domain (bullseye + spotlight)
/// union into one spatial query; mob High does not widen vegetation.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LodRefreshDomain(TypeId);

impl LodRefreshDomain {
	pub fn of<T: 'static>() -> Self {
		Self(TypeId::of::<T>())
	}
}

/// Hosts stamped with this only emit levels for matching [`LodSceneRefreshAabb::domain`].
///
/// Unstamped hosts still participate in every domain (playgrounds / tests).
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct LodRefreshMembership(pub LodRefreshDomain);

/// Refresh AABB tagged with the channel's produce domain.
///
/// Region production writes this beside the typed [`super::super::LodSceneRefreshRegion`]
/// message. Fill unions regions **per domain**, then queries once per domain.
#[derive(Message, Debug, Clone, Copy)]
pub struct LodSceneRefreshAabb {
	pub region: Aabb3d,
	pub domain: LodRefreshDomain,
}

#[derive(Debug, Default)]
struct DomainHits {
	hits: HashSet<Entity>,
}

/// This-frame driver snapshots + per-domain host hits.
///
/// Filled once by [`fill_lod_produce_cache`], then consumed once by the erased
/// producer. Viewer snapshots are collected once even when several domains fire.
#[derive(Resource, Debug, Default)]
pub struct LodProduceCache {
	pub snapshots: Vec<LodNodeSnapshot>,
	domains: HashMap<LodRefreshDomain, DomainHits>,
}

impl LodProduceCache {
	fn clear(&mut self) {
		self.snapshots.clear();
		self.domains.clear();
	}

	/// Hosts retained for `domain` after the last fill (membership-filtered).
	pub fn hits_for(&self, domain: LodRefreshDomain) -> Option<&HashSet<Entity>> {
		self.domains.get(&domain).map(|hits| &hits.hits)
	}
}

fn remove_contained_regions(regions: &mut Vec<Aabb3d>) {
	regions.sort_by(|a, b| region_volume(*b).total_cmp(&region_volume(*a)));
	let mut index = 0;
	while index < regions.len() {
		let region = regions[index];
		if regions[..index].iter().any(|outer| contains_region(*outer, region)) {
			regions.remove(index);
		} else {
			index += 1;
		}
	}
}

fn region_volume(region: Aabb3d) -> f32 {
	let size = Vec3::from(region.max - region.min);
	size.x.max(0.0) * size.y.max(0.0) * size.z.max(0.0)
}

fn contains_region(outer: Aabb3d, inner: Aabb3d) -> bool {
	outer.min.cmple(inner.min).all() && outer.max.cmpge(inner.max).all()
}

/// Collect every [`LodNode`] snapshot once, then query hosts per produce domain.
///
/// Region production still filters drivers (`With<Camera>` vs [`super::super::LodViewer`]).
/// Fill must not: those filters used to instantiate two systems that each cleared
/// [`LodProduceCache`], so the second walk wiped the first.
pub fn fill_lod_produce_cache<I>(
	mut regions: MessageReader<LodSceneRefreshAabb>,
	index: StaticSystemParam<I>,
	nodes: Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), With<LodNode>>,
	membership: Query<&LodRefreshMembership>,
	mut cache: ResMut<LodProduceCache>,
) where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	cache.clear();
	if regions.is_empty() {
		return;
	}

	let mut by_domain: HashMap<LodRefreshDomain, Vec<Aabb3d>> = HashMap::new();
	for msg in regions.read() {
		let list = by_domain.entry(msg.domain).or_default();
		if !list.contains(&msg.region) {
			list.push(msg.region);
		}
	}
	if by_domain.is_empty() {
		return;
	}

	collect_node_snapshots_into(&nodes, &mut cache.snapshots);
	if cache.snapshots.is_empty() {
		return;
	}

	let mut index = index.into_inner();
	for (domain, mut regions) in by_domain {
		remove_contained_regions(&mut regions);
		let mut hits = HashSet::new();
		for region in &regions {
			index.extend_hosts_in_region(*region, &mut hits);
		}
		hits.retain(|entity| membership_allows(membership.get(*entity).ok(), domain));
		cache.domains.insert(domain, DomainHits { hits });
	}
}

/// Emit [`LodSceneRefreshLevel`] for hosts `T` overlapping this frame's regions.
pub fn produce_lod_refresh_levels<T>(
	cache: Res<LodProduceCache>,
	hosts: Query<&T, With<LodSceneHost>>,
	membership: Query<&LodRefreshMembership>,
	mut levels: MessageWriter<LodSceneRefreshLevel>,
	child_of: Query<&ChildOf, Allow<Disabled>>,
	host_levels: Query<&LodSceneLevel, (With<LodSceneHost>, Allow<Disabled>)>,
	level_roots: Query<&LodLevelRoot, Allow<Disabled>>,
	children_q: Query<&Children, Allow<Disabled>>,
	level_roots_bags: Query<(), (With<LodLevelRoots>, Allow<Disabled>)>,
	visibilities: Query<(&Visibility, Has<Disabled>), Allow<Disabled>>,
) where
	T: Component + SemanticLodScene + 'static,
{
	if cache.domains.is_empty() || cache.snapshots.is_empty() {
		return;
	}
	let refs = lod_refs_from_snapshots(&cache.snapshots);
	for (domain, hits) in &cache.domains {
		for &entity in &hits.hits {
			if !membership_allows(membership.get(entity).ok(), *domain) {
				continue;
			}
			let Ok(scene) = hosts.get(entity) else {
				continue;
			};
			if !nested_host_parent_allows_refresh(
				entity,
				&child_of,
				&host_levels,
				&level_roots,
				&children_q,
				&level_roots_bags,
				&visibilities,
			) {
				continue;
			}
			let level = scene.scene_lod_level_from_levels(&refs);
			levels.write(LodSceneRefreshLevel { entity, level });
		}
	}
}

/// Emit levels once from the shared spatial-hit cache.
pub fn produce_lod_refresh_levels_erased(world: &mut World) {
	world.resource_scope(|world, cache: Mut<LodProduceCache>| {
		if cache.domains.is_empty() || cache.snapshots.is_empty() {
			return;
		}
		let refs = lod_refs_from_snapshots(&cache.snapshots);
		for (domain, hits) in &cache.domains {
			for &entity in &hits.hits {
				if !host_matches_domain(world, entity, *domain) {
					continue;
				}
				if !nested_host_parent_allows_refresh_world(world, entity) {
					continue;
				}
				let Some(producer) = world.get::<LodLevelProducer>(entity).copied() else {
					continue;
				};
				let Some(level) = (producer.level_from_all)(world, entity, &refs) else {
					continue;
				};
				world.write_message(LodSceneRefreshLevel { entity, level });
			}
		}
	});
}

fn membership_allows(membership: Option<&LodRefreshMembership>, domain: LodRefreshDomain) -> bool {
	membership.is_none_or(|membership| membership.0 == domain)
}

fn host_matches_domain(world: &World, entity: Entity, domain: LodRefreshDomain) -> bool {
	membership_allows(world.get::<LodRefreshMembership>(entity), domain)
}

/// Fill [`LodProduceCache`] from untyped region AABBs via host index `I`.
///
/// Once per `I`. Snapshot collection is every [`LodNode`], not a node filter:
/// vegetation `With<Camera>` and mob [`super::super::LodViewer`] share this plugin.
pub struct LodSceneRefreshLevelsFillPlugin<I>
where
	I: SystemParam + 'static,
{
	_marker: PhantomData<fn() -> I>,
}

impl<I> Default for LodSceneRefreshLevelsFillPlugin<I>
where
	I: SystemParam + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<I> Plugin for LodSceneRefreshLevelsFillPlugin<I>
where
	I: SystemParam + 'static,
	for<'w, 's> I::Item<'w, 's>: LodSceneHostIndex,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_systems(
			Update,
			fill_lod_produce_cache::<I>.in_set(LodLevelProduceSystems::FillCache),
		);
	}
}

/// Register host `T` with the shared erased level producer.
pub struct LodSceneRefreshLevelsPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	_marker: PhantomData<fn() -> T>,
}

impl<T> Default for LodSceneRefreshLevelsPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	fn default() -> Self {
		Self { _marker: PhantomData }
	}
}

impl<T> Plugin for LodSceneRefreshLevelsPlugin<T>
where
	T: Component + SemanticLodScene + 'static,
{
	fn build(&self, app: &mut App) {
		ensure_refresh_core(app);
		app.add_observer(attach_lod_level_producer::<T>);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn contained_refresh_region_is_removed_before_spatial_query() {
		let outer = Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0));
		let inner = Aabb3d::from_min_max(Vec3::splat(-10.0), Vec3::splat(10.0));
		let mut regions = vec![inner, outer];
		remove_contained_regions(&mut regions);
		assert_eq!(regions, vec![outer]);
	}

	struct VegDomain;
	struct MobDomain;

	#[test]
	fn overlapping_channels_share_a_domain_without_a_second_query_region() {
		let outer = Aabb3d::from_min_max(Vec3::splat(-100.0), Vec3::splat(100.0));
		let inner = Aabb3d::from_min_max(Vec3::splat(-10.0), Vec3::splat(10.0));
		let mut veg = vec![inner, outer];
		remove_contained_regions(&mut veg);
		assert_eq!(veg, vec![outer]);
		assert_ne!(LodRefreshDomain::of::<VegDomain>(), LodRefreshDomain::of::<MobDomain>());
	}
}
