//! Partition scene components (linear, angular, and slice variants).
//!
//! IR: [`PartitionStyle`] + [`PartitionGeometry`] + [`Placement`] → [`PartitionNode`] (`LodScene`).
//! [`PartitionNode`] covers both **direct** kit mappings and **tessellated** forms (polyline / arc).
//! Primitive kit geometry — higher-order walls/portals live in `buildings::{paneling,arcs,portals}`.
//!
//! Host split: crate [`crate::LodHostHelper`] (posed content) vs [`host`] (partition GLB resolution policy).

pub mod geometry;
pub mod host;
pub mod lod;
pub mod mesh_set;
pub mod node;
pub mod probe;
pub mod rough_stonework;
pub mod style;

pub use geometry::*;
pub use lod::{
	update_partition_host_levels, PartitionLodBand, PartitionLodProbe, PartitionMeshSet,
	PartitionMeshTier, LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR,
};
pub use node::PartitionNode;
pub use rough_stonework::*;
pub use style::PartitionStyle;
