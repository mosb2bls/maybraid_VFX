//! Core IR types for [`super::PanelComplex`].

use bevy_math::Vec3;
use building_components::panels::{PanelStyle, DEFAULT_MIN_JOINT_ANGLE};

use super::adjacency::canonical_edge;

/// Default world thickness matching unscaled panel kits (\(Y \in [-0.2, 0.2]\)).
pub const DEFAULT_PANEL_THICKNESS: f32 = 0.4;

/// Stable point handle inside a [`PanelComplex`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelPointId(pub u32);

impl PanelPointId {
	pub const fn new(id: u32) -> Self {
		Self(id)
	}

	pub const fn get(self) -> u32 {
		self.0
	}
}

/// World position + panel thickness at a mesh vertex.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelPoint {
	pub position: Vec3,
	pub thickness: f32,
}

impl PanelPoint {
	pub fn new(position: Vec3, thickness: f32) -> Self {
		Self { position, thickness: thickness.max(1e-4) }
	}

	/// Point with [`DEFAULT_PANEL_THICKNESS`].
	pub fn at(position: Vec3) -> Self {
		Self::new(position, DEFAULT_PANEL_THICKNESS)
	}
}

impl From<Vec3> for PanelPoint {
	fn from(position: Vec3) -> Self {
		Self::at(position)
	}
}

/// Ordered triangle of point ids (winding defines the normal).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelTriangle {
	pub a: PanelPointId,
	pub b: PanelPointId,
	pub c: PanelPointId,
}

impl PanelTriangle {
	pub fn new(a: PanelPointId, b: PanelPointId, c: PanelPointId) -> Self {
		Self { a, b, c }
	}

	pub fn vertices(self) -> [PanelPointId; 3] {
		[self.a, self.b, self.c]
	}

	/// Three undirected edges as canonical `(min, max)` keys.
	pub fn undirected_edges(self) -> Option<[(PanelPointId, PanelPointId); 3]> {
		let e0 = canonical_edge(self.a, self.b)?;
		let e1 = canonical_edge(self.b, self.c)?;
		let e2 = canonical_edge(self.c, self.a)?;
		Some([e0, e1, e2])
	}

	pub fn contains(self, id: PanelPointId) -> bool {
		self.a == id || self.b == id || self.c == id
	}

	/// Vertex opposite an undirected edge, if that edge is on this triangle.
	pub fn opposite(self, u: PanelPointId, v: PanelPointId) -> Option<PanelPointId> {
		let key = canonical_edge(u, v)?;
		for &(a, b, opp) in
			&[(self.a, self.b, self.c), (self.b, self.c, self.a), (self.c, self.a, self.b)]
		{
			if canonical_edge(a, b) == Some(key) {
				return Some(opp);
			}
		}
		None
	}
}

/// When to spawn a crease joint from the dihedral kink on a shared edge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelComplexJointPolicy {
	/// Spawn a joint when the dihedral kink (radians) is ≥ this threshold.
	pub min_dihedral_rad: f32,
}

impl Default for PanelComplexJointPolicy {
	fn default() -> Self {
		Self { min_dihedral_rad: DEFAULT_MIN_JOINT_ANGLE }
	}
}

impl PanelComplexJointPolicy {
	pub fn always() -> Self {
		Self { min_dihedral_rad: 0.0 }
	}

	pub fn never() -> Self {
		Self { min_dihedral_rad: f32::INFINITY }
	}

	pub fn min_dihedral_rad(min_dihedral_rad: f32) -> Self {
		Self { min_dihedral_rad: min_dihedral_rad.max(0.0) }
	}
}

/// Interior edge shared by exactly two triangles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SharedEdge {
	pub a: PanelPointId,
	pub b: PanelPointId,
	pub tri0: usize,
	pub tri1: usize,
}

impl SharedEdge {
	pub fn endpoints(self) -> (PanelPointId, PanelPointId) {
		(self.a, self.b)
	}
}

/// Issues found by [`PanelComplex::validate`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PanelComplexValidation {
	pub unknown_point_refs: Vec<(usize, PanelPointId)>,
	pub degenerate_triangles: Vec<usize>,
	pub non_manifold_edges: Vec<(PanelPointId, PanelPointId)>,
}

impl PanelComplexValidation {
	pub fn is_ok(&self) -> bool {
		self.unknown_point_refs.is_empty()
			&& self.degenerate_triangles.is_empty()
			&& self.non_manifold_edges.is_empty()
	}
}

/// Authored triangle complex → tessellated panels + optional crease joints.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelComplex {
	pub style: PanelStyle,
	/// Dense slot table; removed points become `None` (ids are not reused).
	pub(super) points: Vec<Option<PanelPoint>>,
	pub triangles: Vec<PanelTriangle>,
	pub joint_policy: PanelComplexJointPolicy,
}
