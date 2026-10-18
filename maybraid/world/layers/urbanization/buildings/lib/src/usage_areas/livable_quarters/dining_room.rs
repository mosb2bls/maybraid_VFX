//! Dining room usage area: table + optional side furniture.

mod layout;
mod parameterized;

pub use parameterized::{DiningRoomParameterized, DiningRoomPlan, SCOPE};

use building_components::furniture::FurnitureNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::furniture_util::{furniture_fill, FurnitureFill};
use crate::usage_areas::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct DiningRoom {
	pub room_type: LabelNode,
	pub tables: Vec<FurnitureFill>,
	pub fillers: Vec<FurnitureFill>,
}

impl DiningRoom {
	pub fn from_plan(plan: DiningRoomPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let host = &confines.bounds;
		let tables = plan
			.packed
			.tables
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"DiningTable",
					aabb,
					host,
					confines.roll,
					FurnitureNode::table,
				)
			})
			.collect();
		let fillers = plan
			.packed
			.fillers
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"DiningFiller",
					aabb,
					host,
					confines.roll,
					FurnitureNode::nightstand,
				)
			})
			.collect();
		Self {
			room_type: label_filling_aabb(
				LabelStyle::Green,
				"DiningRoom",
				&confines.bounds,
				confines.roll,
			),
			tables,
			fillers,
		}
	}
}

impl Fit for DiningRoom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = DiningRoomParameterized::sample(confines, noise)?;
		let plan = DiningRoomPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl DiningRoom {
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		params: DiningRoomParameterized,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = DiningRoomPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for DiningRoom {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.room_type.clone()];
		labels.extend(self.tables.iter().map(|f| f.label.clone()));
		labels.extend(self.fillers.iter().map(|f| f.label.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(Layers::from_free(self.tables.iter().map(|f| f.furniture.clone()).collect()));
		out.extend(Layers::from_free(self.fillers.iter().map(|f| f.furniture.clone()).collect()));
		out
	}
}
