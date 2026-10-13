//! Public restroom: large walled toilet block + sinks by the stalls door.
//!
//! **Semantically:** customer entry → sink strip → door into toilet stalls that
//! fill most of the bay; stalls enclosure only adds walls on non-boundary sides.
//!
//! **Programmatically:** reserve a door-side strip (`2×PASSAGE_CLEARANCE + sink
//! depth`) while packing stalls ≥2×2; enclose + author stalls door; pack sinks
//! with [`pack_abutting_clearance`] against the door keep-out. Soft-fail on
//! undersized bays.

pub mod parameterized;

pub use parameterized::{PublicRestroomParameterized, PublicRestroomPlan};

use bevy_math::bounding::Aabb3d;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillRegion, FillableRegions, Fit, FitError, SpaceKind};
use crate::openings::{Opening, OpeningId, Openings};
use crate::paneling::Rectangle;

use super::label_util::label_filling_aabb;
use super::stall_layout::public_restroom::RestroomStallsDoor;

#[derive(Debug, Clone, PartialEq)]
pub struct PublicRestroom {
	pub stall_type: LabelNode,
	pub stall_walls: Vec<Rectangle>,
	pub stalls_bounds: Aabb3d,
	pub toilet_stalls: LabelNode,
	pub sink_bounds: Vec<Aabb3d>,
	pub sinks: Vec<LabelNode>,
	pub stalls_door_id: OpeningId,
	pub stalls_door: Opening,
}

impl PublicRestroom {
	pub fn from_plan(plan: PublicRestroomPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let sink_bounds = plan.packed.sinks.clone();
		let sinks = sink_bounds
			.iter()
			.map(|aabb| {
				label_filling_aabb(LabelStyle::Cyan, "PublicRestroomSinks", aabb, confines.roll)
			})
			.collect();
		let RestroomStallsDoor { id, opening } = plan.packed.stalls_door.clone();
		let stalls_bounds = plan.packed.stalls;
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Gray,
				"PublicRestroom",
				&confines.bounds,
				confines.roll,
			),
			stall_walls: plan.packed.stall_walls,
			stalls_bounds,
			toilet_stalls: label_filling_aabb(style, "ToiletStalls", &stalls_bounds, confines.roll),
			sink_bounds,
			sinks,
			stalls_door_id: id,
			stalls_door: opening,
		}
	}

	pub fn stalls_fill_region(&self, roll: f32) -> FillRegion {
		let mut openings = Openings::new();
		openings.insert(self.stalls_door_id.clone(), self.stalls_door.clone());
		FillRegion::new(SpaceKind::InternalSpace, Confines::new(self.stalls_bounds, roll, openings))
	}
}

impl Fit for PublicRestroom {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = PublicRestroomParameterized::sample(confines, noise)?;
		let plan = PublicRestroomPlan::from_parameterized(params, confines)?;
		let stall = Self::from_plan(plan, confines);
		let regions = FillableRegions {
			within: vec![stall.stalls_fill_region(confines.roll)],
			atop: Vec::new(),
		};
		Ok((stall, regions))
	}
}

