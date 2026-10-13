//! Quad faces on [`PanelComplex`]: diagonal \(a_0\)–\(b_1\) for `{a0, a1, b0, b1}`.
//!
//! Compact string (via [`PanelQuadMesh`] / [`crate::paneling::quad_panel_complex::QuadPanelComplex`]):
//! ```text
//! 1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}
//! ```

use std::str::FromStr;

use building_components::panels::PanelStyle;

use super::parse::{parse_faces, parse_points, split_mesh_src, ParsePanelComplexError};
use super::types::{PanelComplex, PanelPoint, PanelPointId};

/// Explicit point ids + quads `[a0, a1, b0, b1]` (diagonal `a0`–`b1`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelQuadMesh {
	pub points: Vec<(PanelPointId, PanelPoint)>,
	pub quads: Vec<[PanelPointId; 4]>,
}

impl PanelQuadMesh {
	pub fn new(points: Vec<(PanelPointId, PanelPoint)>, quads: Vec<[PanelPointId; 4]>) -> Self {
		Self { points, quads }
	}

	/// Expand to triangles: `(a0,a1,b1)` and `(a0,b1,b0)`.
	pub fn to_triangle_mesh(&self) -> super::mesh::PanelMesh {
		let mut triangles = Vec::with_capacity(self.quads.len() * 2);
		for &[a0, a1, b0, b1] in &self.quads {
			triangles.push((a0, a1, b1));
			triangles.push((a0, b1, b0));
		}
		super::mesh::PanelMesh { points: self.points.clone(), triangles }
	}
}

impl FromStr for PanelQuadMesh {
	type Err = ParsePanelComplexError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (points_src, faces_src) = split_mesh_src(s)?;
		let points = parse_points(points_src)?;
		let quads = parse_faces(faces_src, 4)?
			.into_iter()
			.map(|ids| [ids[0], ids[1], ids[2], ids[3]])
			.collect();
		Ok(Self { points, quads })
	}
}

impl PanelComplex {
	/// Append two tris for quad `(a0, a1, b0, b1)`; diagonal \(a_0\)–\(b_1\).
	pub fn add_quad(
		&mut self,
		a0: PanelPointId,
		a1: PanelPointId,
		b0: PanelPointId,
		b1: PanelPointId,
	) -> &mut Self {
		self.add_triangle(a0, a1, b1).add_triangle(a0, b1, b0)
	}

	pub fn with_quad(
		mut self,
		a0: PanelPointId,
		a1: PanelPointId,
		b0: PanelPointId,
		b1: PanelPointId,
	) -> Self {
		self.add_quad(a0, a1, b0, b1);
		self
	}

	pub fn from_quad_mesh(style: PanelStyle, mesh: PanelQuadMesh) -> Self {
		let mut complex = Self::new(style);
		complex.apply_quad_mesh(mesh);
		complex
	}

	pub fn apply_quad_mesh(&mut self, mesh: PanelQuadMesh) -> &mut Self {
		for (id, point) in mesh.points {
			self.put_point(id, point);
		}
		for [a0, a1, b0, b1] in mesh.quads {
			self.add_quad(a0, a1, b0, b1);
		}
		self
	}

	pub fn with_quad_mesh(mut self, mesh: PanelQuadMesh) -> Self {
		self.apply_quad_mesh(mesh);
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;

	#[test]
	fn add_quad_shares_diagonal() {
		let mut c = PanelComplex::rough_stone();
		let a0 = c.insert_point(Vec3::ZERO);
		let a1 = c.insert_point(Vec3::new(1.0, 0.0, 0.0));
		let b0 = c.insert_point(Vec3::new(0.0, 1.0, 0.0));
		let b1 = c.insert_point(Vec3::new(0.0, 0.0, 1.0));
		c.add_quad(a0, a1, b0, b1);
		assert_eq!(c.triangles().len(), 2);
		let shared = c.shared_edges();
		assert_eq!(shared.len(), 1);
		let (u, v) = shared[0].endpoints();
		assert!(
			(u, v) == (a0, b1) || (u, v) == (b1, a0),
			"expected diagonal a0–b1, got {u:?}–{v:?}"
		);
	}

	#[test]
	fn quad_mesh_string_parses() {
		let mesh: PanelQuadMesh =
			"1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}".parse().expect("parse");
		assert_eq!(mesh.quads.len(), 1);
		let c = PanelComplex::from_quad_mesh(PanelStyle::RoughStonework, mesh);
		assert_eq!(c.triangles().len(), 2);
		assert_eq!(c.shared_edges().len(), 1);
	}
}
