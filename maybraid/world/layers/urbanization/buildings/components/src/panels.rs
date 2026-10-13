//! Shared panel geometry and tessellation (rectangles, right triangles).
//!
//! Panel space is **lower-left** anchored like roof pitch layouts:
//! - **X** along the long edge (length)
//! - **Z** depth / run (top/eave at \(Z = 0\), bottom/ridge at \(Z = -\texttt{depth}\))
//! - **Y** is kit thickness; domain nodes own extra orientation (roof pitch about \(+X\), wall stand-up, etc.)
//!
//! IR: [`PanelStyle`] + [`PanelGeometry`] + [`Placement`] → [`PanelNode`] (`LodScene`).
//!
//! LOD: High / Medium / Low keep the style triad; **UltraLow** always uses the shared
//! flat low-res kit ([`PANEL_ULTRA_LOW_RECTANGLE`] / [`PANEL_ULTRA_LOW_RIGHT_TRIANGLE`]).
//!
//! ```text
//! TessellatedTriangle.decompose() → Placed<RightTriangle>
//! PanelGeometry::flatten(caps)    → Placed<Rectangle | RightTriangle>
//! ```
//!
//! Crease / kink fillers are the separate [`crate::joints`] domain (`JointNode`), not panel geometry.
mod geometry;
mod kit_space;
mod lod;
mod node;
mod placement;
mod rough_stonework;
mod style;
mod tessellated_triangle;
mod triangle;

pub use geometry::{
	fitted_tile_count, PanelGeometry, PanelKitCaps, Rectangle, RightTriangle, DEFAULT_TILE_WIDTH,
};
pub use kit_space::{
	circle_inscribed_square_kit_hull, rectangle_kit_hull, right_triangle_kit_hull,
	tessellated_triangle_kit_hull, to_centered_rect_placement, with_wall_standup_pitch,
	INSCRIBED_SQUARE_HALF, PANEL_KIT_MAX, PANEL_KIT_MIN,
};
pub use lod::{
	panel_scene_ref_for_level, update_panel_host_levels, PanelLodBand, PanelLodProbe,
	PANEL_HIGH_FACTOR, PANEL_LOW_FACTOR, PANEL_MEDIUM_FACTOR, PANEL_ULTRA_LOW_RECTANGLE,
	PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
};
pub use node::PanelNode;
pub use placement::{roll_along_slope, yaw_along_xz, DEFAULT_MIN_JOINT_ANGLE};
pub use rough_stonework::RoughStonePanelRectangle;
pub use style::PanelStyle;
pub use tessellated_triangle::TessellatedTriangle;
pub use triangle::{dihedral_kink, triangle_normal};

pub(crate) use placement::wrap_pi;