impl BuildingComponents for PublicRestroom {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = Layers::new();
		for wall in &self.stall_walls {
			out.extend(wall.panel_nodes_for_level(level));
		}
		out
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone(), self.toilet_stalls.clone()];
		labels.extend(self.sinks.iter().cloned());
		Layers::from_free(labels)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::Vec3;
	use procedural_common::{aabb3_to_plan, PlanAxes};

	use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};

	use super::super::stall_layout::public_restroom::{
		RESTROOM_SINK_MIN, RESTROOM_STALLS_MIN, SCOPE,
	};

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
	fn restroom_fits_walled_stalls_door_and_sinks() {
		use bevy_math::bounding::Aabb2d;
		use bevy_math::Vec2;
		use procedural_common::{aabb2_area, intersects_aabb2};

		let confines = roomy_south();
		let (stall, regions) = PublicRestroom::fit_to_confines(
			&confines,
			NoiseParams { seed: 3, ..Default::default() },
		)
		.unwrap();
		assert_eq!(stall.stall_type.text.as_str(), "PublicRestroom");
		assert!(!stall.sinks.is_empty());
		assert!(!stall.stall_walls.is_empty());
		assert_eq!(stall.stalls_door_id, OpeningId::scoped(SCOPE, "stalls_door", "0"));
		assert!(matches!(stall.stalls_door.label, OpeningLabel::Passage));
		assert_eq!(regions.within.len(), 1);
		assert!(regions.within[0].confines.openings.get(&stall.stalls_door_id).is_some());

		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		let stalls = aabb3_to_plan(&stall.stalls_bounds, PlanAxes::XZ);
		assert!(stalls.max.x - stalls.min.x + 1e-3 >= RESTROOM_STALLS_MIN);
		assert!(stalls.max.y - stalls.min.y + 1e-3 >= RESTROOM_STALLS_MIN);
		// Stalls should dominate the plan.
		assert!(aabb2_area(stalls) / aabb2_area(host).max(1.0) > 0.45);

		let door = aabb3_to_plan(&stall.stalls_door.bounds, PlanAxes::XZ);
		let door_pad = Aabb2d {
			min: Vec2::new(door.min.x - 1.05, door.min.y - 1.05),
			max: Vec2::new(door.max.x + 1.05, door.max.y + 1.05),
		};
		let mut sink_near_door = false;
		for aabb in &stall.sink_bounds {
			let plan = aabb3_to_plan(aabb, PlanAxes::XZ);
			assert!(plan.max.x - plan.min.x + 1e-3 >= RESTROOM_SINK_MIN);
			assert!(plan.max.y - plan.min.y + 1e-3 >= RESTROOM_SINK_MIN);
			if intersects_aabb2(plan, door_pad) {
				sink_near_door = true;
			}
		}
		assert!(sink_near_door, "stalls door should connect into the sinks area");
	}

	#[test]
	fn restroom_soft_fails_tiny_bay() {
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
			PublicRestroom::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}

	#[derive(Clone, Copy)]
	enum DemoDoor {
		South,
		North,
		East,
		West,
	}

	fn demo_doors(extent: Vec3, side: DemoDoor) -> Confines {
		let along = match side {
			DemoDoor::South | DemoDoor::North => extent.x,
			DemoDoor::East | DemoDoor::West => extent.z,
		};
		let door_h = (extent.y * 0.72).clamp(2.0, extent.y.max(2.0));
		let band = 0.25_f32;
		let mut openings = Openings::new();
		let mk = |a0: f32, a1: f32| -> Aabb3d {
			match side {
				DemoDoor::South => {
					Aabb3d::from_min_max(Vec3::new(a0, 0.0, -band), Vec3::new(a1, door_h, band))
				}
				DemoDoor::North => Aabb3d::from_min_max(
					Vec3::new(a0, 0.0, extent.z - band),
					Vec3::new(a1, door_h, extent.z + band),
				),
				DemoDoor::East => Aabb3d::from_min_max(
					Vec3::new(extent.x - band, 0.0, a0),
					Vec3::new(extent.x + band, door_h, a1),
				),
				DemoDoor::West => {
					Aabb3d::from_min_max(Vec3::new(-band, 0.0, a0), Vec3::new(band, door_h, a1))
				}
			}
		};
		if along >= 6.0 {
			openings.insert(
				OpeningId::new("demo_bites_door_a"),
				Opening::passage(mk(0.4, (along * 0.42).max(2.5))),
			);
			openings.insert(
				OpeningId::new("demo_bites_door_b"),
				Opening::passage(mk(along * 0.58, (along - 0.4).max(along * 0.58 + 2.5))),
			);
		} else {
			openings.insert(
				OpeningId::new("demo_bites_door"),
				Opening::passage(mk(0.3, (along - 0.3).max(2.2))),
			);
		}
		Confines::new(Aabb3d::from_min_max(Vec3::ZERO, extent), 0.0, openings)
	}

	#[test]
	fn restroom_fits_gallery_example_seeds() {
		// Mirrors playground `public_restroom_examples_specs`.
		let cases = [
			(Vec3::new(10.0, 3.2, 8.0), 3, DemoDoor::South),
			(Vec3::new(12.0, 3.2, 7.0), 11, DemoDoor::South),
			(Vec3::new(8.0, 3.2, 10.0), 42, DemoDoor::East),
			(Vec3::new(14.0, 3.2, 8.0), 7, DemoDoor::South),
			(Vec3::new(10.0, 3.2, 9.0), 21, DemoDoor::North),
			(Vec3::new(11.0, 3.2, 8.0), 55, DemoDoor::West),
		];
		for (extent, seed, side) in cases {
			let confines = demo_doors(extent, side);
			let (stall, _) = PublicRestroom::fit_to_confines(
				&confines,
				NoiseParams { seed, ..Default::default() },
			)
			.unwrap_or_else(|e| panic!("gallery ({extent:?} seed={seed}) failed: {e}"));
			assert!(!stall.sink_bounds.is_empty());
		}
	}
}
