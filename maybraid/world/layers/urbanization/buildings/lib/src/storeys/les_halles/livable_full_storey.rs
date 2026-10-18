//! Les Halles livable full storey: ring floor plan + lengthwise gallery bays.
//!
//! Reuses [`LesHallesFloorPlan`] (same shell / strip residuals as the commercial
//! Full\*). Gallery strips are painted with [`LesHallesLivableUsage`].
//!
//! Deeper galleries come from [`LesHallesParameterized::sample_livable`]. Prefer
//! larger footprints than commercial demos (playground default `72,4,54`).

use building_components::furniture::FurnitureNode;
use building_components::joints::JointNode;
use building_components::labels::LabelNode;
use building_components::panels::PanelNode;
use building_components::{BuildingComponents, BuildingStructuralLodProbe, Layers};
use lod::scene::LodSceneLevel;
use procedural_common::NoiseParams;

use crate::fit::{Confines, FillableRegions, Fit, FitError};
use crate::paneling::clipped_rectangular_strip::ClippedRectangularStrip;
use crate::usage_areas::rectangular_livable_area::RectangularLivableArea;

use super::floor_plan::LesHallesFloorPlan;
use super::parameterized::LesHallesParameterized;
use super::usage_plan::{LesHallesLivableUsage, LesHallesUsagePlan};

/// Full Les Halles storey with residential gallery fills.
#[derive(Debug, Clone, PartialEq)]
pub struct LesHallesLivableFullStorey {
	pub floor_plan: LesHallesFloorPlan,
	pub usage: LesHallesLivableUsage,
}

impl LesHallesLivableFullStorey {
	/// One RLA per filled gallery bay.
	pub fn areas(&self) -> &[RectangularLivableArea] {
		&self.usage.areas
	}

	/// Within-strip bay cuts + noisy cross-strip shared edges.
	pub fn party_walls(&self) -> &[ClippedRectangularStrip] {
		&self.usage.party_walls
	}

