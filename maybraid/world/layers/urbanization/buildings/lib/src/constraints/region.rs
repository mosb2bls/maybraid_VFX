//! Boundary-local region lists.

use bevy_math::{bounding::Aabb2d, Vec2};

use crate::constraints::face::FACE_EPS;

/// A boundary sub-region paired with a value.
///
/// Region [`Aabb2d`] values are **boundary-local**:
/// - \(x\) (`t`) runs along the face's primary length in \([0, 1]\)
/// - \(y\) (`h`) runs along the face's secondary axis in \([0, 1]\)
///   (vertical for side faces; the other horizontal axis for top/bottom)
pub type BoundaryRegionList<T> = Vec<(Aabb2d, T)>;

pub trait BoundaryRegionListExt<T>: Sized {
	fn clip_to_coverage(&self, coverage: Aabb2d) -> Self
	where
		T: Clone;
}

impl<T: Clone> BoundaryRegionListExt<T> for BoundaryRegionList<T> {
	fn clip_to_coverage(&self, coverage: Aabb2d) -> Self {
		self.iter()
			.filter_map(|(region, value)| {
				remap_to_child_local(*region, coverage).map(|local| (local, value.clone()))
			})
			.collect()
	}
}

pub fn intersect_aabb2d(a: Aabb2d, b: Aabb2d) -> Option<Aabb2d> {
	let min = Vec2::new(a.min.x.max(b.min.x), a.min.y.max(b.min.y));
	let max = Vec2::new(a.max.x.min(b.max.x), a.max.y.min(b.max.y));
	if min.x <= max.x + FACE_EPS && min.y <= max.y + FACE_EPS {
		Some(Aabb2d { min, max })
	} else {
		None
	}
}

/// Remap a region from parent-local into child-local given the child's coverage on the parent face.
pub fn remap_to_child_local(region: Aabb2d, coverage: Aabb2d) -> Option<Aabb2d> {
	let clipped = intersect_aabb2d(region, coverage)?;
	let size = coverage.max - coverage.min;
	let sx = size.x.max(FACE_EPS);
	let sy = size.y.max(FACE_EPS);
	Some(Aabb2d {
		min: Vec2::new(
			((clipped.min.x - coverage.min.x) / sx).clamp(0.0, 1.0),
			((clipped.min.y - coverage.min.y) / sy).clamp(0.0, 1.0),
		),
		max: Vec2::new(
			((clipped.max.x - coverage.min.x) / sx).clamp(0.0, 1.0),
			((clipped.max.y - coverage.min.y) / sy).clamp(0.0, 1.0),
		),
	})
}
