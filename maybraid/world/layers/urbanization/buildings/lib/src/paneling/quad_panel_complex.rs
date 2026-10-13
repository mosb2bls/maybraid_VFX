//! Construction wrapper for quad-face panel meshes (separate [`FromStr`] for playground).
//!
//! ```text
//! 1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}
//! ```
//!
//! Quads are `{a0, a1, b0, b1}` with diagonal \(a_0\)–\(b_1\). Helpers live on
//! [`PanelComplex`](crate::paneling::panel_complex::PanelComplex); this type only owns a
//! complex built from a [`PanelQuadMesh`](crate::paneling::panel_complex::PanelQuadMesh).

use std::str::FromStr;

use building_components::panels::PanelStyle;

use crate::paneling::panel_complex::{
	PanelComplex, PanelComplexJointPolicy, PanelQuadMesh, ParsePanelComplexError,
};

/// Thin wrapper around a [`PanelComplex`] authored as a quad mesh.
#[derive(Debug, Clone, PartialEq)]
pub struct QuadPanelComplex(PanelComplex);

impl QuadPanelComplex {
	pub fn new(style: PanelStyle, mesh: PanelQuadMesh) -> Self {
		Self(PanelComplex::from_quad_mesh(style, mesh))
	}

	pub fn rough_stone(mesh: PanelQuadMesh) -> Self {
		Self::new(PanelStyle::RoughStonework, mesh)
	}

	pub fn shepherds_thatch(mesh: PanelQuadMesh) -> Self {
		Self::new(PanelStyle::ShepherdsThatch, mesh)
	}

	pub fn with_joint_policy(mut self, joint_policy: PanelComplexJointPolicy) -> Self {
		self.0 = self.0.with_joint_policy(joint_policy);
		self
	}

	pub fn into_complex(self) -> PanelComplex {
		self.0
	}

	pub fn as_complex(&self) -> &PanelComplex {
		&self.0
	}
}

impl AsRef<PanelComplex> for QuadPanelComplex {
	fn as_ref(&self) -> &PanelComplex {
		&self.0
	}
}

impl From<QuadPanelComplex> for PanelComplex {
	fn from(value: QuadPanelComplex) -> Self {
		value.into_complex()
	}
}

impl FromStr for QuadPanelComplex {
	type Err = ParsePanelComplexError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		let mesh: PanelQuadMesh = s.parse()?;
		Ok(Self::rough_stone(mesh))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;

	#[test]
	fn from_str_into_complex_escape_hatch() {
		let q: QuadPanelComplex =
			"1=(0,0,0),2=(1,0,0),3=(0,1,0),4=(0,0,1) ... {1,2,3,4}".parse().expect("parse");
		let mut c = q.into_complex();
		assert_eq!(c.triangles().len(), 2);
		// Escape hatch: add another triangle after unwrap.
		let a = c.insert_point(bevy_math::Vec3::new(2.0, 0.0, 0.0));
		let b = c.insert_point(bevy_math::Vec3::new(2.0, 1.0, 0.0));
		let d = c.insert_point(bevy_math::Vec3::new(2.0, 0.0, 1.0));
		c.add_triangle(a, b, d);
		assert_eq!(c.panel_nodes_for_level(LodSceneLevel::High).flatten().len(), 3);
	}
}