	/// Wrap an already-fitted floor plan and paint livable usage.
	pub fn from_floor_plan(
		floor_plan: LesHallesFloorPlan,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let regions = floor_plan.fillable_regions();
		let (usage, residual) = LesHallesLivableUsage::paint(regions, noise)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl Fit for LesHallesLivableFullStorey {
	fn fit_to_confines(
		confines: &Confines,
		noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let params = LesHallesParameterized::sample_livable(confines, noise)?;
		let (floor_plan, regions) = LesHallesFloorPlan::from_parameterized(params, confines)?;
		let (usage, residual) = LesHallesLivableUsage::paint(regions, noise)?;
		Ok((Self { floor_plan, usage }, residual))
	}
}

impl BuildingComponents for LesHallesLivableFullStorey {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		let mut out = self.floor_plan.panel_nodes_for_level(level);
		out.extend(self.usage.panel_nodes_for_level(level));
		out
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		let mut out = self.floor_plan.joint_nodes_for_level(level);
		out.extend(self.usage.joint_nodes_for_level(level));
		out
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

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		Some(self.floor_plan.structural_probe(None))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::fit::{FillRegion, SpaceKind};
	use crate::usage_areas::livable_apartment::INTERNAL_WALLS_LAYER;
	use crate::usage_areas::rectangular_livable_area::{RectAreaRoom, RectLivableStrategy};
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;
	use building_components::{BuildingComponents, Layer};
	use lod::scene::LodSceneLevel;
	use procedural_common::NoiseParams;

	fn large_bounds() -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-36.0, 0.0, -27.0), Vec3::new(36.0, 4.0, 27.0))
	}

	fn storey_with_shafts(seed: i32) -> (LesHallesLivableFullStorey, FillableRegions) {
		let bounds = large_bounds();
		let empty = Confines::from_bounds(bounds);
		let noise = NoiseParams { seed, ..NoiseParams::default() };
		let params = LesHallesParameterized::sample_livable(&empty, noise).unwrap();
		let openings = LesHallesFloorPlan::shaft_requests_for_all_slots(&params, &empty);
		let confines = Confines::new(bounds, 0.0, openings);
		let (plan, _) = LesHallesFloorPlan::from_parameterized(params, &confines).unwrap();
		LesHallesLivableFullStorey::from_floor_plan(plan, noise).unwrap()
	}

	#[test]
	fn livable_full_storey_fills_external_strips() {
		let (storey, regions) = storey_with_shafts(1337);
		assert!(!storey.areas().is_empty());
		assert!(regions.within.iter().all(|r| r.kind != SpaceKind::ExternalSpace));
		assert!(regions.within.iter().any(|r| r.kind == SpaceKind::Walkway));
		assert_eq!(regions.atop.len(), 1);
		assert!(storey.floor_plan.gallery.wall_count() >= 4);
		assert!(!storey.panel_nodes_for_level(LodSceneLevel::High).is_empty());
		assert!(!storey.label_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn livable_bays_skip_spine_hall() {
		let (storey, _) = storey_with_shafts(7);
		assert!(storey.areas().iter().any(|a| !a.rooms.is_empty()));
		assert!(
			storey.areas().iter().all(|a| a.plan.chosen != RectLivableStrategy::SpineHall),
			"Les Halles livable path must not choose SpineHall"
		);
	}

	#[test]
	fn bedrooms_appear_on_typical_seed() {
		let (storey, _) = storey_with_shafts(1337);
		let bedrooms = storey
			.areas()
			.iter()
			.flat_map(|a| a.rooms.iter())
			.filter(|r| matches!(r, RectAreaRoom::Bedroom(_)))
			.count();
		assert!(
			bedrooms >= 2,
			"expected several bedrooms after SingleClosed/bedroom-first program; got {bedrooms}"
		);
	}

	#[test]
	fn internal_walls_only_on_high_structural_band() {
		let (storey, _) = storey_with_shafts(1337);
		let high = storey.panel_nodes_for_level(LodSceneLevel::High);
		assert!(
			high.labeled.contains_key(&Layer::new(INTERNAL_WALLS_LAYER)),
			"High should keep internal_walls"
		);
		let mid = storey.panel_nodes_for_level(LodSceneLevel::Medium);
		assert!(
			!mid.labeled.contains_key(&Layer::new(INTERNAL_WALLS_LAYER)),
			"Medium should drop internal_walls"
		);
	}

	#[test]
	fn too_small_strip_left_as_external_residual() {
		let tiny = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(2.0, 3.0, 2.0),
		));
		let regions = FillableRegions {
			within: vec![FillRegion::new(SpaceKind::ExternalSpace, tiny)],
			atop: Vec::new(),
		};
		let (usage, residual) =
			LesHallesLivableUsage::paint(regions, NoiseParams::default()).unwrap();
		assert!(usage.areas.is_empty());
		assert!(residual.within.iter().any(|r| r.kind == SpaceKind::ExternalSpace));
	}

	#[test]
	fn residual_gallery_gets_a_chest() {
		// Tall enough to walk, too short on Y for RLA (needs height >= 2).
		let leftover = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(6.0, 1.8, 4.0),
		));
		let regions = FillableRegions {
			within: vec![FillRegion::new(SpaceKind::ExternalSpace, leftover)],
			atop: Vec::new(),
		};
		let (usage, _) = LesHallesLivableUsage::paint(regions, NoiseParams::default()).unwrap();
		assert!(usage.areas.is_empty());
		assert_eq!(usage.residual_chests.len(), 1);
		assert!(!usage.furniture_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}

	#[test]
	fn strip_without_passages_still_gets_an_open_room() {
		let open = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(0.0, 0.0, 0.0),
			Vec3::new(8.0, 3.2, 5.0),
		));
		let regions = FillableRegions {
			within: vec![FillRegion::new(SpaceKind::ExternalSpace, open)],
			atop: Vec::new(),
		};
		let (usage, residual) =
			LesHallesLivableUsage::paint(regions, NoiseParams::default()).unwrap();
		assert_eq!(usage.areas.len(), 1);
		assert!(residual.within.iter().all(|r| r.kind != SpaceKind::ExternalSpace));
		assert!(!usage.furniture_nodes_for_level(LodSceneLevel::High).flatten().is_empty());
	}
}
