//! Grove LOD footprint ([RFC-170 §3.1.3], [RFC-183 §3.4.2.3]).

use bevy_math::bounding::Aabb3d;
use bevy_math::{Vec2, Vec3};
use gimme_gen::Cell;

/// Default square grove preview / isolation-render footprint in metres on X and Z.
pub const DEFAULT_GROVE_EXTENT_XZ: f32 = 100.0;

/// Axis-aligned grove LOD unit in world space (first-order cell \(C\) in [RFC-170 §3.1.3]).
///
/// Vegetation cells may overspill their own bounds. The presenting tile owns a cell when
/// the cell **center** lies in the half-open XZ footprint. Placement offsets still clip
/// to this tile ([`Self::contains_xz`]), not to a neighbor recipe's home square.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct GroveExtent {
	min: Vec3,
	max: Vec3,
}

impl GroveExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
	}

	pub fn min(&self) -> Vec3 {
		self.min
	}

	pub fn max(&self) -> Vec3 {
		self.max
	}

	/// World-aligned planting cells whose **center** lies in this tile (half-open XZ).
	///
	/// Origins sit on `k * cell_extent`, not on `self.min`, so adjacent tiles of the
	/// same grove share one lattice. Edge cells keep their full span (not clipped).
	pub fn subdivide_xz(&self, cell_extent_xz: Vec2) -> Vec<Cell> {
		self.cells_overlapping(cell_extent_xz)
	}

	/// Same as [`Self::subdivide_xz`]: world-aligned cells owned by this tile.
	pub fn cells_overlapping(&self, cell_extent_xz: Vec2) -> Vec<Cell> {
		let step = cell_extent_xz.max(Vec2::splat(0.1));
		let ix0 = ((self.min.x / step.x).floor() as i32).saturating_sub(1);
		let ix1 = ((self.max.x / step.x).ceil() as i32).saturating_add(1);
		let iz0 = ((self.min.z / step.y).floor() as i32).saturating_sub(1);
		let iz1 = ((self.max.z / step.y).ceil() as i32).saturating_add(1);
		let mut cells = Vec::with_capacity(((ix1 - ix0).max(0) * (iz1 - iz0).max(0)) as usize);
		for ix in ix0..ix1 {
			for iz in iz0..iz1 {
				let min = Vec3::new(ix as f32 * step.x, self.min.y, iz as f32 * step.y);
				let max = Vec3::new(min.x + step.x, self.max.y, min.z + step.y);
				let center = Vec3::new((min.x + max.x) * 0.5, 0.0, (min.z + max.z) * 0.5);
				if self.owns_center_xz(center) {
					cells.push(Cell(Aabb3d::from_min_max(min, max)));
				}
			}
		}
		cells
	}

	/// Half-open XZ ownership for planting-cell centers (`[min, max)`).
	pub fn owns_center_xz(&self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x < self.max.x
			&& position.z >= self.min.z
			&& position.z < self.max.z
	}

	/// Whether `position` lies inside the presenting tile on XZ (Y is ignored).
	///
	/// Inclusive on the max face so a placement offset can sit on the shared edge
	/// without being rejected; cell **ownership** uses [`Self::owns_center_xz`].
	pub fn contains_xz(&self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x <= self.max.x
			&& position.z >= self.min.z
			&& position.z <= self.max.z
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn new_orders_bounds() -> Result<()> {
		let extent = GroveExtent::new(Vec3::new(8.0, 1.0, 4.0), Vec3::ZERO);
		assert_eq!(extent.min(), Vec3::new(0.0, 0.0, 0.0));
		assert_eq!(extent.max(), Vec3::new(8.0, 1.0, 4.0));
		Ok(())
	}

	#[test]
	fn subdivide_xz_is_world_aligned_and_center_owned() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(10.0, 1.0, 9.0));
		let cells = extent.subdivide_xz(Vec2::new(4.0, 3.0));
		// X centers 2, 6 (10 is not < 10). Z centers 1.5, 4.5, 7.5.
		assert_eq!(cells.len(), 6);
		let first = cells.first().ok_or_else(|| anyhow::anyhow!("expected at least one cell"))?;
		assert_eq!(Vec3::from(first.as_region().min), Vec3::new(0.0, 0.0, 0.0));
		assert_eq!(Vec3::from(first.as_region().max), Vec3::new(4.0, 1.0, 3.0));
		Ok(())
	}

	#[test]
	fn adjacent_tiles_do_not_share_a_cell_center() -> Result<()> {
		let step = Vec2::splat(3.25);
		let west = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let east = GroveExtent::new(Vec3::new(100.0, 0.0, 0.0), Vec3::new(200.0, 1.0, 100.0));
		let west_cells = west.subdivide_xz(step);
		let east_cells = east.subdivide_xz(step);
		let west_mins: std::collections::HashSet<_> = west_cells
			.iter()
			.map(|c| (c.as_region().min.x.to_bits(), c.as_region().min.z.to_bits()))
			.collect();
		for cell in &east_cells {
			let key = (cell.as_region().min.x.to_bits(), cell.as_region().min.z.to_bits());
			assert!(!west_mins.contains(&key), "shared planting cell at {key:?}");
		}
		Ok(())
	}

	#[test]
	fn contains_xz_ignores_y() -> Result<()> {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(4.0, 1.0, 4.0));
		assert!(extent.contains_xz(Vec3::new(2.0, 99.0, 2.0)));
		assert!(!extent.contains_xz(Vec3::new(6.0, 0.0, 2.0)));
		Ok(())
	}
}
