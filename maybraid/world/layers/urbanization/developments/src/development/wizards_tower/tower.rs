//! [`SolitaryWizardsTower`]: the procedural Wizard's Tower fitted to confines.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use building_components::{
	BuildingComponents, DoorNode, FloorNode, FurnitureNode, FurnitureUsageNode, JointNode,
	LabelNode, Layers, PanelNode, PartitionNode, RoofNode, StairNode,
};
use buildings::wizards_tower::WizardsTower;
use buildings::{CellConstraints, Confines, FillableRegions, Fit, FitError};
use lod::scene::LodSceneLevel;
use material_ref::MaterialRef;
use procedural_common::NoiseParams;

use crate::BuildingFootprint;

/// Solitary integration wrapper for the existing procedural Wizard's Tower.
#[derive(Debug, Clone, PartialEq)]
pub struct SolitaryWizardsTower {
	pub bounds: Aabb3d,
	pub tower: WizardsTower,
}

impl SolitaryWizardsTower {
	pub fn with_finish(mut self, wall: MaterialRef, room: MaterialRef) -> Self {
		self.tower = self.tower.with_finish(wall, room);
		self
	}
}

impl Fit for SolitaryWizardsTower {
	fn fit_to_confines(
		confines: &Confines,
		_noise: NoiseParams,
	) -> Result<(Self, FillableRegions), FitError> {
		let footprint = confines.footprint();
		let side = footprint.x.min(footprint.y) * 0.72;
		let height = confines.bounds.max.y - confines.bounds.min.y;
		let storey_height = 4.0;
		let available_floors = (height / storey_height).floor() as u32;
		let available_floors = available_floors.saturating_sub(1);
		if side < 12.0 || available_floors < 10 {
			return Err(FitError::TooSmall { reason: "solitary_wizards_tower" });
		}
		let floor_count = available_floors.min(30);
		let center = confines.center();
		let half = side * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, confines.bounds.min.y, center.z - half),
			Vec3::new(
				center.x + half,
				confines.bounds.min.y + (floor_count + 1) as f32 * storey_height,
				center.z + half,
			),
		);
		let constraints = CellConstraints::cell_owned(bounds);
		let floor_noise = (floor_count - 10) as f32 / 20.0;
		let tower = WizardsTower::new(&constraints, floor_noise);
		Ok((Self { bounds, tower }, FillableRegions { within: Vec::new(), atop: Vec::new() }))
	}
}

impl BuildingFootprint for SolitaryWizardsTower {
	fn footprint_rects(&self) -> Vec<Aabb2d> {
		vec![Aabb2d {
			min: Vec2::new(self.bounds.min.x, self.bounds.min.z),
			max: Vec2::new(self.bounds.max.x, self.bounds.max.z),
		}]
	}
}

impl BuildingComponents for SolitaryWizardsTower {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.tower.panel_nodes_for_level(level)
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		self.tower.partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.tower.floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		self.tower.roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.tower.stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		self.tower.door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.tower.joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		self.tower.furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureUsageNode> {
		self.tower.furniture_usage_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.tower.label_nodes_for_level(level)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn tower_stays_inside_the_requested_vertical_envelope() -> anyhow::Result<()> {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-20.0, 0.0, -20.0),
			Vec3::new(20.0, 80.0, 20.0),
		));
		let (wizard, _) = SolitaryWizardsTower::fit_to_confines(&confines, NoiseParams::default())?;
		assert!(wizard.bounds.max.y <= confines.bounds.max.y + 1e-3);
		Ok(())
	}
}
