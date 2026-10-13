//! Engine-agnostic downward elevation samples.
//!
//! Parallel to [`lod::LodSceneRegionIndex`]: this crate answers “what is under
//! this point?”, not “which hosts overlap this AABB?”. How colliders entered
//! the world (authored terrain, generated meshes, props) is out of scope.
//!
//! Backends implement [`ElevationProbe`]. Character tilt (`character-motion`)
//! is generic over that trait and must not import Durham or Avian types.

pub mod probe;

pub use probe::{ElevationProbe, GroundHit};
