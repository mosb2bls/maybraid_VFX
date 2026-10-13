//! Cell face identity and coincidence / coverage helpers.

use bevy_math::{
	bounding::{Aabb2d, Aabb3d},
	Vec2,
};

/// Epsilon for face-coincidence and containment checks.
pub(crate) const FACE_EPS: f32 = 1e-4;

/// One face of a cell AABB.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FaceKind {
	Top,
	Bottom,
	Left,
	Right,
	Front,
	Back,
}

impl FaceKind {
	pub const ALL: [Self; 6] =
		[Self::Top, Self::Bottom, Self::Left, Self::Right, Self::Front, Self::Back];

	pub fn contains_aabb(parent: &Aabb3d, child: &Aabb3d) -> bool {
		child.min.x >= parent.min.x - FACE_EPS
			&& child.min.y >= parent.min.y - FACE_EPS
			&& child.min.z >= parent.min.z - FACE_EPS
			&& child.max.x <= parent.max.x + FACE_EPS
			&& child.max.y <= parent.max.y + FACE_EPS
			&& child.max.z <= parent.max.z + FACE_EPS
			&& child.min.x <= child.max.x
			&& child.min.y <= child.max.y
			&& child.min.z <= child.max.z
	}

	pub fn is_coincident(self, parent: &Aabb3d, child: &Aabb3d) -> bool {
		match self {
			Self::Top => (child.max.y - parent.max.y).abs() <= FACE_EPS,
			Self::Bottom => (child.min.y - parent.min.y).abs() <= FACE_EPS,
			Self::Left => (child.min.x - parent.min.x).abs() <= FACE_EPS,
			Self::Right => (child.max.x - parent.max.x).abs() <= FACE_EPS,
			Self::Front => (child.min.z - parent.min.z).abs() <= FACE_EPS,
			Self::Back => (child.max.z - parent.max.z).abs() <= FACE_EPS,
		}
	}

	/// Child's footprint on this face, in parent boundary-local \([0,1]^2\).
	pub fn child_coverage_in_parent_local(self, parent: &Aabb3d, child: &Aabb3d) -> Aabb2d {
		let (t0, t1, h0, h1) = match self {
			Self::Top | Self::Bottom => {
				let dx = (parent.max.x - parent.min.x).max(FACE_EPS);
				let dz = (parent.max.z - parent.min.z).max(FACE_EPS);
				(
					(child.min.x - parent.min.x) / dx,
					(child.max.x - parent.min.x) / dx,
					(child.min.z - parent.min.z) / dz,
					(child.max.z - parent.min.z) / dz,
				)
			}
			Self::Left | Self::Right => {
				let dz = (parent.max.z - parent.min.z).max(FACE_EPS);
				let dy = (parent.max.y - parent.min.y).max(FACE_EPS);
				(
					(child.min.z - parent.min.z) / dz,
					(child.max.z - parent.min.z) / dz,
					(child.min.y - parent.min.y) / dy,
					(child.max.y - parent.min.y) / dy,
				)
			}
			Self::Front | Self::Back => {
				let dx = (parent.max.x - parent.min.x).max(FACE_EPS);
				let dy = (parent.max.y - parent.min.y).max(FACE_EPS);
				(
					(child.min.x - parent.min.x) / dx,
					(child.max.x - parent.min.x) / dx,
					(child.min.y - parent.min.y) / dy,
					(child.max.y - parent.min.y) / dy,
				)
			}
		};
		Aabb2d {
			min: Vec2::new(t0.clamp(0.0, 1.0), h0.clamp(0.0, 1.0)),
			max: Vec2::new(t1.clamp(0.0, 1.0), h1.clamp(0.0, 1.0)),
		}
	}
}
