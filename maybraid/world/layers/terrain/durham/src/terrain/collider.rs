//! Terrain presentation collider wiring.
//!
//! Near (and FinePatch) fill scenes mark [`TerrainColliderMeshSource`]. After
//! Cached fulfill puts [`Mesh3d`] on that same entity, this module cooks an
//! Avian trimesh onto it. Far / background cells omit the marker.
//!
//! Constructed trimeshes use [`PhysicsInteractionLayer::Fixed`] so they contact
//! Animated movers only — not other Fixed geometry or LOD Host volumes.
//!
//! [`queue_terrain_trimesh_colliders`] honors [`TerrainColliderCookBudget`] so a
//! sprint-in wave of Near / FinePatch fills cannot cook every pending Avian
//! trimesh in one `Update` ([#1279](https://github.com/ramate-io/maybraid/issues/1279)).

use std::time::{Duration, Instant};

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

/// Per-frame cap for [`queue_terrain_trimesh_colliders`].
///
/// Near / FinePatch fills can become [`Mesh3d`]-ready in a wave (sprint into a
/// new column). Cooking every pending Avian trimesh in one `Update` was
/// 5–7.5 ms on golden-herd ([#1279](https://github.com/ramate-io/maybraid/issues/1279)).
/// LOD exclusive drain already stops at 2 ms
/// ([`lod::LodChunkFulfillBudget::spawn_time_per_frame`]); this matches that
/// clock after the first cook.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainColliderCookBudget {
	/// Wall-clock target checked after each cook. Default 2 ms.
	///
	/// The first ready source always cooks so a zeroed cap cannot stall
	/// collision. Remaining sources wait for the next `Update`.
	pub time_per_frame: Duration,
	/// Max successful cooks this frame. Default 1.
	pub cooks_per_frame: u32,
}

impl Default for TerrainColliderCookBudget {
	fn default() -> Self {
		Self { time_per_frame: Duration::from_millis(2), cooks_per_frame: 1 }
	}
}

impl TerrainColliderCookBudget {
	/// Successful cooks admitted this frame. At least one so a zeroed cap
	/// cannot stall collision.
	pub fn cooks_per_frame(self) -> u32 {
		self.cooks_per_frame.max(1)
	}
}

