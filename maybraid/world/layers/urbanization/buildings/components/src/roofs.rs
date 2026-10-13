//! Roof / cap scene components.
//!
//! IR: [`RoofStyle`] + [`RoofGeometry`] + [`Placement`] → [`RoofNode`] (`LodScene`).

pub mod geometry;
pub mod lod;
pub mod node;
pub mod shepherds_thatch;
pub mod style;
pub(crate) mod tessellate;

pub use geometry::*;
pub use lod::{
	roof_scene_ref_for_level, update_roof_host_levels, RoofLodBand, RoofLodProbe, ROOF_HIGH_FACTOR,
	ROOF_LOW_FACTOR, ROOF_MEDIUM_FACTOR,
};
pub use node::RoofNode;
pub use shepherds_thatch::*;
pub use style::RoofStyle;
