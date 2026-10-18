//! Gimme-backed [`LodSceneHost`] index ([#802](https://github.com/ramate-io/maybraid/issues/802)).
//!
//! Hosts are AABB cells. Each is stored in the canonical multi-resolution cells
//! it intersects, updated on transform / bounds change, and removed on despawn.

use bevy::ecs::system::SystemParam;
use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use gimme_core::SpatialIndex;
use lod::{
	LodHostBounds, LodRefreshSystems, LodSceneBoundsMarshaller, LodSceneHost, LodSceneHostIndex,
};

/// 64 m XZ cells; a tall Y slab so ground hosts stay in one vertical cell.
const HOST_BASE_SCALE: DVec3 = DVec3::new(64.0, 4_096.0, 64.0);

/// Marker stamped by [`GimmeLodHostMarshaller`] instead of an Avian Host collider.
#[derive(Component, Clone, Copy, Default)]
pub struct GimmeLodHostVolume;

/// Writes [`LodHostBounds`] without an Avian query volume.
#[derive(Debug, Clone, Copy, Default)]
pub struct GimmeLodHostMarshaller;

impl LodSceneBoundsMarshaller for GimmeLodHostMarshaller {
	type Volume = GimmeLodHostVolume;

	fn volume_from_bounds(_bounds: Aabb3d) -> Self::Volume {
		GimmeLodHostVolume
	}
}

/// Multi-resolution AABB index of live [`LodSceneHost`] entities.
#[derive(Resource)]
pub struct GimmeLodHostIndex(pub SpatialIndex<Entity>);

impl FromWorld for GimmeLodHostIndex {
	fn from_world(_world: &mut World) -> Self {
		Self(SpatialIndex::new(HOST_BASE_SCALE).expect("positive host base scale"))
	}
}

impl GimmeLodHostIndex {
	pub fn hosts_in_region(&self, region: Aabb3d) -> Vec<Entity> {
		self.0.query(region)
	}
}

/// Maintains [`GimmeLodHostIndex`] from host transforms and bounds.
pub struct GimmeLodHostPlugin;

impl Plugin for GimmeLodHostPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<GimmeLodHostIndex>().add_systems(
			Update,
			(remove_despawned_hosts, reindex_moved_hosts)
				.chain()
				.before(LodRefreshSystems::ProduceLevels),
		);
	}
}

/// [`LodSceneHostIndex`] over [`GimmeLodHostIndex`].
#[derive(SystemParam)]
pub struct GimmeLodSceneHostIndex<'w> {
	index: Res<'w, GimmeLodHostIndex>,
}

impl LodSceneHostIndex for GimmeLodSceneHostIndex<'_> {
	fn hosts_in_region<'a>(&'a mut self, region: Aabb3d) -> impl Iterator<Item = Entity> + 'a {
		self.index.hosts_in_region(region).into_iter()
	}

	fn extend_hosts_in_region(&mut self, region: Aabb3d, out: &mut HashSet<Entity>) {
		self.index.0.extend_intersecting(region, out);
	}
}

fn remove_despawned_hosts(
	mut index: ResMut<GimmeLodHostIndex>,
	mut removed: RemovedComponents<LodSceneHost>,
) {
	for entity in removed.read() {
		index.0.remove(entity);
	}
}

fn reindex_moved_hosts(
	mut index: ResMut<GimmeLodHostIndex>,
	hosts: Query<
		(Entity, &GlobalTransform, &LodHostBounds),
		(
			With<LodSceneHost>,
			Or<(Changed<GlobalTransform>, Changed<LodHostBounds>, Added<LodSceneHost>)>,
		),
	>,
) {
	for (entity, transform, bounds) in &hosts {
		let world = world_aabb(transform, bounds.0);
		if index.0.get(entity) == Some(world) {
			continue;
		}
		let _ = index.0.insert(entity, world);
	}
}

fn world_aabb(transform: &GlobalTransform, local: Aabb3d) -> Aabb3d {
	let min = Vec3::from(local.min);
	let max = Vec3::from(local.max);
	let corners = [
		Vec3::new(min.x, min.y, min.z),
		Vec3::new(min.x, min.y, max.z),
		Vec3::new(min.x, max.y, min.z),
		Vec3::new(min.x, max.y, max.z),
		Vec3::new(max.x, min.y, min.z),
		Vec3::new(max.x, min.y, max.z),
		Vec3::new(max.x, max.y, min.z),
		Vec3::new(max.x, max.y, max.z),
	];
	let mut wmin = Vec3::splat(f32::MAX);
	let mut wmax = Vec3::splat(f32::MIN);
	for corner in corners {
		let p = transform.transform_point(corner);
		wmin = wmin.min(p);
		wmax = wmax.max(p);
	}
	Aabb3d::from_min_max(wmin, wmax)
}

