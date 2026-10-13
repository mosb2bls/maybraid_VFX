//! Residential half bathroom: empty shell (label + passage keep-outs).

mod layout;
mod parameterized;

pub use parameterized::{ResidentialHalfBathroomParameterized, ResidentialHalfBathroomPlan, SCOPE};

use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct ResidentialHalfBathroom {
	pub room_type: LabelNode,
}

impl ResidentialHalfBathroom {
	pub fn from_plan(_plan: ResidentialHalfBathroomPlan, confines: &Confines) -> Self {
		Self {
			room_type: label_filling_aabb(
				LabelStyle::Cyan,
				"ResidentialHalfBathroom",
				&confines.bounds,
				confines.roll,
			),
		}
	}
}

impl Fit for ResidentialHalfBathroom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = ResidentialHalfBathroomParameterized::sample(confines, noise)?;
		let plan = ResidentialHalfBathroomPlan::from_parameterized(params, confines)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for ResidentialHalfBathroom {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![self.room_type.clone()])
	}
}
