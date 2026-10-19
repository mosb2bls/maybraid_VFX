//! Driver entities ([`LodNode`]) and ephemeral [`LodRef`] views.
//!
//! Not scene-specific: generation, presentation, and refresh all consume these.

mod node;

use bevy::prelude::*;
use lod_cascade::Aabb3d;

pub use node::{
	collect_node_snapshots, collect_node_snapshots_into, lod_refs_from_snapshots, point_bounds,
	track_lod_nodes, LodNode, LodNodeBounds, LodNodePlugin, LodNodePose, LodNodeSnapshot,
	LodNodeSystems,
};

/// A component type to mark fine LOD.
/// This enables archetype filtering to ignore a lot of entities.
#[derive(Debug, Component)]
pub struct FineLod;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LodRequest {
	/// Details with cells of the immediate neighborhood.
	Fine,
	/// Details outside the immediate neighborhood but that are still required for gameplay.
	Coarse,
	/// Details that are meant to be prepared for future use.
	Warm,
}

/// Borrowed view of a driver ([`LodNode`]) pose + that driver's extents.
#[derive(Debug, Clone, Copy)]
pub struct LodRef<'a> {
	/// The entity that triggered the LOD change.
	///
	/// For the most part, the developer will not use this.
	/// However, for more complex use cases without well-defined systems, this can be used to lookup the original entity
	/// and perform bespoke logic.
	pub entity: Entity,
	/// The previous transform of the entity that triggered the LOD change.
	pub previous_transform: &'a Transform,
	/// The transform of the entity that triggered the LOD change.
	pub current_transform: &'a Transform,
	/// Extents of the driver ([`LodNodeBounds`], or a point if the node is pointlike).
	///
	/// Not the host / scene AABB — host geometry is separate ([`crate::LodHostBounds`]).
	pub bounds: &'a Aabb3d,
}
