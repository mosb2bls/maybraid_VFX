//! Inner lattice for update frequency; outer cube is the emitted refresh region.

use bevy::math::bounding::Aabb3d;
use bevy::math::{IVec3, Vec3};
use bevy::prelude::*;

use crate::lod_ref::LodRef;

use super::produce::{LodRefreshRegions, LodRefreshRegionsStatus};

/// Bullseye region production: lattice-gated outer cube.
///
/// - **Inner** — cubic cell size that controls how often a region is emitted
///   (only when the driver crosses into a new inner cell).
/// - **Outer** — edge length of the AABB centered on that inner cell (the
///   actual refresh region).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Bullseye {
	/// Inner cell edge length (world units).
	pub inner: f32,
	/// Outer refresh-region edge length (world units).
	pub outer: f32,
}

impl Default for Bullseye {
	fn default() -> Self {
		Self { inner: 50.0, outer: 500.0 }
	}
}

impl Bullseye {
	pub fn new(inner: f32, outer: f32) -> Self {
		Self { inner, outer }
	}

	fn cell_index(&self, point: Vec3) -> IVec3 {
		let s = self.inner;
		IVec3::new(
			(point.x / s).floor() as i32,
			(point.y / s).floor() as i32,
			(point.z / s).floor() as i32,
		)
	}

	fn cell_center(&self, index: IVec3) -> Vec3 {
		(index.as_vec3() + Vec3::splat(0.5)) * self.inner
	}

	fn outer_aabb(&self, center: Vec3) -> Aabb3d {
		cube_aabb(center, self.outer)
	}
}

fn cube_aabb(center: Vec3, edge: f32) -> Aabb3d {
	let half = Vec3::splat(edge * 0.5);
	Aabb3d::from_min_max(center - half, center + half)
}

impl LodRefreshRegions for Bullseye {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		let current = self.cell_index(lod_ref.current_transform.translation);
		let previous = self.cell_index(lod_ref.previous_transform.translation);
		if current == previous {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(self.outer_aabb(self.cell_center(current)))
	}

	fn lod_coverage(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		let current = self.cell_index(lod_ref.current_transform.translation);
		Some(self.outer_aabb(self.cell_center(current)))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::Transform;

	fn lod_ref_at<'a>(prev: &'a Transform, curr: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::from_bits(1),
			previous_transform: prev,
			current_transform: curr,
			bounds,
		}
	}

	#[test]
	fn unchanged_when_inner_cell_stable() {
		let bullseye = Bullseye::new(50.0, 500.0);
		let prev = Transform::from_translation(Vec3::new(10.0, 10.0, 10.0));
		let curr = Transform::from_translation(Vec3::new(20.0, 20.0, 20.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let status = bullseye.lod_refresh_regions(&lod_ref_at(&prev, &curr, &bounds));
		assert!(matches!(status, LodRefreshRegionsStatus::Unchanged));
	}

	#[test]
	fn changed_emits_outer_cube_on_inner_cell() {
		let bullseye = Bullseye::new(50.0, 500.0);
		let prev = Transform::from_translation(Vec3::new(-25.0, 0.0, 0.0));
		let curr = Transform::from_translation(Vec3::new(10.0, 10.0, 10.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let LodRefreshRegionsStatus::Changed(region) =
			bullseye.lod_refresh_regions(&lod_ref_at(&prev, &curr, &bounds))
		else {
			panic!("expected Changed");
		};
		// current in cell (0,0,0) for 50m → center 25; outer 500 → [-225, 275]
		assert_eq!(region.min.x, -225.0);
		assert_eq!(region.max.x, 275.0);
		assert_eq!(region.min.y, -225.0);
		assert_eq!(region.max.y, 275.0);
	}
}
