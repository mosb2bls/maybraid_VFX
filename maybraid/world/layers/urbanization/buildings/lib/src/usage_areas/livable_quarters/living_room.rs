//! Living room usage area: primary seating + optional secondary seating.

mod layout;
mod parameterized;

pub use parameterized::{LivingRoomParameterized, LivingRoomPlan, SCOPE};

use building_components::furniture::FurnitureNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::furniture_util::{furniture_fill, FurnitureFill};
use crate::usage_areas::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct LivingRoom {
	pub room_type: LabelNode,
	pub primary_seating: Vec<FurnitureFill>,
	pub secondary_seating: Vec<FurnitureFill>,
	pub fillers: Vec<FurnitureFill>,
}

impl LivingRoom {
	pub fn from_plan(plan: LivingRoomPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let host = &confines.bounds;
		let primary_seating = plan
			.packed
			.primary_seating
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"PrimarySeating",
					aabb,
					host,
					confines.roll,
					FurnitureNode::chair,
				)
			})
			.collect();
		let secondary_seating = plan
			.packed
			.secondary_seating
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"SecondarySeating",
					aabb,
					host,
					confines.roll,
					FurnitureNode::chair,
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
					"LivingFiller",
					aabb,
					host,
					confines.roll,
					FurnitureNode::dresser,
				)
			})
			.collect();
		Self {
			room_type: label_filling_aabb(
				LabelStyle::Orange,
				"LivingRoom",
				&confines.bounds,
				confines.roll,
			),
			primary_seating,
			secondary_seating,
			fillers,
		}
	}
}

impl Fit for LivingRoom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LivingRoomParameterized::sample(confines, noise)?;
		let plan = LivingRoomPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl LivingRoom {
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		params: LivingRoomParameterized,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = LivingRoomPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for LivingRoom {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.room_type.clone()];
		labels.extend(self.primary_seating.iter().map(|f| f.label.clone()));
		labels.extend(self.secondary_seating.iter().map(|f| f.label.clone()));
		labels.extend(self.fillers.iter().map(|f| f.label.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(Layers::from_free(
			self.primary_seating.iter().map(|f| f.furniture.clone()).collect(),
		));
		out.extend(Layers::from_free(
			self.secondary_seating.iter().map(|f| f.furniture.clone()).collect(),
		));
		out.extend(Layers::from_free(self.fillers.iter().map(|f| f.furniture.clone()).collect()));
		out
	}
}
