//! Auto / hardware parts counter: small office + parts storage pockets.
//!
//! **Semantically:** staff office with a door onto the sales floor; leftover
//! plan pockets are parts bins / racks.
//!
//! **Programmatically:** same clearance → walled office (+door) → grow-into
//! parts pockets pattern as MiniMart, with smaller mins (office ≥2×2, parts
//! ≥2×2 / extras ≥1.5×1.5). Office fill region carries the authored door.

pub mod parameterized;

pub use parameterized::{PartsStallParameterized, PartsStallPlan};

use bevy_math::bounding::Aabb3d;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::openings::{Opening, OpeningId, Openings};
use crate::paneling::Rectangle;

use super::label_util::label_filling_aabb;
use super::stall_layout::parts::PartsOfficeDoor;

#[derive(Debug, Clone, PartialEq)]
pub struct PartsStall {
	pub stall_type: LabelNode,
	pub office_walls: Vec<Rectangle>,
	pub office_bounds: Aabb3d,
	pub parts_office: LabelNode,
	pub parts: Vec<LabelNode>,
	pub office_door_id: OpeningId,
	pub office_door: Opening,
}

impl PartsStall {
	pub fn from_plan(plan: PartsStallPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let parts = plan
			.packed
			.parts
			.iter()
			.map(|aabb| label_filling_aabb(LabelStyle::Gray, "Parts", aabb, confines.roll))
			.collect();
		let PartsOfficeDoor { id, opening } = plan.packed.office_door.clone();
		let office_bounds = plan.packed.office;
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Blue,
				"PartsStall",
				&confines.bounds,
				confines.roll,
			),
			office_walls: plan.packed.office_walls,
			office_bounds,
			parts_office: label_filling_aabb(style, "PartsOffice", &office_bounds, confines.roll),
			parts,
			office_door_id: id,
			office_door: opening,
		}
	}

	pub fn office_fill_region(&self, roll: f32) -> FillRegion {
		let mut openings = Openings::new();
		openings.insert(self.office_door_id.clone(), self.office_door.clone());
		FillRegion::new(SpaceKind::InternalSpace, Confines::new(self.office_bounds, roll, openings))
	}
}

impl Fit for PartsStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = PartsStallParameterized::sample(confines, noise)?;
		let plan = PartsStallPlan::from_parameterized(params, confines)?;
		let stall = Self::from_plan(plan, confines);
		let regions = FillableRegions {
			within: vec![stall.office_fill_region(confines.roll)],
			atop: Vec::new(),
		};
		Ok((stall, regions))
	}
}

impl BuildingComponents for PartsStall {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.office_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone(), self.parts_office.clone()];
		labels.extend(self.parts.iter().cloned());
		Layers::from_free(labels)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::Vec3;
	use procedural_common::{aabb3_to_plan, PlanAxes};

	use super::super::stall_layout::parts::{PARTS_OFFICE_MIN, SCOPE};

	fn roomy_south() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(3.0, 2.2, 0.2),
			)),
		);
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.2, 8.0)), 0.0, openings)
	}

	#[test]
	fn parts_fits_and_tracks_office_door() {
		let confines = roomy_south();
		let (stall, regions) =
			PartsStall::fit_to_confines(&confines, NoiseParams { seed: 3, ..Default::default() })
				.unwrap();
		assert_eq!(stall.stall_type.text.as_str(), "PartsStall");
		assert!(!stall.parts.is_empty());
		assert_eq!(stall.office_door_id, OpeningId::scoped(SCOPE, "office_door", "0"));
		assert!(matches!(stall.office_door.label, OpeningLabel::Passage));
		assert_eq!(regions.within.len(), 1);
		assert!(regions.within[0].confines.openings.get(&stall.office_door_id).is_some());

		let office = aabb3_to_plan(&stall.office_bounds, PlanAxes::XZ);
		assert!(office.max.x - office.min.x + 1e-3 >= PARTS_OFFICE_MIN);
		assert!(office.max.y - office.min.y + 1e-3 >= PARTS_OFFICE_MIN);
	}

	#[test]
	fn parts_soft_fails_tiny_bay() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.4, 0.0, -0.2),
				Vec3::new(1.4, 2.0, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(3.5, 3.0, 3.0)),
			0.0,
			openings,
		);
		assert!(matches!(
			PartsStall::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}
