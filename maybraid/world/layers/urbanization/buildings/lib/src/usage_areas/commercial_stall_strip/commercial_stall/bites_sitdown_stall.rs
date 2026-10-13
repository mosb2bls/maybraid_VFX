//! Sit-down food stall: counters + passage-connected seating + kitchen.
//!
//! **Semantically:** same as bites, plus seating that reads as “at the door”
//! (shared border with a passage long face).
//!
//! **Programmatically:** compose bites counter packing, then staged seating
//! (seed on passage face → grow with kitchen reserve) → kitchen grow-into.
//! Soft-fail ([`FitError::TooSmall`]) if any region cannot meet mins
//! (seating/kitchen ≥1×1; seating ↔ passage contact ≥1 m).

pub mod parameterized;

pub use parameterized::{BitesSitdownParameterized, BitesSitdownPlan};

use building_components::furniture::{FurnitureUsage, FurnitureUsageNode};
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::furniture_util::{furniture_usage_fill, FurnitureUsageFill};

use super::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct BitesSitdownStall {
	pub stall_type: LabelNode,
	pub bites_counters: Vec<FurnitureUsageFill>,
	pub bites_kitchen: FurnitureUsageFill,
	pub bites_seating_area: LabelNode,
	pub bites_seating: FurnitureUsageFill,
}

impl BitesSitdownStall {
	pub fn from_plan(plan: BitesSitdownPlan, confines: &Confines, _noise: NoiseParams) -> Self {
		let style = plan.parameterized.style();
		let host = &confines.bounds;
		let bites_counters = plan
			.counter_aabbs
			.iter()
			.map(|aabb| {
				furniture_usage_fill(
					style,
					"BitesCounter",
					FurnitureUsage::BitesCounter,
					aabb,
					host,
					confines.roll,
				)
			})
			.collect();
		let bites_seating = furniture_usage_fill(
			LabelStyle::Green,
			"BitesSeating",
			FurnitureUsage::BitesSeating,
			&plan.seating_aabb,
			host,
			confines.roll,
		);
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Yellow,
				"BitesSitdownStall",
				&confines.bounds,
				confines.roll,
			),
			bites_counters,
			bites_kitchen: furniture_usage_fill(
				LabelStyle::Orange,
				"BitesKitchen",
				FurnitureUsage::BitesKitchen,
				&plan.kitchen_aabb,
				host,
				confines.roll,
			),
			bites_seating_area: label_filling_aabb(
				LabelStyle::Green,
				"BitesSeatingArea",
				&plan.seating_aabb,
				confines.roll,
			),
			bites_seating,
		}
	}
}

impl Fit for BitesSitdownStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = BitesSitdownParameterized::sample(confines, noise)?;
		let plan = BitesSitdownPlan::from_parameterized(params, confines)?;
		Ok((Self::from_plan(plan, confines, noise), FillableRegions::empty()))
	}
}

impl BuildingComponents for BitesSitdownStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone()];
		labels.extend(self.bites_counters.iter().map(|fill| fill.label.clone()));
		labels.push(self.bites_kitchen.label.clone());
		labels.push(self.bites_seating_area.clone());
		labels.push(self.bites_seating.label.clone());
		Layers::from_free(labels)
	}

	fn furniture_usage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureUsageNode> {
		let mut out: Vec<_> = self.bites_counters.iter().map(|fill| fill.usage.clone()).collect();
		out.push(self.bites_kitchen.usage.clone());
		out.push(self.bites_seating.usage.clone());
		Layers::from_free(out)
	}
}

#[cfg(test)]
mod tests {
	use super::super::bites_stall::BitesStallParameterized;
	use super::super::stall_layout::bites::BITES_SEATING_FACE_CONTACT;
	use super::super::stall_layout::{BitesPassageSpec, EligibleBitesPassage};
	use super::*;
	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::{
		aabb2_area, aabb3_to_plan, OptionalFaceBand, PlanAxes, PlanOpeningFace,
	};

