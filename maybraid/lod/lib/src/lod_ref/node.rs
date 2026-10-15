//! [`LodNode`] drivers that produce ephemeral [`super::LodRef`]s.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;

use super::LodRef;

/// Marker: this entity is an LOD driver (camera, probe, cascade track, …).
///
/// Pose history lives on [`LodNodePose`]. Region production still filters with `F`
/// (`With<Camera>` vs [`crate::LodViewer`]); produce- and cull-cache fill snapshot every node.
#[derive(Debug, Clone, Copy, Default, Component)]
#[require(LodNodePose)]
pub struct LodNode;

/// Driver extents for [`LodRef::bounds`].
///
/// If absent, the node is treated as pointlike at [`LodNodePose::current`] translation.
#[derive(Debug, Clone, Copy, Component)]
pub struct LodNodeBounds(pub Aabb3d);

/// Previous / current transform for a [`LodNode`] (for ephemeral [`LodRef`]s).
#[derive(Debug, Clone, Copy, Component)]
pub struct LodNodePose {
	pub previous: Transform,
	pub current: Transform,
}

impl Default for LodNodePose {
	fn default() -> Self {
		Self { previous: Transform::IDENTITY, current: Transform::IDENTITY }
	}
}

impl LodNodePose {
	/// Borrowed [`LodRef`] for this pose + driver `bounds`.
	pub fn as_lod_ref<'a>(&'a self, entity: Entity, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity,
			previous_transform: &self.previous,
			current_transform: &self.current,
			bounds,
		}
	}
}

/// Owned pose + bounds snapshot for building [`LodRef`]s inside a system.
#[derive(Debug, Clone, Copy)]
pub struct LodNodeSnapshot {
	pub entity: Entity,
	pub previous: Transform,
	pub current: Transform,
	/// Driver extents (from [`LodNodeBounds`], or a point at `current.translation`).
	pub bounds: Aabb3d,
}

impl LodNodeSnapshot {
	/// Borrowed [`LodRef`] for this snapshot.
	pub fn as_lod_ref(&self) -> LodRef<'_> {
		LodRef {
			entity: self.entity,
			previous_transform: &self.previous,
			current_transform: &self.current,
			bounds: &self.bounds,
		}
	}
}

/// Advance [`LodNodePose`] when the node's [`Transform`] changes.
///
/// `previous`/`current` are only updated on motion, so they may disagree while
/// the node is at rest. Region production keys off [`Changed<LodNodePose>`]
/// instead of requiring an every-frame collapse of that window.
pub fn track_lod_nodes(
	mut nodes: Query<(&Transform, &mut LodNodePose), (With<LodNode>, Changed<Transform>)>,
) {
	for (transform, mut pose) in &mut nodes {
		pose.previous = pose.current;
		pose.current = *transform;
	}
}

/// Pointlike driver AABB at `translation`.
pub fn point_bounds(translation: Vec3) -> Aabb3d {
	Aabb3d::from_min_max(translation, translation)
}

/// Collect node poses/bounds from a query of drivers into `out` (reuses capacity).
pub fn collect_node_snapshots_into<F: bevy::ecs::query::QueryFilter>(
	nodes: &Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), F>,
	out: &mut Vec<LodNodeSnapshot>,
) {
	out.clear();
	out.extend(nodes.iter().map(|(entity, pose, bounds)| LodNodeSnapshot {
		entity,
		previous: pose.previous,
		current: pose.current,
		bounds: bounds.map(|b| b.0).unwrap_or_else(|| point_bounds(pose.current.translation)),
	}));
}

/// Collect node poses/bounds from a query of drivers.
pub fn collect_node_snapshots<F: bevy::ecs::query::QueryFilter>(
	nodes: &Query<(Entity, &LodNodePose, Option<&LodNodeBounds>), F>,
) -> Vec<LodNodeSnapshot> {
	let mut out = Vec::new();
	collect_node_snapshots_into(nodes, &mut out);
	out
}

/// Build one [`LodRef`] per snapshot (driver pose + that driver's bounds).
pub fn lod_refs_from_snapshots<'a>(snapshots: &'a [LodNodeSnapshot]) -> Vec<LodRef<'a>> {
	snapshots.iter().map(LodNodeSnapshot::as_lod_ref).collect()
}

/// Shared driver tracking. Generate, present, and scene plugins add this once.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, SystemSet)]
pub enum LodNodeSystems {
	/// Advance [`LodNodePose`] from [`Transform`].
	Track,
}

/// Pose tracking only — not the scene refresh stack.
pub struct LodNodePlugin;

impl Plugin for LodNodePlugin {
	fn build(&self, app: &mut App) {
		app.configure_sets(Update, LodNodeSystems::Track)
			.add_systems(Update, track_lod_nodes.in_set(LodNodeSystems::Track));
	}
}