/// Cooks a trimesh onto each fill that asked for collision and now has [`Mesh3d`].
///
/// Stops after [`TerrainColliderCookBudget::cooks_per_frame`] successful cooks
/// or [`TerrainColliderCookBudget::time_per_frame`], whichever comes first.
/// Handles that are not in [`Assets<Mesh>`] yet are skipped without consuming
/// the cook slot.
pub(crate) fn queue_terrain_trimesh_colliders(
	mut commands: Commands,
	friction: Res<TerrainFrictionConfig>,
	budget: Res<TerrainColliderCookBudget>,
	meshes: Res<Assets<Mesh>>,
	sources: Query<
		(Entity, &Mesh3d),
		(With<TerrainColliderMeshSource>, Without<TerrainColliderReady>),
	>,
) {
	let start = Instant::now();
	let max_cooks = budget.cooks_per_frame();
	let mut cooked = 0u32;
	for (entity, mesh) in &sources {
		if cooked > 0 && (cooked >= max_cooks || start.elapsed() >= budget.time_per_frame) {
			break;
		}
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
		cooked += 1;
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn triangle_mesh() -> Mesh {
		let mut mesh = Mesh::new(
			bevy::mesh::PrimitiveTopology::TriangleList,
			bevy::asset::RenderAssetUsages::MAIN_WORLD,
		);
		mesh.insert_attribute(
			Mesh::ATTRIBUTE_POSITION,
			vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0]],
		);
		mesh.insert_indices(bevy::mesh::Indices::U32(vec![0, 1, 2]));
		mesh
	}

	fn cook_app(budget: TerrainColliderCookBudget) -> App {
		let mut app = App::new();
		app.insert_resource(Assets::<Mesh>::default())
			.insert_resource(TerrainFrictionConfig::default())
			.insert_resource(budget)
			.add_systems(Update, queue_terrain_trimesh_colliders);
		app
	}

	fn spawn_triangle_source(app: &mut App) -> Entity {
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(triangle_mesh());
		app.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(mesh), CascadeChunk::default()))
			.id()
	}

	fn cooked_count(app: &mut App) -> usize {
		app.world_mut()
			.query_filtered::<Entity, With<TerrainTrimeshCollider>>()
			.iter(app.world())
			.count()
	}

	#[test]
	fn default_terrain_friction_config_matches_const() {
		assert_eq!(TerrainFrictionConfig::default().0, TERRAIN_FRICTION);
	}

	#[test]
	fn default_cook_budget_is_one_cook_and_two_ms() {
		let budget = TerrainColliderCookBudget::default();
		assert_eq!(budget.cooks_per_frame(), 1);
		assert_eq!(budget.time_per_frame, Duration::from_millis(2));
	}

	#[test]
	fn zeroed_cook_cap_still_cooks_one() {
		assert_eq!(
			TerrainColliderCookBudget {
				cooks_per_frame: 0,
				..TerrainColliderCookBudget::default()
			}
			.cooks_per_frame(),
			1
		);
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
	fn trimesh_bakes_onto_the_fill_scene() -> anyhow::Result<()> {
		let mut app = cook_app(TerrainColliderCookBudget::default());
		let pose = Transform::from_xyz(2.0, 3.0, 4.0);
		let mesh = app.world_mut().resource_mut::<Assets<Mesh>>().add(triangle_mesh());
		let scene = app
			.world_mut()
			.spawn((TerrainColliderMeshSource, Mesh3d(mesh), pose, CascadeChunk::default()))
			.id();

		app.update();

		if app.world().get::<TerrainTrimeshCollider>(scene).is_none() {
			return Err(anyhow::anyhow!("expected TerrainTrimeshCollider after one update"));
		}
		if app.world().get::<Collider>(scene).is_none() {
			return Err(anyhow::anyhow!("expected Collider after one update"));
		}
		assert_eq!(app.world().get::<Transform>(scene).copied(), Some(pose));

		app.update();
		if app.world().get::<TerrainTrimeshCollider>(scene).is_none() {
			return Err(anyhow::anyhow!("collider should remain after a second update"));
		}
		Ok(())
	}

	#[test]
	fn pending_trimesh_cooks_spread_across_frames() -> anyhow::Result<()> {
		let mut app = cook_app(TerrainColliderCookBudget::default());
		let _scenes = [
			spawn_triangle_source(&mut app),
			spawn_triangle_source(&mut app),
			spawn_triangle_source(&mut app),
		];

		app.update();
		assert_eq!(cooked_count(&mut app), 1, "default budget cooks one ready source per frame");

		app.update();
		assert_eq!(cooked_count(&mut app), 2, "the next Update cooks the second source");

		app.update();
		assert_eq!(cooked_count(&mut app), 3, "the third Update finishes the leftover");
		Ok(())
	}

	#[test]
	fn raised_cook_cap_admits_two_sources_in_one_frame() -> anyhow::Result<()> {
		let mut app = cook_app(TerrainColliderCookBudget {
			cooks_per_frame: 2,
			..TerrainColliderCookBudget::default()
		});
		let _scenes = [
			spawn_triangle_source(&mut app),
			spawn_triangle_source(&mut app),
			spawn_triangle_source(&mut app),
		];

		app.update();
		assert_eq!(cooked_count(&mut app), 2, "a raised cap cooks two ready sources");
		app.update();
		assert_eq!(cooked_count(&mut app), 3, "the leftover cooks on the next frame");
		Ok(())
	}

	#[test]
	fn missing_mesh_asset_does_not_consume_the_cook_slot() -> anyhow::Result<()> {
		let mut app = cook_app(TerrainColliderCookBudget::default());
		let dangling = Handle::<Mesh>::default();
		app.world_mut().spawn((
			TerrainColliderMeshSource,
			Mesh3d(dangling),
			CascadeChunk::default(),
		));
		let ready = spawn_triangle_source(&mut app);

		app.update();
		assert_eq!(cooked_count(&mut app), 1, "the ready source still cooks this frame");
		if app.world().get::<TerrainTrimeshCollider>(ready).is_none() {
			return Err(anyhow::anyhow!("ready source should receive the cook slot"));
		}
		Ok(())
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
