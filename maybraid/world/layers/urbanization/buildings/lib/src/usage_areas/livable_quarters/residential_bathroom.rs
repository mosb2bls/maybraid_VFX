//! Residential bathroom usage area: empty shell (label + passage keep-outs).

mod layout;
mod parameterized;

pub use parameterized::{ResidentialBathroomParameterized, ResidentialBathroomPlan, SCOPE};

use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct ResidentialBathroom {
	pub room_type: LabelNode,
}

impl ResidentialBathroom {
	pub fn from_plan(_plan: ResidentialBathroomPlan, confines: &Confines) -> Self {
		Self {
			room_type: label_filling_aabb(
				LabelStyle::Cyan,
				"ResidentialBathroom",
				&confines.bounds,
				confines.roll,
			),
		}
	}
}

impl Fit for ResidentialBathroom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = ResidentialBathroomParameterized::sample(confines, noise)?;
		let plan = ResidentialBathroomPlan::from_parameterized(params, confines)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for ResidentialBathroom {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::from_free(vec![self.room_type.clone()])
	}
}
