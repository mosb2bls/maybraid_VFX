//! Partition LOD re-exports.
//!
//! Posed content: [`crate::LodHostHelper`]. Partition GLB→level wiring: [`crate::partitions::host`].

pub use crate::partitions::geometry::{
	LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR,
};
pub use crate::partitions::mesh_set::{PartitionMeshSet, PartitionMeshTier};
pub use crate::partitions::probe::{
	update_partition_host_levels, PartitionLodBand, PartitionLodProbe,
};
