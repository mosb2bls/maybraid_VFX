//! Urbanization cell footprint (1600 m), parallel to forest cells.

use bevy::math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::gen::{Id, OriginalId};

/// Default square urbanization cell span in metres on X and Z.
pub const DEFAULT_URBANIZATION_EXTENT_XZ: f32 = 1600.0;

/// Axis-aligned urbanization cell in world space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UrbanizationExtent {
	min: Vec3,
	max: Vec3,
}

impl UrbanizationExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
	}

	/// Square cell of [`DEFAULT_URBANIZATION_EXTENT_XZ`] centered on the origin.
	pub fn default_cell() -> Self {
		let h = DEFAULT_URBANIZATION_EXTENT_XZ * 0.5;
		Self::new(Vec3::new(-h, 0.0, -h), Vec3::new(h, 1.0, h))
	}

	pub fn min(self) -> Vec3 {
		self.min
	}

	pub fn max(self) -> Vec3 {
		self.max
	}

	pub fn center(self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		Some(Self::new(bounds.min.into(), bounds.max.into()))
	}

	/// The cell whose guillotine produced the leaf with id `leaf`.
	///
	/// Leaves partition their cell, so the leaf center lies inside it.
	pub fn owning_leaf(leaf: Id) -> Option<Self> {
		let bounds = leaf.origin_cell_bounds()?;
		let center = (Vec3::from(bounds.min) + Vec3::from(bounds.max)) * 0.5;
		let (ix, iz) = Self::cell_index_containing(center);
		Some(Self::from_cell_index(ix, iz))
	}

	/// Urbanization cells whose footprints overlap `region` on XZ.
	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let min_idx = Self::cell_index_containing(Vec3::new(region.min.x, 0.0, region.min.z));
		let max_x = (region.max.x - 1e-3).max(region.min.x);
		let max_z = (region.max.z - 1e-3).max(region.min.z);
		let max_idx = Self::cell_index_containing(Vec3::new(max_x, 0.0, max_z));
		let (x0, x1) = (min_idx.0.min(max_idx.0), min_idx.0.max(max_idx.0));
		let (z0, z1) = (min_idx.1.min(max_idx.1), min_idx.1.max(max_idx.1));
		(x0..=x1)
			.flat_map(|ix| (z0..=z1).map(move |iz| Self::from_cell_index(ix, iz)))
			.collect()
	}

	/// [`OriginalId`]s of cells whose footprints overlap `region` on XZ.
	pub fn original_ids_overlapping(region: Aabb3d) -> Vec<OriginalId> {
		Self::cells_overlapping(region)
			.into_iter()
			.map(|extent| OriginalId(extent.id()))
			.collect()
	}

	/// Axis-aligned XZ disk of `radius` metres around `center`.
	pub fn xz_radius_aabb(center: Vec3, radius: f32) -> Aabb3d {
		let r = radius.max(0.0);
		Aabb3d::from_min_max(
			Vec3::new(center.x - r, 0.0, center.z - r),
			Vec3::new(center.x + r, 1.0, center.z + r),
		)
	}

	/// AABB covering a Chebyshev ring of urbanization cells.
	pub fn ring_aabb(center: (i32, i32), radius: u32) -> Aabb3d {
		let r = radius as i32;
		let min_e = Self::from_cell_index(center.0 - r, center.1 - r);
		let max_e = Self::from_cell_index(center.0 + r, center.1 + r);
		Aabb3d::from_min_max(min_e.min(), max_e.max())
	}

	/// Half-open XZ (`[min, max)`) for ownership tests.
	pub fn owns_center_xz(self, position: Vec3) -> bool {
		position.x >= self.min.x
			&& position.x < self.max.x
			&& position.z >= self.min.z
			&& position.z < self.max.z
	}

	/// Origin-centered cell whose `(0, 0)` index is [`Self::default_cell`].
	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let s = DEFAULT_URBANIZATION_EXTENT_XZ;
		let h = s * 0.5;
		Self::new(
			Vec3::new(ix as f32 * s - h, 0.0, iz as f32 * s - h),
			Vec3::new(ix as f32 * s + h, 1.0, iz as f32 * s + h),
		)
	}

	/// Cell index containing `position` on the origin-centered 1600 m grid.
	///
	/// The +X / +Z faces are exclusive so a point on a shared edge belongs to the
	/// higher-index neighbor.
	pub fn cell_index_containing(position: Vec3) -> (i32, i32) {
		let s = DEFAULT_URBANIZATION_EXTENT_XZ;
		let h = s * 0.5;
		let ix = ((position.x + h) / s).floor() as i32;
		let iz = ((position.z + h) / s).floor() as i32;
		(ix, iz)
	}

	/// Inclusive Chebyshev ring of cell indices around `center`.
	pub fn cell_ring(center: (i32, i32), radius: u32) -> impl Iterator<Item = (i32, i32)> {
		let r = radius as i32;
		let (cx, cz) = center;
		(-r..=r).flat_map(move |dx| (-r..=r).map(move |dz| (cx + dx, cz + dz)))
	}

	/// Stay on `current` until `position` is `margin` metres inside a neighboring cell.
	///
	/// Stops the streamer from thrashing when the camera sits on a shared face.
	pub fn cell_index_committed(position: Vec3, current: (i32, i32), margin: f32) -> (i32, i32) {
		let raw = Self::cell_index_containing(position);
		if raw == current {
			return current;
		}
		let next = Self::from_cell_index(raw.0, raw.1);
		let m = margin.max(0.0);
		let min = next.min();
		let max = next.max();
		let committed = position.x >= min.x + m
			&& position.x <= max.x - m
			&& position.z >= min.z + m
			&& position.z <= max.z - m;
		if committed {
			raw
		} else {
			current
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn owns_center_xz_is_half_open() -> Result<()> {
		let cell = UrbanizationExtent::default_cell();
		assert!(cell.owns_center_xz(Vec3::ZERO));
		assert!(cell.owns_center_xz(Vec3::new(-800.0, 0.0, 0.0)));
		assert!(!cell.owns_center_xz(Vec3::new(800.0, 0.0, 0.0)));
		Ok(())
	}

	#[test]
	fn origin_cell_index_matches_default_cell() -> Result<()> {
		assert_eq!(UrbanizationExtent::from_cell_index(0, 0), UrbanizationExtent::default_cell());
		assert_eq!(UrbanizationExtent::cell_index_containing(Vec3::ZERO), (0, 0));
		assert_eq!(UrbanizationExtent::cell_index_containing(Vec3::new(799.9, 0.0, 0.0)), (0, 0));
		assert_eq!(UrbanizationExtent::cell_index_containing(Vec3::new(800.0, 0.0, 0.0)), (1, 0));
		assert_eq!(UrbanizationExtent::cell_index_containing(Vec3::new(-800.0, 0.0, 0.0)), (0, 0));
		assert_eq!(UrbanizationExtent::cell_index_containing(Vec3::new(-800.1, 0.0, 0.0)), (-1, 0));
		Ok(())
	}

	#[test]
	fn cell_ring_radius_one_is_three_by_three() -> Result<()> {
		let cells: Vec<_> = UrbanizationExtent::cell_ring((0, 0), 1).collect();
		assert_eq!(cells.len(), 9);
		assert!(cells.contains(&(0, 0)));
		assert!(cells.contains(&(1, -1)));
		Ok(())
	}

	#[test]
	fn ring_aabb_covers_chebyshev_radius() -> Result<()> {
		let aabb = UrbanizationExtent::ring_aabb((0, 0), 1);
		assert!((aabb.min.x + 2400.0).abs() < 1e-3);
		assert!((aabb.max.x - 2400.0).abs() < 1e-3);
		assert_eq!(UrbanizationExtent::cells_overlapping(aabb).len(), 9);
		Ok(())
	}

	#[test]
	fn cell_index_committed_ignores_shared_face() -> Result<()> {
		let on_face = Vec3::new(800.0, 0.0, 0.0);
		assert_eq!(UrbanizationExtent::cell_index_containing(on_face), (1, 0));
		assert_eq!(UrbanizationExtent::cell_index_committed(on_face, (0, 0), 80.0), (0, 0));
		let inside_next = Vec3::new(880.0, 0.0, 0.0);
		assert_eq!(UrbanizationExtent::cell_index_committed(inside_next, (0, 0), 80.0), (1, 0));
		Ok(())
	}
}
