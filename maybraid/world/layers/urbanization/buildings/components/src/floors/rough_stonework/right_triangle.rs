//! Rough-stonework unit right-triangle panel kit (LOD triad).

use bevy::prelude::{Component, Transform};
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::LodSceneLevel;
use scene_ref::MirrorAxis;

use crate::assets::panels::rough_stonework::{
	RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
};
use crate::partitions::geometry::LinearLod;
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;
use crate::partitions::probe::PartitionLodProbe;

/// Unit right triangle \(X \in [0, 1]\), \(Z \in [-1, 0]\), \(Y \in [-0.2, 0.2]\).
#[derive(Debug, Clone, Copy, PartialEq, Default, Component)]
pub struct RoughStoneFloorRightTriangle;

impl_partition_mesh_lod_scene!(
	RoughStoneFloorRightTriangle,
	PartitionMeshSet::new(RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_MID, RIGHT_TRIANGLE_LOW)
);

impl RoughStoneFloorRightTriangle {
	/// Posed triad content with optional axis mirroring (no host scaffolding).
	pub fn scene_for_level_mirrored(
		level: LodSceneLevel,
		mirror: Option<MirrorAxis>,
	) -> impl Scene + 'static {
		LinearLod::posed_mirrored_tier(
			PartitionMeshSet::new(RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_MID, RIGHT_TRIANGLE_LOW),
			Transform::IDENTITY,
			level,
			mirror,
		)
	}

	/// Level from [`LodRef`] bounds, then mirrored posed content.
	pub fn scene_with_lod_mirrored(
		lod_ref: &LodRef,
		mirror: Option<MirrorAxis>,
	) -> impl Scene + 'static {
		let level =
			PartitionLodProbe::from_aabb(lod_ref.bounds).level_for(lod_ref.current_transform);
		Self::scene_for_level_mirrored(level, mirror)
	}
}
