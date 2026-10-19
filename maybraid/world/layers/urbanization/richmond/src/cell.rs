//! 300 m development lattice and occupancy.

use bevy::math::bounding::Aabb3d;
use bevy::math::{Vec2, Vec3};
use lod::gen::{Id, OriginalId};

/// Square development-cell edge length (metres).
pub const DEVELOPMENT_CELL_SIZE: f32 = 300.0;

/// Vertical span used only for origin-cell identity (XZ tiling).
const CELL_Y: f32 = 1.0;

/// Default fill likelihood (`0.0..=1.0`).
pub const DEFAULT_LIKELIHOOD: f32 = 0.28;

/// Occupancy lattice spacing (world units). Larger → bigger clusters.
pub const DEFAULT_SPATIAL_CORRELATION: f32 = 300.0;

/// Inset from the cell edge so the building plus berm and ease stay on the tile.
pub const BUILDING_INSET: f32 = 14.0;

/// Axis-aligned development tile.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevelopmentExtent {
	min: Vec3,
	max: Vec3,
}

impl DevelopmentExtent {
	pub fn new(min: Vec3, max: Vec3) -> Self {
		Self { min: min.min(max), max: min.max(max) }
	}

	pub fn from_cell_index(ix: i32, iz: i32) -> Self {
		let s = DEVELOPMENT_CELL_SIZE;
		Self::new(
			Vec3::new(ix as f32 * s, 0.0, iz as f32 * s),
			Vec3::new((ix + 1) as f32 * s, CELL_Y, (iz + 1) as f32 * s),
		)
	}

	pub fn from_id(id: Id) -> Option<Self> {
		let bounds = id.origin_cell_bounds()?;
		Some(Self::new(bounds.min.into(), bounds.max.into()))
	}

	pub fn aabb(self) -> Aabb3d {
		Aabb3d::from_min_max(self.min, self.max)
	}

	pub fn id(self) -> Id {
		Id::from_cell(self.aabb())
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

	pub fn center_xz(self) -> Vec2 {
		let c = self.center();
		Vec2::new(c.x, c.z)
	}

	/// Tiles whose footprints overlap `region` on XZ.
	pub fn cells_overlapping(region: Aabb3d) -> Vec<Self> {
		let s = DEVELOPMENT_CELL_SIZE;
		let min_ix = (region.min.x / s).floor() as i32;
		let min_iz = (region.min.z / s).floor() as i32;
		let max_x = (region.max.x - 1e-3).max(region.min.x);
		let max_z = (region.max.z - 1e-3).max(region.min.z);
		let max_ix = (max_x / s).floor() as i32;
		let max_iz = (max_z / s).floor() as i32;
		let (x0, x1) = (min_ix.min(max_ix), min_ix.max(max_ix));
		let (z0, z1) = (min_iz.min(max_iz), min_iz.max(max_iz));
		(x0..=x1)
			.flat_map(|ix| (z0..=z1).map(move |iz| Self::from_cell_index(ix, iz)))
			.collect()
	}

	/// [`OriginalId`]s of cells whose footprints overlap `region` on XZ.
	pub fn original_ids_overlapping(region: Aabb3d) -> Vec<OriginalId> {
		Self::cells_overlapping(region)
			.into_iter()
			.map(|e| OriginalId(e.id()))
			.collect()
	}
}

/// Spatially correlated occupancy via bilinear value noise at the cell center.
///
/// Same scheme as Stamp leaf selection: `likelihood` is the approximate
/// fill fraction; `spatial_correlation` is the lattice spacing.
pub fn cell_selected(
	cell: Aabb3d,
	occupancy_seed: u32,
	likelihood: f32,
	spatial_correlation: f32,
) -> bool {
	let p = likelihood.clamp(0.0, 1.0);
	if p >= 1.0 {
		return true;
	}
	if p <= 0.0 {
		return false;
	}
	let center = Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5);
	occupancy_unit(center, occupancy_seed, spatial_correlation) < p
}

fn occupancy_unit(p: Vec2, seed: u32, spatial_correlation: f32) -> f32 {
	let spacing = spatial_correlation.max(1.0);
	let fx = p.x / spacing;
	let fz = p.y / spacing;
	let x0 = fx.floor() as i32;
	let z0 = fz.floor() as i32;
	let tx = fx - x0 as f32;
	let tz = fz - z0 as f32;
	let sx = tx * tx * (3.0 - 2.0 * tx);
	let sz = tz * tz * (3.0 - 2.0 * tz);
	let n00 = lattice_unit(seed, x0, z0);
	let n10 = lattice_unit(seed, x0 + 1, z0);
	let n01 = lattice_unit(seed, x0, z0 + 1);
	let n11 = lattice_unit(seed, x0 + 1, z0 + 1);
	let nx0 = n00 + (n10 - n00) * sx;
	let nx1 = n01 + (n11 - n01) * sx;
	nx0 + (nx1 - nx0) * sz
}

fn lattice_unit(seed: u32, ix: i32, iz: i32) -> f32 {
	let mut n = seed
		.wrapping_add((ix as u32).wrapping_mul(73856093))
		.wrapping_add((iz as u32).wrapping_mul(19349663));
	n = n.wrapping_mul(0x9E37_79B9) ^ (n >> 16);
	(n >> 8) as f32 / ((u32::MAX >> 8) as f32)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn overlapping_origin_is_one_cell() {
		let region = Aabb3d::from_min_max(Vec3::new(1.0, 0.0, 1.0), Vec3::new(2.0, 1.0, 2.0));
		let cells = DevelopmentExtent::cells_overlapping(region);
		assert_eq!(cells.len(), 1);
		assert_eq!(cells[0], DevelopmentExtent::from_cell_index(0, 0));
	}

	#[test]
	fn shared_development_lattice_is_three_hundred_metres() {
		let cell = DevelopmentExtent::from_cell_index(2, -1).aabb();
		assert!((cell.max.x - cell.min.x - 300.0).abs() < 1e-4);
		assert!((cell.max.z - cell.min.z - 300.0).abs() < 1e-4);
	}

	#[test]
	fn occupancy_is_deterministic() {
		let cell = DevelopmentExtent::from_cell_index(3, -2).aabb();
		assert_eq!(cell_selected(cell, 42, 0.4, 300.0), cell_selected(cell, 42, 0.4, 300.0));
	}

	#[test]
	fn likelihood_one_always_fills() {
		let cell = DevelopmentExtent::from_cell_index(0, 0).aabb();
		assert!(cell_selected(cell, 1, 1.0, 300.0));
		assert!(!cell_selected(cell, 1, 0.0, 300.0));
	}
}
