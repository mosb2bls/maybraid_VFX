//! Kitchen usage area: counter layouts (galley / L / peninsula) + optional island.

mod layout;
mod parameterized;

pub use layout::KitchenCounterLayout;
pub use parameterized::{KitchenParameterized, KitchenPlan, SCOPE};

use building_components::furniture::FurnitureNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::furniture_util::{furniture_fill, FurnitureFill};
use crate::usage_areas::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct Kitchen {
	pub room_type: LabelNode,
	pub counter_layout: KitchenCounterLayout,
	pub counter_runs: Vec<FurnitureFill>,
	pub peninsulas: Vec<FurnitureFill>,
	pub islands: Vec<FurnitureFill>,
	pub fillers: Vec<FurnitureFill>,
}

impl Kitchen {
	pub fn from_plan(plan: KitchenPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let counter_layout = plan.packed.layout.unwrap_or(KitchenCounterLayout::Galley);
		let host = &confines.bounds;
		let counter_runs = plan
			.packed
			.counter_runs
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"CounterRun",
					aabb,
					host,
					confines.roll,
					FurnitureNode::counter,
				)
			})
			.collect();
		let peninsulas = plan
			.packed
			.peninsulas
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"Peninsula",
					aabb,
					host,
					confines.roll,
					FurnitureNode::counter,
				)
			})
			.collect();
		let islands = plan
			.packed
			.islands
			.iter()
			.map(|aabb| {
				furniture_fill(style, "Island", aabb, host, confines.roll, FurnitureNode::counter)
			})
			.collect();
		let fillers = plan
			.packed
			.fillers
			.iter()
			.map(|aabb| {
				furniture_fill(
					style,
					"KitchenFiller",
					aabb,
					host,
					confines.roll,
					FurnitureNode::nightstand,
				)
			})
			.collect();
		Self {
			room_type: label_filling_aabb(
				LabelStyle::Yellow,
				"Kitchen",
				&confines.bounds,
				confines.roll,
			),
			counter_layout,
			counter_runs,
			peninsulas,
			islands,
			fillers,
		}
	}
}

impl Fit for Kitchen {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = KitchenParameterized::sample(confines, noise)?;
		let plan = KitchenPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl Kitchen {
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		params: KitchenParameterized,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = KitchenPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for Kitchen {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.room_type.clone()];
		labels.extend(self.counter_runs.iter().map(|f| f.label.clone()));
		labels.extend(self.peninsulas.iter().map(|f| f.label.clone()));
		labels.extend(self.islands.iter().map(|f| f.label.clone()));
		labels.extend(self.fillers.iter().map(|f| f.label.clone()));
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(Layers::from_free(
			self.counter_runs.iter().map(|f| f.furniture.clone()).collect(),
		));
		out.extend(Layers::from_free(
			self.peninsulas.iter().map(|f| f.furniture.clone()).collect(),
		));
		out.extend(Layers::from_free(self.islands.iter().map(|f| f.furniture.clone()).collect()));
		out.extend(Layers::from_free(self.fillers.iter().map(|f| f.furniture.clone()).collect()));
		out
	}
}
