//! Small janitorial closet along a hallway.
//!
//! Floor plans seat the slot; this type builds a shell envelope. Soft-fails when
//! the confines cannot host minimum extents.

use bevy_math::{Vec2, Vec3};
use building_components::joints::JointNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::shells::{RectFloor, RectFloorParams, RectFloorSlab};

/// Minimum plan extents for a janitorial closet (meters).
pub const MIN_JANITORIAL: Vec2 = Vec2::new(1.6, 1.6);
/// Minimum clear height.
pub const MIN_HEIGHT: f32 = 2.2;

/// Fitted janitorial closet shell.
#[derive(Debug, Clone, PartialEq)]
pub struct Janitorial {
	pub confines: Confines,
	pub shell: RectFloor,
}

impl Janitorial {
	pub fn from_confines(confines: &Confines) -> Result<(Self, FillableRegions), FitError> {
		let min = Vec3::from(confines.bounds.min);
		let max = Vec3::from(confines.bounds.max);
		let footprint = Vec2::new((max.x - min.x).max(0.0), (max.z - min.z).max(0.0));
		let height = (max.y - min.y).max(0.0);
		if footprint.x + 1e-3 < MIN_JANITORIAL.x || footprint.y + 1e-3 < MIN_JANITORIAL.y {
			return Err(FitError::TooSmall { reason: "janitorial_footprint" });
		}
		if height + 1e-3 < MIN_HEIGHT {
			return Err(FitError::TooSmall { reason: "janitorial_height" });
		}
		let center_xz = Vec3::new(0.5 * (min.x + max.x), min.y, 0.5 * (min.z + max.z));
		let shell = RectFloor::new(RectFloorParams {
			center_xz,
			footprint,
			storey_height: height,
			openings: confines.openings.clone(),
			floor: RectFloorSlab::Solid,
			ceiling: RectFloorSlab::None,
			..RectFloorParams::default()
		});
		Ok((Self { confines: confines.clone(), shell }, FillableRegions::empty()))
	}
}

impl Fit for Janitorial {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Self::from_confines(confines)
	}
}

impl BuildingComponents for Janitorial {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.shell.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.shell.joint_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::Openings;
	use bevy_math::bounding::Aabb3d;

	#[test]
	fn fits_minimum_closet() {
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 3.0, 2.0)),
			0.0,
			Openings::new(),
		);
		let (j, _) = Janitorial::from_confines(&confines).unwrap();
		assert!(j.shell.has_floor());
	}

	#[test]
	fn rejects_tiny() {
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 3.0, 1.0)),
			0.0,
			Openings::new(),
		);
		assert!(matches!(Janitorial::from_confines(&confines), Err(FitError::TooSmall { .. })));
	}
}
