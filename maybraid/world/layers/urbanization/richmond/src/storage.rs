//! Richmond's nodes in [`HcsgStorage`].
//!
//! Generation reads stored values through [`GenerationContext`] on each
//! scheme's [`lod::hcsg::GenerationScheme::build_with_id`].

use bevy::math::bounding::Aabb3d;
use bevy::math::{DVec3, Vec3};

use crate::cell::DEVELOPMENT_CELL_SIZE;

/// Half height of development columns. Development cells are flat XZ
/// tiles, so their stored bounds span every terrain elevation.
const COLUMN_HALF_HEIGHT: f32 = 100_000.0;

/// `cell` extended over every elevation, so storage overlap is XZ-only.
pub fn column_bounds(cell: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(
		Vec3::new(cell.min.x, -COLUMN_HALF_HEIGHT, cell.min.z),
		Vec3::new(cell.max.x, COLUMN_HALF_HEIGHT, cell.max.z),
	)
}

/// Inclusive XZ overlap.
pub(crate) fn overlaps_xz(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x <= b.max.x && a.max.x >= b.min.x && a.min.z <= b.max.z && a.max.z >= b.min.z
}

/// Strict XZ overlap: cells that only share an edge do not overlap.
pub(crate) fn overlaps_xz_strictly(a: Aabb3d, b: Aabb3d) -> bool {
	a.min.x < b.max.x && a.max.x > b.min.x && a.min.z < b.max.z && a.max.z > b.min.z
}

/// Index scale of development columns: one development cell per bucket.
pub(crate) const COLUMN_INDEX_SCALE: DVec3 = DVec3::new(
	DEVELOPMENT_CELL_SIZE as f64,
	2.0 * COLUMN_HALF_HEIGHT as f64,
	DEVELOPMENT_CELL_SIZE as f64,
);
