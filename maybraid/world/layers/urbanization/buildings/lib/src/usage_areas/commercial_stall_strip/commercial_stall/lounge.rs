//! Lounge: always-fit fallback when no catalog interior accepts the bay.
//!
//! **Semantically:** soft open sitting / holding space — “we still fill the cell.”
//!
//! **Programmatically:** single bay-filling label; never soft-fails (outside the
//! weighted catalog; last resort in [`super::interior`]).

use building_components::furniture::FurnitureNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::furniture_util::{chairs_in_aabb, FurnitureFill};

use super::label_util::label_filling_aabb;

/// Soft open lounge that accepts any non-degenerate confines.
#[derive(Debug, Clone, PartialEq)]
pub struct Lounge {
	pub stall_type: LabelNode,
	pub seating: Vec<FurnitureFill>,
}

impl Fit for Lounge {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		Ok((
			Self {
				stall_type: label_filling_aabb(
					LabelStyle::Gray,
					"Lounge",
					&confines.bounds,
					confines.roll,
				),
				seating: chairs_in_aabb(
					&confines.bounds,
					confines.roll,
					noise,
					LabelStyle::Gray,
					"LoungeSeating",
				),
			},
			FillableRegions::empty(),
		))
	}
}

impl BuildingComponents for Lounge {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone()];
		labels.extend(self.seating.iter().map(|fill| fill.label.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(self.seating.iter().map(|fill| fill.furniture.clone()).collect())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	#[test]
	fn lounge_fits_tiny_bay() {
		let confines =
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(0.5, 1.0, 0.5)));
		let (lounge, _) = Lounge::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert_eq!(lounge.stall_type.text, "Lounge");
		assert!(lounge.seating.is_empty());
	}

	#[test]
	fn lounge_bay_gets_chairs() {
		let confines =
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.2, 4.0)));
		let (lounge, _) = Lounge::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!lounge.seating.is_empty());
		assert!(lounge.seating.iter().all(|fill| {
			fill.furniture.geometry == building_components::FurnitureGeometry::Chair
		}));
	}
}
