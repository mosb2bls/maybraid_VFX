//! Stair scene components (spiral and straight runs).
//!
//! IR: [`StairStyle`] + [`StairGeometry`] + [`Placement`] → [`StairNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;
pub(crate) mod tessellate;
pub mod wood;

pub use geometry::*;
pub use node::StairNode;
pub use rough_stonework::*;
pub use style::StairStyle;
pub use wood::*;
