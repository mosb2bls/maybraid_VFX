//! 15° angular rough stonework partition for curved door/window framing.

use crate::assets::partitions::rough_stonework::{ARC_15_HIGH, ARC_15_LOW, ARC_15_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Narrow arc sweep on local \(+X\) toward \(+Z\) (see [`crate::arc_ring_dir`]).
#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStonework15;

impl_partition_mesh_lod_scene!(
	RoughStonework15,
	PartitionMeshSet::new(ARC_15_HIGH, ARC_15_MID, ARC_15_LOW)
);
