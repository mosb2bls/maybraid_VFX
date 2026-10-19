//! Regular XZ host lattice and overlapping query regions.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

/// Regular XZ lattice used by the index and fill production-path microbenches.
#[derive(Clone, Copy, Debug)]
pub struct HostGrid {
	pub hosts: usize,
	pub spacing: f32,
}

impl HostGrid {
	pub const SMALL: Self = Self { hosts: 64, spacing: 96.0 };
	pub const LARGE: Self = Self { hosts: 4_096, spacing: 16.0 };

	pub fn lattice_scenarios() -> [Self; 2] {
		[Self::SMALL, Self::LARGE]
	}

	pub fn side(self) -> usize {
		(self.hosts as f64).sqrt().ceil().max(1.0) as usize
	}

	pub fn extent(self) -> f32 {
		self.side() as f32 * self.spacing
	}

	pub fn host_xz(self, i: usize) -> (f32, f32) {
		let side = self.side();
		((i % side) as f32 * self.spacing, (i / side) as f32 * self.spacing)
	}

	pub fn host_translation(self, i: usize) -> Vec3 {
		let (x, z) = self.host_xz(i);
		Vec3::new(x, 0.0, z)
	}

	pub fn local_host_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-8.0, -2.0, -8.0), Vec3::new(8.0, 8.0, 8.0))
	}

	pub fn host_aabb(self, i: usize) -> Aabb3d {
		let translation = self.host_translation(i);
		let local = Self::local_host_bounds();
		Aabb3d::from_min_max(
			Vec3::from(local.min) + translation,
			Vec3::from(local.max) + translation,
		)
	}

	/// Four overlapping 55% corner AABBs. Their union covers the lattice.
	pub fn overlapping_regions(self) -> [Aabb3d; 4] {
		let extent = self.extent();
		let mid = extent * 0.55;
		[
			Aabb3d::from_min_max(Vec3::new(0.0, -16.0, 0.0), Vec3::new(mid, 32.0, mid)),
			Aabb3d::from_min_max(Vec3::new(extent - mid, -16.0, 0.0), Vec3::new(extent, 32.0, mid)),
			Aabb3d::from_min_max(Vec3::new(0.0, -16.0, extent - mid), Vec3::new(mid, 32.0, extent)),
			Aabb3d::from_min_max(
				Vec3::new(extent - mid, -16.0, extent - mid),
				Vec3::new(extent, 32.0, extent),
			),
		]
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn small_lattice_is_eight_by_eight() {
		assert_eq!(HostGrid::SMALL.side(), 8);
		assert_eq!(HostGrid::SMALL.extent(), 768.0);
		assert_eq!(HostGrid::SMALL.host_translation(9), Vec3::new(96.0, 0.0, 96.0));
	}

	#[test]
	fn overlapping_corners_cover_every_host_center() {
		let grid = HostGrid::SMALL;
		let regions = grid.overlapping_regions();
		for i in 0..grid.hosts {
			let p = grid.host_translation(i);
			let covered = regions.iter().any(|region| {
				p.x >= region.min.x
					&& p.x <= region.max.x
					&& p.z >= region.min.z
					&& p.z <= region.max.z
			});
			assert!(covered, "host {i} at {p:?} missed overlapping regions");
		}
	}
}
