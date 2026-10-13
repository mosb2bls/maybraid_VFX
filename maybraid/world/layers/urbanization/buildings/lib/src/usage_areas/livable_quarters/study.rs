//! Study usage area: wall desk + optional bookcase.

mod layout;
mod parameterized;

pub use parameterized::{StudyParameterized, StudyPlan, SCOPE};

use building_components::furniture::FurnitureNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::furniture_util::{furniture_fill, FurnitureFill};
use crate::usage_areas::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct Study {
	pub room_type: LabelNode,
	pub desks: Vec<FurnitureFill>,
	pub bookcases: Vec<FurnitureFill>,
}

impl Study {
	pub fn from_plan(plan: StudyPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let host = &confines.bounds;
		let desks = plan
			.packed
			.desks
			.iter()
			.map(|aabb| {
				furniture_fill(style, "Desk", aabb, host, confines.roll, FurnitureNode::dresser)
			})
			.collect();
		let bookcases = plan
			.packed
			.bookcases
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"Bookcase",
					aabb,
					host,
					confines.roll,
					FurnitureNode::wardrobe,
				)
			})
			.collect();
		Self {
			room_type: label_filling_aabb(
				LabelStyle::Blue,
				"Study",
				&confines.bounds,
				confines.roll,
			),
			desks,
			bookcases,
		}
	}
}

impl Fit for Study {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = StudyParameterized::sample(confines, noise)?;
		let plan = StudyPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl Study {
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		params: StudyParameterized,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = StudyPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for Study {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.room_type.clone()];
		labels.extend(self.desks.iter().map(|f| f.label.clone()));
		labels.extend(self.bookcases.iter().map(|f| f.label.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(Layers::from_free(self.desks.iter().map(|f| f.furniture.clone()).collect()));
		out.extend(Layers::from_free(self.bookcases.iter().map(|f| f.furniture.clone()).collect()));
		out
	}
}
