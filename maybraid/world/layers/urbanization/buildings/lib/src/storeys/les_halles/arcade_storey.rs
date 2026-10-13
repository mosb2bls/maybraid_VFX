//! Ground-floor arcade: open gallery ring with midspan breezeways, no inner walls.

use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};

use super::floor_plan::{LesHallesFloorPlan, LesHallesOpeningProgram};
use super::parameterized::LesHallesParameterized;
use super::usage_plan::LesHallesArcadeUsage;

/// Full Les Halles ground storey: ring shell plus empty arcade usage.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesArcadeStorey {
	pub floor_plan: LesHallesFloorPlan,
	pub usage: LesHallesArcadeUsage,
}

impl LesHallesArcadeStorey {
	/// Wrap an already-fitted arcade floor plan.
	pub fn from_floor_plan(
		floor_plan: LesHallesFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		let keep_outs = LesHallesArcadeUsage::pillar_keep_outs_of(&floor_plan);
		let (usage, residual) = LesHallesArcadeUsage::paint_avoiding(regions, noise, &keep_outs)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl Fit for LesHallesArcadeStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LesHallesParameterized::sample_monotower(confines, noise)
			.or_else(|_| LesHallesParameterized::sample(confines, noise))?;
		let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized_with(
			params,
			confines,
			crate::shells::rect_ring_floor::RectRingFloorSlab::None,
			LesHallesOpeningProgram::GroundArcade,
		)?;
		let keep_outs = LesHallesArcadeUsage::pillar_keep_outs_of(&floor_plan);
		let (usage, residual) = LesHallesArcadeUsage::paint_avoiding(regions, noise, &keep_outs)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl BuildingComponents for LesHallesArcadeStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.floor_plan.panel_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.floor_plan.joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		self.usage.furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(
		&self,
		level: LodSceneLevel,
	) -> Layers<building_components::FurnitureUsageNode> {
		self.usage.furniture_usage_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.usage.label_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use procedural_common::NoiseParams;

	use crate::fit::SpaceKind;
	use crate::openings::OpeningLabel;

	#[test]
	fn arcade_storey_leaves_gallery_unfilled() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-36.0, 0.0, -27.0),
			Vec3::new(36.0, 4.0, 27.0),
		));
		let (storey, residual) =
			LesHallesArcadeStorey::fit_to_confines(&confines, NoiseParams::default()).unwrap();
		assert!(storey.floor_plan.openings.iter().any(|(id, o)| {
			id.as_str().contains("outer_breezeway") && matches!(o.label, OpeningLabel::Passage)
		}));
		assert_eq!(storey.floor_plan.gallery.wall_count(), 4);
		assert!(storey.floor_plan.arcade_pillars.iter().any(|l| !l.is_empty()));
		assert!(residual.within.iter().any(|r| r.kind == SpaceKind::ExternalSpace));
		assert!(!storey.usage.is_empty(), "open arcade leftovers should still get residual chests");
	}
}
