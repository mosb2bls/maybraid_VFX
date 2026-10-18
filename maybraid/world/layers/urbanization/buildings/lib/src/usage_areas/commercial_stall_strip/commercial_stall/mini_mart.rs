//! Convenience grocery: entry clearances, back office, register, aisles, shelves.
//!
//! **Semantically:** shoppable floor with a private office and a checkout zone;
//! shelves hug free walls when space remains.
//!
//! **Programmatically:** `PassageClearance` → office (≥2×3) with sales-door
//! opening + enclosure panels → register ≥2×2 → aisle pockets → optional wall
//! shelves. Office residual is emitted in [`FillableRegions::within`].

pub mod parameterized;

pub use parameterized::{MiniMartParameterized, MiniMartPlan};

use building_components::furniture::FurnitureNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use bevy_math::bounding::Aabb3d;

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::openings::{Opening, OpeningId, Openings};
use crate::paneling::Rectangle;
use crate::usage_areas::furniture_util::{furniture_fill, FurnitureFill};

use super::label_util::label_filling_aabb;
use super::stall_layout::mini_mart::MiniMartOfficeDoor;

#[derive(Debug, Clone, PartialEq)]
pub struct MiniMart {
	pub stall_type: LabelNode,
	pub office_walls: Vec<Rectangle>,
	pub office_bounds: Aabb3d,
	pub office: LabelNode,
	pub stall_aisles: Vec<LabelNode>,
	pub register: FurnitureFill,
	pub grocery_shelves: Vec<LabelNode>,
	/// Tracked passage through the office sales divider.
	pub office_door_id: OpeningId,
	pub office_door: Opening,
}

impl MiniMart {
	pub fn from_plan(plan: MiniMartPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let stall_aisles = plan
			.packed
			.aisles
			.iter()
			.map(|aabb| label_filling_aabb(LabelStyle::Cyan, "StallAisles", aabb, confines.roll))
			.collect();
		let grocery_shelves = plan
			.packed
			.shelves
			.iter()
			.map(|aabb| label_filling_aabb(LabelStyle::Gray, "GroceryShelves", aabb, confines.roll))
			.collect();
		let MiniMartOfficeDoor { id, opening } = plan.packed.office_door.clone();
		let office_bounds = plan.packed.office;
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Blue,
				"MiniMart",
				&confines.bounds,
				confines.roll,
			),
			office_walls: plan.packed.office_walls,
			office_bounds,
			office: label_filling_aabb(style, "MiniMartOffice", &office_bounds, confines.roll),
			stall_aisles,
			register: furniture_fill(
				LabelStyle::Magenta,
				"Register",
				&plan.packed.register,
				&confines.bounds,
				confines.roll,
				FurnitureNode::counter,
			),
			grocery_shelves,
			office_door_id: id,
			office_door: opening,
		}
	}

	/// Office residual with the authored office-door passage on its confines.
	pub fn office_fill_region(&self, roll: f32) -> FillRegion {
		let mut openings = Openings::new();
		openings.insert(self.office_door_id.clone(), self.office_door.clone());
		FillRegion::new(SpaceKind::InternalSpace, Confines::new(self.office_bounds, roll, openings))
	}
}

impl Fit for MiniMart {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = MiniMartParameterized::sample(confines, noise)?;
		let plan = MiniMartPlan::from_parameterized(params, confines)?;
		let stall = Self::from_plan(plan, confines);
		let regions = FillableRegions {
			within: vec![stall.office_fill_region(confines.roll)],
			atop: Vec::new(),
		};
		Ok((stall, regions))
	}
}

impl BuildingComponents for MiniMart {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.office_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels =
			vec![self.stall_type.clone(), self.office.clone(), self.register.label.clone()];
		labels.extend(self.stall_aisles.iter().cloned());
		labels.extend(self.grocery_shelves.iter().cloned());
		Layers::from_free(labels)
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::from_free(vec![self.register.furniture.clone()])
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::{
		aabb2_area, aabb3_to_plan, intersects_aabb2, Aabb2dPack, PlanAxes, PlanOpeningFace,
	};

	use super::super::stall_layout::mini_mart::{
		MINI_MART_AISLES_MIN, MINI_MART_OFFICE_LONG_MIN, MINI_MART_OFFICE_SHORT_MIN,
		MINI_MART_PASSAGE_CLEARANCE, MINI_MART_REGISTER_MIN, SCOPE,
	};

