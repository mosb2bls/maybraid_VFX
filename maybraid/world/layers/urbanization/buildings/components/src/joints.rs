//! Joint scene components (circular / post fillers at creases and kinks).
//!
//! IR: [`JointStyle`] + [`JointGeometry`] + [`Placement`] → [`JointNode`] (`LodScene`).

pub mod geometry;
pub mod node;
pub mod rough_stonework;
pub mod style;

pub use geometry::{
	joint_xz_scale, Joint, JointGeometry, JointPost, JOINT_BASE_RADIUS, JOINT_KIT_HALF,
	JOINT_KIT_XZ, JOINT_RADIUS_PER_SLOPE_RAD,
};
pub use node::JointNode;
pub use rough_stonework::*;
pub use style::JointStyle;
