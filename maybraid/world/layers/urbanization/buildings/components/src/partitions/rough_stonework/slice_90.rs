//! Slice-height 90° rough stonework arc.

use crate::assets::partitions::rough_stonework::{SLICE_90_HIGH, SLICE_90_LOW, SLICE_90_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStoneworkSlice90;

impl_partition_mesh_lod_scene!(
	RoughStoneworkSlice90,
	PartitionMeshSet::new(SLICE_90_HIGH, SLICE_90_MID, SLICE_90_LOW)
);