	fn roomy_south() -> Confines {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door_a"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.5, 0.0, -0.2),
				Vec3::new(3.5, 2.2, 0.2),
			)),
		);
		openings.insert(
			OpeningId::new("door_b"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(6.0, 0.0, -0.2),
				Vec3::new(9.0, 2.2, 0.2),
			)),
		);
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(12.0, 3.2, 8.0)), 0.0, openings)
	}

	fn roomy_params(seating_area_target: f32) -> BitesSitdownParameterized {
		let confines = roomy_south();
		let eligible = EligibleBitesPassage::collect(&confines);
		BitesSitdownParameterized {
			base: BitesStallParameterized {
				style: LabelStyle::Cyan,
				passages: eligible
					.into_iter()
					.map(|passage| BitesPassageSpec {
						passage,
						counter: OptionalFaceBand {
							place: true,
							along: 1.5,
							depth: 0.8,
							along_t: 0.0,
						},
					})
					.collect(),
			},
			seating_area_target,
			kitchen_area_reserve: 12.0,
			seating_seed_depth: 1.5,
			seating_along_t: 0.5,
		}
	}

	#[test]
	fn sitdown_emits_counters_seating_kitchen() {
		let confines = roomy_south();
		let plan = BitesSitdownPlan::from_parameterized(roomy_params(35.0), &confines).unwrap();
		let stall = BitesSitdownStall::from_plan(plan, &confines, NoiseParams::default());
		assert!(!stall.bites_counters.is_empty());
		assert!(stall
			.bites_counters
			.iter()
			.all(|fill| { fill.usage.kind == building_components::FurnitureUsage::BitesCounter }));
		assert_eq!(
			stall.bites_kitchen.usage.kind,
			building_components::FurnitureUsage::BitesKitchen
		);
		assert_eq!(
			stall.bites_seating.usage.kind,
			building_components::FurnitureUsage::BitesSeating
		);
		assert_eq!(stall.stall_type.text, "BitesSitdownStall");
		assert_eq!(stall.bites_seating_area.text, "BitesSeatingArea");
		assert!(stall.bites_seating_area.placement.scale.x >= 1.0);
		assert!(stall.bites_seating_area.placement.scale.z >= 1.0);
		assert!(stall.bites_kitchen.label.placement.scale.x >= 1.0);
		assert!(stall.bites_kitchen.label.placement.scale.z >= 1.0);
	}

	#[test]
	fn seating_shares_one_meter_opening_face() {
		let confines = roomy_south();
		let plan = BitesSitdownPlan::from_parameterized(roomy_params(35.0), &confines).unwrap();
		let seating = plan.seating_aabb;
		let seat2 = aabb3_to_plan(&seating, PlanAxes::XZ);
		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		let ok = confines.openings.iter().any(|(_, o)| {
			if !matches!(o.label, OpeningLabel::Passage) {
				return false;
			}
			let Some(face) =
				PlanOpeningFace::from_passage(host, aabb3_to_plan(&o.bounds, PlanAxes::XZ))
			else {
				return false;
			};
			face.contacts(seat2, BITES_SEATING_FACE_CONTACT)
		});
		assert!(ok, "seating must share ≥1m with a passage long face");
	}

	#[test]
	fn seating_grows_toward_area_target() {
		let confines = roomy_south();
		let target = 40.0_f32;
		let plan = BitesSitdownPlan::from_parameterized(roomy_params(target), &confines).unwrap();
		let seat_area = aabb2_area(aabb3_to_plan(&plan.seating_aabb, PlanAxes::XZ));
		let kit_area = aabb2_area(aabb3_to_plan(&plan.kitchen_aabb, PlanAxes::XZ));
		assert!(
			seat_area + 1.0 >= target.min(30.0),
			"seating area {seat_area} should approach target {target}"
		);
		assert!(
			seat_area + 1.0 >= kit_area * 0.45,
			"seating {seat_area} dominated by kitchen {kit_area}"
		);
	}

	#[test]
	fn sample_fit_works() {
		let (stall, _) =
			BitesSitdownStall::fit_to_confines(&roomy_south(), NoiseParams::default()).unwrap();
		assert!(!stall.bites_counters.is_empty());
	}

	#[test]
	fn shallow_fails_without_seating_and_kitchen() {
		let mut openings = Openings::new();
		openings.insert(
			OpeningId::new("door"),
			Opening::passage(Aabb3d::from_min_max(
				Vec3::new(0.2, 0.0, -0.2),
				Vec3::new(5.8, 2.2, 0.2),
			)),
		);
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 2.4)),
			0.0,
			openings,
		);
		assert!(matches!(
			BitesSitdownStall::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}
