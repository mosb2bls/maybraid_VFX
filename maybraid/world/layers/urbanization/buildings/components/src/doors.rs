//! Door scene components (frames composed from partition kits + leaves).
//!
//! IR: [`DoorStyle`] + [`DoorGeometry`] + [`Placement`] → [`DoorNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod style;
pub(crate) mod tessellate;
pub mod wood;

pub use geometry::*;
pub use node::DoorNode;
pub use style::DoorStyle;
pub use wood::*;