#[cfg(test)]
mod fill_bench;

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::math::Vec3;

	fn unit_bounds() -> LodHostBounds {
		LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-1.0), Vec3::splat(1.0)))
	}

	fn around(center: Vec3, pad: f32) -> Aabb3d {
		Aabb3d::from_min_max(center - Vec3::splat(pad), center + Vec3::splat(pad))
	}

	#[test]
	fn spawn_indexes_and_despawn_removes() {
		let mut app = App::new();
		app.add_plugins((TransformPlugin, GimmeLodHostPlugin));
		let entity = app
			.world_mut()
			.spawn((LodSceneHost, Transform::from_xyz(10.0, 0.0, 0.0), unit_bounds()))
			.id();
		app.update();
		assert_eq!(
			app.world()
				.resource::<GimmeLodHostIndex>()
				.hosts_in_region(around(Vec3::X * 10.0, 2.0)),
			vec![entity]
		);

		app.world_mut().despawn(entity);
		app.update();
		assert!(app
			.world()
			.resource::<GimmeLodHostIndex>()
			.hosts_in_region(around(Vec3::X * 10.0, 2.0))
			.is_empty());
	}

	#[test]
	fn transform_change_reindexes() {
		let mut app = App::new();
		app.add_plugins((TransformPlugin, GimmeLodHostPlugin));
		let entity = app.world_mut().spawn((LodSceneHost, Transform::IDENTITY, unit_bounds())).id();
		app.update();

		app.world_mut().entity_mut(entity).insert(Transform::from_xyz(400.0, 0.0, 0.0));
		app.update();
		app.update();
		let index = app.world().resource::<GimmeLodHostIndex>();
		assert!(index.hosts_in_region(around(Vec3::ZERO, 2.0)).is_empty());
		assert_eq!(index.hosts_in_region(around(Vec3::X * 400.0, 2.0)), vec![entity]);
	}

	#[test]
	fn extend_hosts_in_region_unions_overlaps_and_preserves_existing() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;

		#[derive(Resource)]
		struct Collected(HashSet<Entity>);

		let mut app = App::new();
		app.add_plugins((TransformPlugin, GimmeLodHostPlugin));
		let near = app.world_mut().spawn((LodSceneHost, Transform::IDENTITY, unit_bounds())).id();
		let mid = app
			.world_mut()
			.spawn((LodSceneHost, Transform::from_xyz(20.0, 0.0, 0.0), unit_bounds()))
			.id();
		let far = app
			.world_mut()
			.spawn((LodSceneHost, Transform::from_xyz(400.0, 0.0, 0.0), unit_bounds()))
			.id();
		let already = app
			.world_mut()
			.spawn((LodSceneHost, Transform::from_xyz(800.0, 0.0, 0.0), unit_bounds()))
			.id();
		let coarse = app
			.world_mut()
			.spawn((
				LodSceneHost,
				Transform::IDENTITY,
				LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-200.0), Vec3::splat(200.0))),
			))
			.id();
		app.update();

		let regions = [around(Vec3::ZERO, 2.0), around(Vec3::X * 20.0, 8.0)];
		let mut expected = HashSet::from_iter([already]);
		{
			let index = app.world().resource::<GimmeLodHostIndex>();
			for region in regions {
				expected.extend(index.hosts_in_region(region));
			}
		}
		assert!(expected.contains(&near));
		assert!(expected.contains(&mid));
		assert!(expected.contains(&coarse));
		assert!(!expected.contains(&far));

		app.insert_resource(Collected(HashSet::from_iter([already])));
		app.world_mut()
			.run_system_once(
				move |mut index: GimmeLodSceneHostIndex, mut collected: ResMut<Collected>| {
					for region in regions {
						index.extend_hosts_in_region(region, &mut collected.0);
					}
				},
			)
			.map_err(|error| anyhow::anyhow!("{error}"))?;
		assert_eq!(app.world().resource::<Collected>().0, expected);
		Ok(())
	}

	#[test]
	fn small_query_finds_a_large_host() {
		let mut app = App::new();
		app.add_plugins((TransformPlugin, GimmeLodHostPlugin));
		let entity = app
			.world_mut()
			.spawn((
				LodSceneHost,
				Transform::IDENTITY,
				LodHostBounds(Aabb3d::from_min_max(Vec3::splat(-200.0), Vec3::splat(200.0))),
			))
			.id();
		app.update();
		assert_eq!(
			app.world()
				.resource::<GimmeLodHostIndex>()
				.hosts_in_region(around(Vec3::ZERO, 1.0)),
			vec![entity]
		);
	}
}
