//! Full-height linear rough stonework partition (normalized \(X \in [-1, 1]\)).

use crate::assets::partitions::rough_stonework::{LINEAR_HIGH, LINEAR_LOW, LINEAR_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Linear wall segment for radial subdividers and straight partitions.
#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStoneworkLinear;

impl_partition_mesh_lod_scene!(
	RoughStoneworkLinear,
	PartitionMeshSet::new(LINEAR_HIGH, LINEAR_MID, LINEAR_LOW)
);
