//! Shared placement and plan-footprint records for development buildings.

use bevy_math::bounding::Aabb2d;
use bevy_math::Vec2;
use buildings::{Confines, FillableRegions, Fit, FitError};
use procedural_common::NoiseParams;

/// Building geometry authored axis-aligned in world coordinates, plus the pose
/// applied around its own plan center at host spawn.
#[derive(Debug, Clone, PartialEq)]
pub struct PlacedBuilding<T> {
	pub center_xz: Vec2,
	pub yaw: f32,
	pub footprint: Vec2,
	pub ground_height: f32,
	pub building: T,
}

impl<T> PlacedBuilding<T> {
	pub fn map<U>(self, map: impl FnOnce(T) -> U) -> PlacedBuilding<U> {
		PlacedBuilding {
			center_xz: self.center_xz,
			yaw: self.yaw,
			footprint: self.footprint,
			ground_height: self.ground_height,
			building: map(self.building),
		}
	}
}

/// `T` fitted to the unrotated confines, posed at their center and floor with
/// their roll as yaw.
impl<T: Fit> Fit for PlacedBuilding<T> {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (building, regions) = T::fit_to_confines(confines, noise)?;
		Ok((
			Self {
				center_xz: confines.center_xz(),
				yaw: confines.roll,
				footprint: confines.footprint(),
				ground_height: confines.bounds.min.y,
				building,
			},
			regions,
		))
	}
}

/// Axis-aligned authored plan pieces before [`PlacedBuilding::yaw`] is applied.
pub trait BuildingFootprint {
	fn footprint_rects(&self) -> Vec<Aabb2d>;
}
