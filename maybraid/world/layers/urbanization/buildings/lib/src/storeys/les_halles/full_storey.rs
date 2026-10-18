//! Les Halles full storey: floor plan plus commercial gallery strip fills.
//!
//! The ring shell lives on [`LesHallesFloorPlan`]. This type fits that plan and
//! paints [`LesHallesCommercialUsage`] onto [`SpaceKind::ExternalSpace`] strips.
//! Residual walkways / shafts remain in [`FillableRegions::within`].

use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::usage_areas::CommercialStallStrip;

use super::floor_plan::LesHallesFloorPlan;
use super::usage_plan::{LesHallesCommercialUsage, LesHallesUsagePlan};

/// Full Les Halles storey: shell on [`Self::floor_plan`] plus commercial usage.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesFullStorey {
	pub floor_plan: LesHallesFloorPlan,
	pub usage: LesHallesCommercialUsage,
}

impl LesHallesFullStorey {
	/// Gallery stall strips (same as [`Self::usage`].stall_strips).
	pub fn stall_strips(&self) -> &[CommercialStallStrip] {
		&self.usage.stall_strips
	}

	/// Wrap an already-fitted floor plan and paint commercial usage.
	pub fn from_floor_plan(
		floor_plan: LesHallesFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		let (usage, residual) = LesHallesCommercialUsage::paint(regions, noise)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl Fit for LesHallesFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let (floor_plan, regions) = LesHallesFloorPlan::fit_to_confines(confines, noise)?;
		let (usage, residual) = LesHallesCommercialUsage::paint(regions, noise)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl BuildingComponents for LesHallesFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		out.extend(self.usage.panel_nodes_for_level(level));
		out
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
	use building_components::BuildingComponents;
	use lod::scene::LodSceneLevel;
	use procedural_common::NoiseParams;

	use crate::fit::SpaceKind;
	use crate::storeys::les_halles::{LesHallesFloorPlan, LesHallesParameterized};

	#[test]
	fn full_storey_fills_external_strips_with_labels() {
		let bounds = Aabb3d::from_min_max(Vec3::new(-24.0, 0.0, -18.0), Vec3::new(24.0, 4.0, 18.0));
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams::default();
		let params = LesHallesParameterized::sample(&empty, noise).unwrap();
		let openings = LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = Confines::new(bounds, 0.0, openings);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		let (storey, regions) = LesHallesFullStorey::from_floor_plan(plan, noise).unwrap();
		assert!(!storey.stall_strips().is_empty());
		assert!(regions.within.iter().all(|r| r.kind != SpaceKind::ExternalSpace));
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Walkway));
		assert_eq!(regions.atop.len(), 1);
		assert!(storey.floor_plan.gallery.wall_count() >= 4);
		assert!(!storey.label_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
		let panels = storey.panel_nodes_for_level(LodSceneLevel::High);
		assert!(!panels.is_empty());
	}

	#[test]
	fn lounge_or_leftover_gallery_gets_a_chest() {
		let leftover = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(6.0, 3.2, 4.0),
		));
		let regions = crate::fit::FillableRegions {
			within: vec![crate::fit::FillRegion::new(SpaceKind::ExternalSpace, leftover)],
			atop: Vec::new(),
		};
		let (usage, _) = LesHallesCommercialUsage::paint(regions, NoiseParams::default()).unwrap();
		assert!(!usage.stall_strips.is_empty() || !usage.residual_chests.is_empty());
		assert!(
			!usage.furniture_nodes_for_level(LodSceneLevel::High).flatten().is_empty()
				|| !usage.furniture_usage_nodes_for_level(LodSceneLevel::High).flatten().is_empty(),
			"gallery leftover should get stall usage regions, chairs, or a residual chest"
		);
	}

	#[test]
	fn leftover_walkways_allocate_occasional_chests() {
		let walk =
			Confines::from_bounds(Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(6.0, 3.0, 4.0)));
		let regions = crate::fit::FillableRegions {
			within: vec![crate::fit::FillRegion::new(SpaceKind::Walkway, walk)],
			atop: Vec::new(),
		};
		let mut any = false;
		for seed in 0..24 {
			let (usage, _) = LesHallesCommercialUsage::paint(
				regions.clone(),
				NoiseParams { seed, ..NoiseParams::default() },
			)
			.unwrap();
			if usage.residual_chests.iter().any(|fill| {
				fill.furniture.geometry == building_components::FurnitureGeometry::Chest
			}) {
				any = true;
				break;
			}
		}
		assert!(any, "walkway residuals should feed occasional chests into generate");
	}
}
