//! Irregular and ruled panel constructions over [`PanelComplex`](panel_complex::PanelComplex).
//!
//! Oriented rectangle strips author each bay via lowest-edge + height + roll.
//! Fitted rectangles / strips best-fit ordinary kits to two-rail skew bays.
//! Free skew quads live in the ruled / clipped-quad types.

pub mod approximated_circle;
pub mod clipped_fitted_rectangular_strip;
pub mod clipped_quad_panel;
pub mod clipped_rectangular_strip;
pub mod clipped_ruled_strip;
pub mod clipped_tessellated_triangle;
pub mod fitted_rectangle;
pub mod fitted_rectangular_strip;
pub mod panel_complex;
pub mod panel_plane;
pub mod path_frame;
pub mod pillar;
pub mod quad_panel;
pub mod quad_panel_complex;
pub mod rect_crease;
pub mod rect_fit;
pub mod rectangle;
pub mod rectangular_n_tube;
pub mod rectangular_strip;
pub mod ruled_pitch;
pub mod ruled_strip;
pub mod tessellated_triangle_panel;
pub mod tube;
pub mod wall_strip;

pub use approximated_circle::{ApproximatedCircle, DEFAULT_SEGMENTS, MIN_SEGMENTS};
pub use clipped_fitted_rectangular_strip::{
	ClippedFittedRectangularStrip, ClippedFittedRectangularStripPiece,
};
pub use clipped_quad_panel::ClippedQuadPanel;
pub use clipped_rectangular_strip::{ClippedRectangularStrip, ClippedRectangularStripPiece};
pub use clipped_ruled_strip::{ClippedRuledStrip, ClippedStripPiece};
pub use clipped_tessellated_triangle::ClippedTessellatedTriangle;
pub use fitted_rectangle::{ClippedFittedRectangle, FittedRectangle};
pub use fitted_rectangular_strip::FittedRectangularStrip;
pub use panel_complex::{
	shared_edges, PanelComplex, PanelComplexJointPolicy, PanelComplexValidation, PanelMesh,
	PanelPoint, PanelPointId, PanelQuadMesh, PanelTriangle, ParsePanelComplexError, SharedEdge,
	DEFAULT_PANEL_THICKNESS,
};
pub use path_frame::TubeFrame;
pub use pillar::{PanelPillar, PanelPillarLine};
pub use quad_panel::QuadPanel;
pub use quad_panel_complex::QuadPanelComplex;
pub use rect_fit::{
	fallback_oriented, fit_rectangle, fit_rectangle_corners, orient_rectangle,
	roll_to_align_height, zero_roll_height_axis, FittedRect, OrientedRect, RectInset,
};
pub use rectangle::{ClippedRectangle, Rectangle};
pub use rectangular_n_tube::{RectangularNTube, RectangularNTubeCorner, RectangularNTubeStation};
pub use rectangular_strip::{RectangularStrip, RectangularStripNode};
pub use ruled_pitch::RuledPitch;
pub use ruled_strip::RuledStrip;
pub use tessellated_triangle_panel::TessellatedTrianglePanel;
pub use tube::{Tube, TubeCorners, TubeCrossSectionNode, TubeFaces};