	fn roomy_south_doors() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(1.0, 0.0, -0.2),
				Vec3::new(3.5, 2.2, 0.2),
			)),
		);
		openings.insert(
			OpeningId::new("door_b"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(8.0, 0.0, -0.2),
				Vec3::new(10.5, 2.2, 0.2),
			)),
		);
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(14.0, 3.0, 12.0)), 0.0, openings)
	}

	fn plan_extent(aabb: &Aabb3d) -> (f32, f32) {
		let p = aabb3_to_plan(aabb, PlanAxes::XZ);
		(p.max.x - p.min.x, p.max.y - p.min.y)
	}

	fn office_dims_ok(aabb: &Aabb3d) -> bool {
		let (w, d) = plan_extent(aabb);
		let (long, short) = if w >= d { (w, d) } else { (d, w) };
		long + 1e-3 >= MINI_MART_OFFICE_LONG_MIN && short + 1e-3 >= MINI_MART_OFFICE_SHORT_MIN
	}

	#[test]
	fn mini_mart_fits_roomy_bay() {
		let confines = roomy_south_doors();
		let (stall, regions) =
			MiniMart::fit_to_confines(&confines, NoiseParams { seed: 7, ..Default::default() })
				.unwrap();
		assert_eq!(stall.stall_type.text.as_str(), "MiniMart");
		assert_eq!(stall.register.label.text.as_str(), "Register");
		assert_eq!(
			stall.register.furniture.geometry,
			building_components::FurnitureGeometry::Counter
		);
		assert!(
			stall.office_walls.len() >= 3,
			"office should enclose open sides (laterals + door jambs/header), got {}",
			stall.office_walls.len()
		);
		assert!(!stall.stall_aisles.is_empty());
		assert_eq!(stall.office_door_id, OpeningId::scoped(SCOPE, "office_door", "0"));
		assert!(matches!(stall.office_door.label, OpeningLabel::Passage));
		assert_eq!(regions.within.len(), 1);
		assert!(regions.within[0].confines.openings.get(&stall.office_door_id).is_some());
	}

	#[test]
	fn mini_mart_meets_size_and_clearance_mins() {
		let confines = roomy_south_doors();
		let params = MiniMartParameterized::sample(
			&confines,
			NoiseParams { seed: 11, ..Default::default() },
		)
		.unwrap();
		let plan = MiniMartPlan::from_parameterized(params, &confines).unwrap();

		assert!(office_dims_ok(&plan.packed.office));
		let (rw, rd) = plan_extent(&plan.packed.register);
		assert!(rw + 1e-3 >= MINI_MART_REGISTER_MIN && rd + 1e-3 >= MINI_MART_REGISTER_MIN);
		assert!(!plan.packed.aisles.is_empty());
		let (aw, ad) = plan_extent(&plan.packed.aisles[0]);
		assert!(aw + 1e-3 >= MINI_MART_AISLES_MIN && ad + 1e-3 >= MINI_MART_AISLES_MIN);
		assert!(matches!(plan.packed.office_door.opening.label, OpeningLabel::Passage));

		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		for (_id, opening) in confines.openings.iter() {
			let Some(face) =
				PlanOpeningFace::from_passage(host, aabb3_to_plan(&opening.bounds, PlanAxes::XZ))
			else {
				continue;
			};
			let clear = face
				.band(host, face.along_len(), MINI_MART_PASSAGE_CLEARANCE, 0.5)
				.expect("clearance band");
			for region in std::iter::once(&plan.packed.office)
				.chain(std::iter::once(&plan.packed.register))
				.chain(plan.packed.aisles.iter())
			{
				let r = aabb3_to_plan(region, PlanAxes::XZ);
				assert!(
					r.is_clear_of(&[clear]) && !intersects_aabb2(r, clear),
					"region overlaps passage clearance"
				);
			}
		}
	}

	#[test]
	fn mini_mart_soft_fails_tiny_bay() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.5, 0.0, -0.2),
				Vec3::new(1.5, 2.0, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 3.5)),
			0.0,
			openings,
		);
		let err = MiniMart::fit_to_confines(&confines, NoiseParams::default()).unwrap_err();
		assert!(matches!(err, FitError::TooSmall { .. }));
	}

	#[test]
	fn mini_mart_office_area_varies_with_noise() {
		let confines = roomy_south_doors();
		let mut areas = Vec::new();
		for seed in [1i32, 2, 3, 5, 8, 13, 21, 34] {
			let params = MiniMartParameterized::sample(
				&confines,
				NoiseParams { seed, ..Default::default() },
			)
			.unwrap();
			areas.push(params.office_area_target);
		}
		let min = areas.iter().cloned().fold(f32::INFINITY, f32::min);
		let max = areas.iter().cloned().fold(f32::NEG_INFINITY, f32::max);
		assert!(
			max - min > 0.25,
			"office_area_target should vary across seeds (min={min}, max={max})"
		);

		let mut packed_areas = Vec::new();
		for seed in [1i32, 8, 21, 34, 55, 89] {
			let params = MiniMartParameterized::sample(
				&confines,
				NoiseParams { seed, ..Default::default() },
			)
			.unwrap();
			if let Ok(plan) = MiniMartPlan::from_parameterized(params, &confines) {
				packed_areas.push(aabb2_area(aabb3_to_plan(&plan.packed.office, PlanAxes::XZ)));
			}
		}
		assert!(packed_areas.len() >= 3);
	}

	#[test]
	fn mini_mart_fits_playground_demo_seeds() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("demo_bites_door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.4, 0.0, -0.25),
				Vec3::new(5.88, 2.304, 0.25),
			)),
		);
		openings.insert(
			OpeningId::new("demo_bites_door_b"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(8.12, 0.0, -0.25),
				Vec3::new(13.6, 2.304, 0.25),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(14.0, 3.2, 12.0)),
			0.0,
			openings,
		);
		for seed in [11i32, 21, 42] {
			MiniMart::fit_to_confines(&confines, NoiseParams { seed, ..Default::default() })
				.unwrap_or_else(|e| panic!("seed {seed} failed: {e}"));
		}
	}
}
