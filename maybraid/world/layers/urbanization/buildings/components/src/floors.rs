//! Floor slab scene components.
//!
//! IR: [`FloorStyle`] + [`FloorGeometry`] + [`Placement`] → [`FloorNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;
pub mod wood;

pub use geometry::*;
pub use node::FloorNode;
pub use rough_stonework::*;
pub use style::FloorStyle;
pub use wood::*;
