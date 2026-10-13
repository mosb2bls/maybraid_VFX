//! 90° angular rough stonework partition for circular outer walls.

use crate::assets::partitions::rough_stonework::{ARC_90_HIGH, ARC_90_LOW, ARC_90_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Quarter-ring wall sweep on local \(+X\) toward \(+Z\) (see [`crate::arc_ring_dir`]).
#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStonework90;

impl_partition_mesh_lod_scene!(
	RoughStonework90,
	PartitionMeshSet::new(ARC_90_HIGH, ARC_90_MID, ARC_90_LOW)
);
