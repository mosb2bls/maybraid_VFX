//! Eating area: kitchen + dining side-by-side, or kitchen-only fallback.

mod layout;
mod parameterized;

pub use parameterized::{EatingAreaPacked, EatingAreaParameterized, EatingAreaPlan, SCOPE};

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::label_util::label_filling_aabb;
use crate::usage_areas::livable_quarters::dining_room::DiningRoom;
use crate::usage_areas::livable_quarters::kitchen::Kitchen;

use parameterized::EatingAreaPacked as Packed;

/// Unified eating zone: dining beside kitchen when space allows, else kitchen only.
#[derive(Debug, Clone, PartialEq)]
pub struct EatingArea {
	pub room_type: LabelNode,
	pub kitchen: Kitchen,
	pub dining: Option<DiningRoom>,
}

impl EatingArea {
	pub fn from_plan(plan: EatingAreaPlan, confines: &Confines) -> Self {
		let y0 = Vec3::from(confines.bounds.min).y;
		let y1 = Vec3::from(confines.bounds.max).y;
		let roll = confines.roll;
		match plan.packed {
			Packed::KitchenDining { kitchen, dining, kitchen_xz, dining_xz } => {
				let k_conf = Confines::new(
					Aabb3d::from_min_max(
						Vec3::new(kitchen_xz.min.x, y0, kitchen_xz.min.y),
						Vec3::new(kitchen_xz.max.x, y1, kitchen_xz.max.y),
					),
					roll,
					confines.openings.clone(),
				);
				let d_conf = Confines::new(
					Aabb3d::from_min_max(
						Vec3::new(dining_xz.min.x, y0, dining_xz.min.y),
						Vec3::new(dining_xz.max.x, y1, dining_xz.max.y),
					),
					roll,
					confines.openings.clone(),
				);
				Self {
					room_type: label_filling_aabb(
						LabelStyle::Yellow,
						"EatingArea",
						&confines.bounds,
						roll,
					),
					kitchen: Kitchen::from_plan(kitchen, &k_conf),
					dining: Some(DiningRoom::from_plan(dining, &d_conf)),
				}
			}
			Packed::KitchenOnly { kitchen } => Self {
				room_type: label_filling_aabb(
					LabelStyle::Yellow,
					"EatingArea",
					&confines.bounds,
					roll,
				),
				kitchen: Kitchen::from_plan(kitchen, confines),
				dining: None,
			},
		}
	}

	pub fn has_dining(&self) -> bool {
		self.dining.is_some()
	}
}

impl Fit for EatingArea {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = EatingAreaParameterized::sample(confines, noise)?;
		let plan = EatingAreaPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl EatingArea {
	pub fn fit_with_fill(
		confines: &Confines,
		noise: NoiseParams,
		params: EatingAreaParameterized,
	) -> Result<(Self, FillableRegions), FitError> {
		let plan = EatingAreaPlan::from_parameterized(params, confines, noise)?;
		let room = Self::from_plan(plan, confines);
		Ok((room, FillableRegions::empty()))
	}
}

impl BuildingComponents for EatingArea {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		out.extend(self.kitchen.panel_nodes_for_level(level));
		if let Some(d) = &self.dining {
			out.extend(d.panel_nodes_for_level(level));
		}
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = Layers::new();
		out.extend(self.kitchen.joint_nodes_for_level(level));
		if let Some(d) = &self.dining {
			out.extend(d.joint_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		let mut out = Layers::new();
		out.push_free(self.room_type.clone());
		out.extend(self.kitchen.label_nodes_for_level(level));
		if let Some(d) = &self.dining {
			out.extend(d.label_nodes_for_level(level));
		}
		out
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		let mut out = Layers::new();
		out.extend(self.kitchen.furniture_nodes_for_level(level));
		if let Some(d) = &self.dining {
			out.extend(d.furniture_nodes_for_level(level));
		}
		out
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	fn host_with_south_door(sx: f32, sz: f32) -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::new(
				Aabb3d::from_min_max(
					Vec3::new(sx * 0.35, 0.0, -0.15),
					Vec3::new(sx * 0.65, 2.2, 0.15),
				),
				OpeningLabel::Passage,
			),
		);
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(sx, 3.0, sz)), 0.0, openings)
	}

	#[test]
	fn large_host_gets_kitchen_and_dining() {
		let confines = host_with_south_door(8.0, 10.0);
		let (area, _) = EatingArea::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(area.has_dining(), "expected dining beside kitchen");
	}

	#[test]
	fn small_host_falls_back_to_kitchen() {
		let confines = host_with_south_door(3.5, 4.0);
		let (area, _) = EatingArea::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!area.has_dining(), "expected kitchen-only fallback");
	}

	#[test]
	fn compact_galley_host_still_gets_kitchen() {
		// Near kitchen layout MIN_AREA (2.4×2.0); should kitchen-only, not fail.
		let confines = host_with_south_door(2.6, 2.5);
		let (area, _) = EatingArea::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(!area.has_dining());
		assert!(
			!area.kitchen.counter_runs.is_empty()
				|| area.kitchen.room_type.text == "Kitchen"
				|| area.room_type.text.contains("Eating"),
			"expected a fitted kitchen in a compact host"
		);
	}

	#[test]
	fn large_host_kitchen_footprint_is_generous() {
		let confines = host_with_south_door(12.0, 10.0);
		let (area, _) =
			EatingArea::fit_to_confines(&confines, NoiseParams { seed: 11, ..Default::default() })
				.unwrap();
		let k = &area.kitchen.room_type.placement.scale;
		let kitchen_a = k.x * k.z;
		// On a 120 m² host, kitchen half / kitchen-only should not stay galley-tiny.
		assert!(
			kitchen_a > 20.0,
			"expected a roomy kitchen on a large host, got {kitchen_a:.1} m²"
		);
	}
}
