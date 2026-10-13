//! Lookups, validation, panels, and crease joints.

use bevy_math::Vec3;
use building_components::joints::{JointNode, JointPost};
use building_components::panels::{dihedral_kink, triangle_normal};

use crate::paneling::tessellated_triangle_panel::TessellatedTrianglePanel;

use super::adjacency::shared_edges;
use super::types::{
	PanelComplex, PanelComplexValidation, PanelPoint, PanelPointId, PanelTriangle, SharedEdge,
};

impl PanelComplex {
	pub fn point(&self, id: PanelPointId) -> Option<&PanelPoint> {
		self.points.get(id.0 as usize).and_then(|p| p.as_ref())
	}

	pub fn point_mut(&mut self, id: PanelPointId) -> Option<&mut PanelPoint> {
		self.points.get_mut(id.0 as usize).and_then(|p| p.as_mut())
	}

	/// Present points as `(id, point)` in id order.
	pub fn points(&self) -> impl Iterator<Item = (PanelPointId, &PanelPoint)> + '_ {
		self.points
			.iter()
			.enumerate()
			.filter_map(|(i, p)| p.as_ref().map(|pt| (PanelPointId(i as u32), pt)))
	}

	pub fn triangles(&self) -> &[PanelTriangle] {
		&self.triangles
	}

	/// Manifold shared edges only (non-manifold omitted).
	pub fn shared_edges(&self) -> Vec<SharedEdge> {
		shared_edges(&self.triangles).0
	}

	pub fn validate(&self) -> PanelComplexValidation {
		let mut report = PanelComplexValidation::default();
		for (i, tri) in self.triangles.iter().enumerate() {
			for id in tri.vertices() {
				if self.point(id).is_none() {
					report.unknown_point_refs.push((i, id));
				}
			}
			if tri.undirected_edges().is_none() {
				report.degenerate_triangles.push(i);
				continue;
			}
			if let (Some(pa), Some(pb), Some(pc)) =
				(self.point(tri.a), self.point(tri.b), self.point(tri.c))
			{
				if triangle_normal(pa.position, pb.position, pc.position).is_none() {
					report.degenerate_triangles.push(i);
				}
			}
		}
		report.non_manifold_edges = shared_edges(&self.triangles).1;
		report
	}

	pub fn triangle_panel(&self, tri: PanelTriangle) -> Option<TessellatedTrianglePanel> {
		let a = self.point(tri.a)?.position;
		let b = self.point(tri.b)?.position;
		let c = self.point(tri.c)?.position;
		Some(TessellatedTrianglePanel::new(self.style, a, b, c))
	}

	pub fn triangle_panels(&self) -> Vec<TessellatedTrianglePanel> {
		self.triangles.iter().filter_map(|t| self.triangle_panel(*t)).collect()
	}

	/// Average thickness of the two endpoints of a shared edge.
	pub fn edge_thickness(&self, a: PanelPointId, b: PanelPointId) -> Option<f32> {
		let ta = self.point(a)?.thickness;
		let tb = self.point(b)?.thickness;
		Some(0.5 * (ta + tb))
	}

	fn triangle_normal_at(&self, tri_idx: usize) -> Option<Vec3> {
		let tri = self.triangles.get(tri_idx)?;
		let a = self.point(tri.a)?.position;
		let b = self.point(tri.b)?.position;
		let c = self.point(tri.c)?.position;
		triangle_normal(a, b, c)
	}

	/// Dihedral kink (radians) between the two faces of a shared edge.
	pub fn dihedral_kink(&self, edge: SharedEdge) -> Option<f32> {
		let n0 = self.triangle_normal_at(edge.tri0)?;
		let n1 = self.triangle_normal_at(edge.tri1)?;
		Some(dihedral_kink(n0, n1))
	}

	/// Crease [`JointNode`]s for shared edges that meet the joint policy.
	///
	/// Non-manifold edges are skipped (see [`Self::validate`]). Presentation
	/// stays infallible.
	pub fn joint_nodes(&self) -> Vec<JointNode> {
		let mut out = Vec::new();
		for edge in self.shared_edges() {
			let Some(kink) = self.dihedral_kink(edge) else {
				continue;
			};
			if kink < self.joint_policy.min_dihedral_rad {
				continue;
			}
			let Some(thickness) = self.edge_thickness(edge.a, edge.b) else {
				continue;
			};
			let Some(n0) = self.triangle_normal_at(edge.tri0) else {
				continue;
			};
			let Some(n1) = self.triangle_normal_at(edge.tri1) else {
				continue;
			};
			let Some(pa) = self.point(edge.a) else {
				continue;
			};
			let Some(pb) = self.point(edge.b) else {
				continue;
			};
			let radial_hint = n0 + n1;
			let Some(placement) =
				JointPost::placed_along_crease(pa.position, pb.position, thickness, radial_hint)
			else {
				continue;
			};
			out.push(JointNode::rough_stone_post(placement));
		}
		out
	}
}
