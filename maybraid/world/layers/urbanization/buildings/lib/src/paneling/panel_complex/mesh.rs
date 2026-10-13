//! Structured triangle mesh IR (Rust form of the compact `points ... {a,b,c}` syntax).

use std::str::FromStr;

use building_components::panels::PanelStyle;

use super::parse::{parse_faces, parse_points, split_mesh_src, ParsePanelComplexError};
use super::types::{PanelComplex, PanelPoint, PanelPointId};

/// Explicit point ids + triangles — same information as the compact string form.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct PanelMesh {
	pub points: Vec<(PanelPointId, PanelPoint)>,
	pub triangles: Vec<(PanelPointId, PanelPointId, PanelPointId)>,
}

impl PanelMesh {
	pub fn new(
		points: Vec<(PanelPointId, PanelPoint)>,
		triangles: Vec<(PanelPointId, PanelPointId, PanelPointId)>,
	) -> Self {
		Self { points, triangles }
	}
}

impl FromStr for PanelMesh {
	type Err = ParsePanelComplexError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let (points_src, faces_src) = split_mesh_src(s)?;
		let points = parse_points(points_src)?;
		let triangles = parse_faces(faces_src, 3)?
			.into_iter()
			.map(|ids| (ids[0], ids[1], ids[2]))
			.collect();
		Ok(Self { points, triangles })
	}
}

impl PanelComplex {
	pub fn from_mesh(style: PanelStyle, mesh: PanelMesh) -> Self {
		let mut complex = Self::new(style);
		complex.apply_mesh(mesh);
		complex
	}

	pub fn apply_mesh(&mut self, mesh: PanelMesh) -> &mut Self {
		for (id, point) in mesh.points {
			self.put_point(id, point);
		}
		for (a, b, c) in mesh.triangles {
			self.add_triangle(a, b, c);
		}
		self
	}

	pub fn with_mesh(mut self, mesh: PanelMesh) -> Self {
		self.apply_mesh(mesh);
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_mesh_matches_from_str() {
		let s = "1=(0.5,0,0),2=(2.5,0,0),3=(0,0.3,3),4=(3,0,3) ... {1,2,4},{1,4,3}";
		let mesh: PanelMesh = s.parse().unwrap();
		let via_mesh = PanelComplex::from_mesh(PanelStyle::RoughStonework, mesh);
		let via_str: PanelComplex = s.parse().unwrap();
		assert_eq!(via_mesh.triangles().len(), via_str.triangles().len());
		assert_eq!(via_mesh.points().count(), via_str.points().count());
		assert_eq!(via_mesh.shared_edges().len(), 1);
	}
}
