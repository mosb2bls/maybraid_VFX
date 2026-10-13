//! 180° angular rough stonework partition for circular outer walls.

use crate::assets::partitions::rough_stonework::{ARC_180_HIGH, ARC_180_LOW, ARC_180_MID};
use crate::partitions::lod::PartitionMeshSet;
use crate::partitions::node::impl_partition_mesh_lod_scene;

/// Half-ring wall sweep on local \(+X\) toward \(+Z\) (see [`crate::arc_ring_dir`]).
#[derive(Debug, Clone, Copy, PartialEq, Default, bevy::prelude::Component)]
pub struct RoughStonework180;

impl_partition_mesh_lod_scene!(
	RoughStonework180,
	PartitionMeshSet::new(ARC_180_HIGH, ARC_180_MID, ARC_180_LOW)
);
