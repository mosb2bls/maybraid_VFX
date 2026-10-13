//! Fitted circular arc constructions → [`PartitionNode`]s.
//!
//! # Naming
//!
//! [`ArcSweep`] here is a **fitted construction** (center, radius, height, sweep).
//! It is distinct from the component IR `partitions::ArcSweep { sweep_degrees }`
//! (angle-only kit decomposition). Prefer `buildings::arcs::ArcSweep`
//! vs `Partition::arc(...)` when composing buildings.
//!
//! # Ellipses (deferred)
//!
//! Ellipsoidal sweeps are a **sibling** type (e.g. `EllipseSweep`), not folded into
//! [`ArcSweep`]. Circular path is kit-exact via [`decompose_arc_sweep`]; ellipses need
//! different sampling / assets.

pub mod clipped_sweep;
pub mod portal_ring;
pub mod sweep;

pub use clipped_sweep::ClippedArcSweep;
pub use portal_ring::{portal_ring_wall, PortalRingParams, PortalRingWall};
pub use sweep::ArcSweep;
