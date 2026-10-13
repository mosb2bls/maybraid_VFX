//! Souvenir / trinket stall: keep the entry clear; merchandise on the walls.
//!
//! **Semantically:** open floor for browsing with displays only along boundaries
//! (no free-standing island racks in this placeholder).
//!
//! **Programmatically:** passage clearance bands, then sampled + opportunistic
//! wall bands (`OptionalFaceBand` / free segments). Soft-fail without a
//! passage or if no display ≥ mins can place.

pub mod parameterized;

pub use parameterized::{KnickKnackStallParameterized, KnickKnackStallPlan};

use bevy_math::bounding::Aabb3d;
use building_components::{BuildingComponents, LabelNode, LabelStyle, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::label_util::label_filling_aabb;

#[derive(Debug, Clone, PartialEq)]
pub struct KnickKnackStall {
	pub stall_type: LabelNode,
	pub display_bounds: Vec<Aabb3d>,
	pub displays: Vec<LabelNode>,
}

impl KnickKnackStall {
	pub fn from_plan(plan: KnickKnackStallPlan, confines: &Confines) -> Self {
		let style = plan.parameterized.style;
		let display_bounds = plan.packed.displays.clone();
		let displays = display_bounds
			.iter()
			.map(|aabb| label_filling_aabb(style, "KnickKnackDisplay", aabb, confines.roll))
			.collect();
		Self {
			stall_type: label_filling_aabb(
				LabelStyle::Magenta,
				"KnickKnackStall",
				&confines.bounds,
				confines.roll,
			),
			display_bounds,
			displays,
		}
	}
}

impl Fit for KnickKnackStall {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = KnickKnackStallParameterized::sample(confines, noise)?;
		let plan = KnickKnackStallPlan::from_parameterized(params, confines)?;
		let stall = Self::from_plan(plan, confines);
		Ok((stall, FillableRegions::empty()))
	}
}

impl BuildingComponents for KnickKnackStall {
	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		let mut labels = vec![self.stall_type.clone()];
		labels.extend(self.displays.iter().cloned());
		Layers::from_free(labels)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::{aabb3_to_plan, PlanAxes};

	use crate::openings::{Opening, OpeningId, Openings};

	use super::super::stall_layout::knick_knack::KNICK_KNACK_DISPLAY_DEPTH_MIN;

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
	fn knick_knack_fits_with_wall_displays() {
		let confines = roomy_south();
		let (stall, regions) = KnickKnackStall::fit_to_confines(
			&confines,
			NoiseParams { seed: 3, ..Default::default() },
		)
		.unwrap();
		assert_eq!(stall.stall_type.text.as_str(), "KnickKnackStall");
		assert!(!stall.displays.is_empty());
		assert!(regions.within.is_empty());

		let host = aabb3_to_plan(&confines.bounds, PlanAxes::XZ);
		for aabb in &stall.display_bounds {
			let plan = aabb3_to_plan(aabb, PlanAxes::XZ);
			let shallow = (plan.max.x - plan.min.x).min(plan.max.y - plan.min.y);
			assert!(
				shallow + 1e-3 >= KNICK_KNACK_DISPLAY_DEPTH_MIN * 0.5,
				"display should be a shallow wall band"
			);
			let on_boundary = (plan.min.x - host.min.x).abs() < 1e-2
				|| (plan.max.x - host.max.x).abs() < 1e-2
				|| (plan.min.y - host.min.y).abs() < 1e-2
				|| (plan.max.y - host.max.y).abs() < 1e-2;
			assert!(on_boundary, "display must touch a host wall");
		}
	}

	#[test]
	fn knick_knack_soft_fails_without_passage() {
		let confines = Confines::new(
			Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(10.0, 3.2, 8.0)),
			0.0,
			Openings::new(),
		);
		assert!(matches!(
			KnickKnackStall::fit_to_confines(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}
}
