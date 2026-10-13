//! Furniture / fixture scene components (placeholder wireframes).
//!
//! IR: [`FurnitureStyle`] + [`FurnitureGeometry`] + [`Placement`] + optional
//! [`FurnitureAbutment`] / finish seed → [`FurnitureNode`].

pub mod abutment;
pub mod geometry;
pub mod node;
pub mod style;
pub mod usage;
pub mod wireframe;

pub use abutment::FurnitureAbutment;
pub use geometry::FurnitureGeometry;
pub use node::{finish_seed_for, FurnitureNode};
pub use style::FurnitureStyle;
pub use usage::{finish_seed_for_usage, FurnitureUsage, FurnitureUsageNode};
pub use wireframe::{FurnitureWireframeAssets, FurnitureWireframePlugin};
