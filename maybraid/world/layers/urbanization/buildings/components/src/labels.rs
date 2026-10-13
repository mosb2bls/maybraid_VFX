//! Debug volume labels (colored wireframes; face strings via playground gizmos).
//!
//! IR: [`LabelStyle`] + [`LabelGeometry`] + text + [`Placement`] → [`LabelNode`].

pub mod geometry;
pub mod node;
pub mod style;
pub mod wireframe;

pub use geometry::LabelGeometry;
pub use node::LabelNode;
pub use style::LabelStyle;
pub use wireframe::{LabelWireframeAssets, LabelWireframePlugin};
