//! Terrain presentation collider wiring.
//!
//! Near (and FinePatch) fill scenes mark [`TerrainColliderMeshSource`]. After
//! Cached fulfill puts [`Mesh3d`] on that same entity, this module cooks an
//! Avian trimesh onto it. Far / background cells omit the marker.
//!
//! Constructed trimeshes use [`PhysicsInteractionLayer::Fixed`] so they contact
//! Animated movers only — not other Fixed geometry or LOD Host volumes.

use avian3d::prelude::{CoefficientCombine, Collider, Friction, RigidBody};
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use lod_avian::PhysicsInteractionLayer;

/// Dirt / grass grip. [`CoefficientCombine::Max`] beats the character controller's
/// `Friction::ZERO` + `Min` (Avian dynamic-character default), otherwise the
/// capsule ice-skates on the trimesh.
///
/// Playgrounds override via [`TerrainFrictionConfig`] (inserted before Durham
/// generation); [`queue_terrain_trimesh_colliders`] reads
/// that resource, not this constant, when both exist.
pub const TERRAIN_FRICTION: Friction = Friction {
	dynamic_coefficient: 0.75,
	static_coefficient: 0.95,
	combine_rule: CoefficientCombine::Max,
};

/// Friction applied to new terrain trimeshes. Defaults to [`TERRAIN_FRICTION`].
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct TerrainFrictionConfig(pub Friction);

impl Default for TerrainFrictionConfig {
	fn default() -> Self {
		Self(TERRAIN_FRICTION)
	}
}

/// Presenters run before [`Self::QueueMeshes`] so a new fill can bake the same frame.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TerrainColliderSystems {
	SyncOverlays,
	SyncHosts,
	QueueMeshes,
}

/// Fill scene that should receive a trimesh once [`Mesh3d`] is fulfilled.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainColliderMeshSource;

/// Cooked Avian trimesh on a Near / FinePatch fill scene.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct TerrainTrimeshCollider;

/// True when any collider column covers `point` on XZ (Y is ignored).
pub fn terrain_collider_covers_xz<'a>(
	point: Vec3,
	chunks: impl IntoIterator<Item = &'a CascadeChunk>,
) -> bool {
	chunks.into_iter().any(|chunk| chunk.column_contains_point(point))
}

#[derive(Component, Debug, Clone, Copy, Default)]
pub(crate) struct TerrainColliderReady;

/// Cooks a trimesh onto each fill that asked for collision and now has [`Mesh3d`].
pub(crate) fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	meshes: Res<Assets<Mesh>>,
	sources: Query<
		(Entity, &Mesh3d),
		(With<TerrainColliderMeshSource>, Without<TerrainColliderReady>),
	>,
) {
	for (entity, mesh) in &sources {
		let Some(mesh) = meshes.get(&mesh.0) else {
			continue;
		};
		let Some(collider) = Collider::trimesh_from_mesh(mesh) else {
			continue;
		};
		commands.entity(entity).insert((
			TerrainTrimeshCollider,
			TerrainColliderReady,
			RigidBody::Static,
			collider,
			PhysicsInteractionLayer::fixed_layers(),
			friction.0,
		));
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_terrain_friction_config_matches_const() {
		assert_eq!(TerrainFrictionConfig::default().0, TERRAIN_FRICTION);
	}

	#[test]
	fn distant_collider_does_not_cover_spawn_column() {
		let spawn = Vec3::ZERO;
		let local = CascadeChunk::unit_chunk();
		let distant = CascadeChunk {
			origin: Vec3::new(1_000.0, -2_000.0, 1_000.0),
			size: 160.0,
			..CascadeChunk::unit_chunk()
		};
		assert!(terrain_collider_covers_xz(spawn, [&local]));
		assert!(!terrain_collider_covers_xz(spawn, [&distant]));
		assert!(terrain_collider_covers_xz(spawn, [&distant, &local]));
	}

	#[test]
	fn trimesh_bakes_onto_the_fill_scene() {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.add_systems(Update, queue_terrain_trimesh_colliders);

		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::MAIN_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(mesh);

		let pose = Transform::from_xyz(2.0, 3.0, 4.0);
		let scene = app
			.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(mesh), pose, CascadeChunk::default()))
			.id();

		app.update();

		assert!(app.world().get::<TerrainTrimeshCollider>(scene).is_some());
		assert!(app.world().get::<Collider>(scene).is_some());
		assert_eq!(app.world().get::<Transform>(scene).copied(), Some(pose));

		app.update();
		assert!(app.world().get::<TerrainTrimeshCollider>(scene).is_some());
	}

	#[test]
	fn despawn_of_the_fill_scene_removes_its_collider() {
		let mut app = App::new();
		let scene = app
			.world_mut()
			.spawn((TerrainTrimeshCollider, TerrainColliderMeshSource, CascadeChunk::unit_chunk()))
			.id();

		app.world_mut().entity_mut(scene).despawn();

		let mut colliders =
			app.world_mut().query_filtered::<Entity, With<TerrainTrimeshCollider>>();
		assert_eq!(colliders.iter(app.world()).count(), 0);
	}
}
