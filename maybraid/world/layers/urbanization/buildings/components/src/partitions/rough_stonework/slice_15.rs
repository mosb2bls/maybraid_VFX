//! Slice-height 15° rough stonework arc for curved door frames.

use crate::assets::partitions::rough_stonework::{SLICE_15_HIGH, SLICE_15_LOW, SLICE_15_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStoneworkSlice15;

impl_partition_mesh_lod_scene!(
	RoughStoneworkSlice15,
	PartitionMeshSet::new(SLICE_15_HIGH, SLICE_15_MID, SLICE_15_LOW)
);
