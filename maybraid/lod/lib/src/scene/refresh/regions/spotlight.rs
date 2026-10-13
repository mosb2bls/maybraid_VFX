//! Continuous cube around the driver position (fires on translation change).

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;

use crate::lod_ref::LodRef;

use super::produce::{LodRefreshRegions, LodRefreshRegionsStatus};

/// Spotlight region production: cube centered on the driver's current position.
///
/// Emits whenever translation changes (rotation-only motion is [`Unchanged`]).
#[derive(Resource, Debug, Clone, Copy, PartialEq)]
pub struct Spotlight {
	/// Edge length of the cube around the driver (world units).
	pub extent: f32,
}

impl Default for Spotlight {
	fn default() -> Self {
		Self { extent: 20.0 }
	}
}

impl Spotlight {
	pub fn new(extent: f32) -> Self {
		Self { extent }
	}

	fn region_at(&self, center: Vec3) -> Aabb3d {
		let half = Vec3::splat(self.extent * 0.5);
		Aabb3d::from_min_max(center - half, center + half)
	}
}

impl LodRefreshRegions for Spotlight {
	fn lod_refresh_regions(&self, lod_ref: &LodRef) -> LodRefreshRegionsStatus {
		let prev = lod_ref.previous_transform.translation;
		let curr = lod_ref.current_transform.translation;
		if prev == curr {
			return LodRefreshRegionsStatus::Unchanged;
		}
		LodRefreshRegionsStatus::Changed(self.region_at(curr))
	}

	fn lod_coverage(&self, lod_ref: &LodRef) -> Option<Aabb3d> {
		Some(self.region_at(lod_ref.current_transform.translation))
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
	fn unchanged_when_translation_stable() {
		let spot = Spotlight::new(20.0);
		let t = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let status = spot.lod_refresh_regions(&lod_ref_at(&t, &t, &bounds));
		assert!(matches!(status, LodRefreshRegionsStatus::Unchanged));
	}

	#[test]
	fn changed_is_cube_around_current() {
		let spot = Spotlight::new(20.0);
		let prev = Transform::from_translation(Vec3::ZERO);
		let curr = Transform::from_translation(Vec3::new(10.0, 0.0, 0.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ZERO);
		let LodRefreshRegionsStatus::Changed(region) =
			spot.lod_refresh_regions(&lod_ref_at(&prev, &curr, &bounds))
		else {
			panic!("expected Changed");
		};
		assert_eq!(region.min.x, 0.0);
		assert_eq!(region.max.x, 20.0);
		assert_eq!(region.min.y, -10.0);
		assert_eq!(region.max.y, 10.0);
	}
}
