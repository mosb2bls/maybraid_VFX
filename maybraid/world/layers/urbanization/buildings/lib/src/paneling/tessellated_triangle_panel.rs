//! World-space triangle filled via shared 2D panel tessellation.
//!
//! Higher-order (buildings) feature — not a kit primitive. Pipeline:
//! 1. Orthonormal frame for the plane of \(A,B,C\) ([`crate::paneling::panel_plane`])
//! 2. Project corners into panel \(X,Z\)
//! 3. Encode plane as parent [`Placement`] (yaw / **pitch** / roll via YXZ)
//! 4. [`PanelNode`] fills in panel space; kit yaw is composed under that parent

use bevy_math::Vec3;
use building_components::panels::{PanelGeometry, PanelNode, PanelStyle, TessellatedTriangle};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;

use crate::paneling::panel_plane::panel_plane_frame;

/// Three world-space corners filled with posed panel right-triangle kits.
#[derive(Debug, Clone, PartialEq)]
pub struct TessellatedTrianglePanel {
	pub style: PanelStyle,
	pub a: Vec3,
	pub b: Vec3,
	pub c: Vec3,
}

impl TessellatedTrianglePanel {
	pub fn new(style: PanelStyle, a: Vec3, b: Vec3, c: Vec3) -> Self {
		Self { style, a, b, c }
	}

	pub fn rough_stone(a: Vec3, b: Vec3, c: Vec3) -> Self {
		Self::new(PanelStyle::RoughStonework, a, b, c)
	}

	pub fn shepherds_thatch(a: Vec3, b: Vec3, c: Vec3) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, a, b, c)
	}

	/// Panel-space triangle + parent [`Placement`] (plane yaw / pitch / roll).
	///
	/// Frame: origin at \(A\), \(+X\) along \(B-A\), \(+Y\) = unit normal
	/// \((B-A)\times(C-A)\), \(+Z\) completes the right-handed basis. Placement
	/// stores that frame as YXZ euler so pitch sits on the panel parent.
	///
	/// Returns [`None`] when the triangle is degenerate.
	pub fn panel_plane(&self) -> Option<(TessellatedTriangle, Placement)> {
		let frame = panel_plane_frame(self.a, self.b, self.c)?;
		Some((
			TessellatedTriangle::new(bevy_math::Vec2::ZERO, frame.b2, frame.c2),
			frame.placement(),
		))
	}

	/// [`PanelNode`] with plane placement, or [`None`] if degenerate.
	pub fn panel_node(&self) -> Option<PanelNode> {
		let (tri, placement) = self.panel_plane()?;
		Some(PanelNode::new(self.style, PanelGeometry::tessellated_triangle(tri), placement))
	}
}

impl BuildingComponents for TessellatedTrianglePanel {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::from_free(self.panel_node().into_iter().collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;
	use bevy_math::Vec2;
	use building_components::scene_children::pose;

	fn assert_vec3_close(got: Vec3, want: Vec3) {
		assert!((got - want).length() < 1e-4, "got {got:?} want {want:?}");
	}

	fn plane_transform(placement: Placement) -> Transform {
		pose(placement)
	}

	#[test]
	fn ground_triangle_projects_flat() {
		let panel = TessellatedTrianglePanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(3.0, 0.0, 0.0),
			Vec3::new(0.0, 0.0, 2.0),
		);
		let (tri, placement) = panel.panel_plane().expect("plane");
		assert!((tri.a - Vec2::ZERO).length() < 1e-4);
		assert!((tri.b - Vec2::new(3.0, 0.0)).length() < 1e-4);
		assert!(placement.pitch.abs() < 1e-3);

		let plane = plane_transform(placement);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.a.x, 0.0, tri.a.y)), panel.a);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), panel.b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), panel.c);
	}

	#[test]
	fn tilted_triangle_has_pitch_on_parent_and_round_trips() {
		let panel = TessellatedTrianglePanel::rough_stone(
			Vec3::new(1.0, 2.0, 3.0),
			Vec3::new(4.0, 2.5, 3.0),
			Vec3::new(1.5, 5.0, 6.0),
		);
		let (tri, placement) = panel.panel_plane().expect("plane");
		assert!(
			placement.pitch.abs() > 1e-3 || placement.roll.abs() > 1e-3,
			"expected non-flat parent euler, got yaw={} pitch={} roll={}",
			placement.yaw,
			placement.pitch,
			placement.roll
		);
		let plane = plane_transform(placement);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.a.x, 0.0, tri.a.y)), panel.a);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), panel.b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), panel.c);
	}

	#[test]
	fn degenerate_is_none() {
		assert!(TessellatedTrianglePanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(1.0, 0.0, 0.0),
			Vec3::new(2.0, 0.0, 0.0),
		)
		.panel_plane()
		.is_none());
	}

	#[test]
	fn vertical_wall_triangle_has_nonzero_pitch() {
		let panel = TessellatedTrianglePanel::rough_stone(
			Vec3::ZERO,
			Vec3::new(2.0, 0.0, 0.0),
			Vec3::new(0.0, 3.0, 0.0),
		);
		let (tri, placement) = panel.panel_plane().expect("plane");
		assert!(
			placement.pitch.abs() > 1.0,
			"expected large pitch for vertical face, got {}",
			placement.pitch
		);
		let plane = plane_transform(placement);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.b.x, 0.0, tri.b.y)), panel.b);
		assert_vec3_close(plane.transform_point(Vec3::new(tri.c.x, 0.0, tri.c.y)), panel.c);
		let n = plane.rotation * Vec3::Y;
		assert!(n.z.abs() > 0.9, "expected wall normal along ±Z, got {n:?}");
	}
}
