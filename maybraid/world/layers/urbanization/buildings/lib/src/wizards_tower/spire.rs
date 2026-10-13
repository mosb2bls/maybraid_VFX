//! Central circular spire region of a Wizard's Tower floor.
//!
//! Geometry: four 90° core wall arcs only for now. Stairs / roofs / floor fill
//! inside the spire hole are omitted (empty scenes).

use bevy_math::Vec3;
use building_components::partitions::{Partition, PartitionNode};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;

use crate::CellConstraints;

/// Spire cell with exclusive boundary rights in its write bounds.
#[derive(Debug, Clone, PartialEq)]
pub struct WizardsTowerSpire {
	pub constraints: CellConstraints,
	pub core_walls: [PartitionNode; 4],
}

impl WizardsTowerSpire {
	/// Build from this spire's subsetted constraints.
	pub fn new(constraints: CellConstraints) -> Self {
		let center = (constraints.aabb.min + constraints.aabb.max) * 0.5;
		let center_xz = Vec3::new(center.x, constraints.aabb.min.y, center.z);
		let extent = constraints.aabb.max - constraints.aabb.min;
		let radius = 0.5 * extent.x.min(extent.z);
		let height = extent.y.max(1e-4);
		let scale = Vec3::new(radius, height, radius);
		Self {
			core_walls: [
				PartitionNode::rough_stone(
					Partition::arc(90.0),
					Placement::new(center_xz, 0.0).with_scale(scale),
				),
				PartitionNode::rough_stone(
					Partition::arc(90.0),
					Placement::new(center_xz, std::f32::consts::FRAC_PI_2).with_scale(scale),
				),
				PartitionNode::rough_stone(
					Partition::arc(90.0),
					Placement::new(center_xz, std::f32::consts::PI).with_scale(scale),
				),
				PartitionNode::rough_stone(
					Partition::arc(90.0),
					Placement::new(center_xz, std::f32::consts::PI + std::f32::consts::FRAC_PI_2)
						.with_scale(scale),
				),
			],
			constraints,
		}
	}
}

impl BuildingComponents for WizardsTowerSpire {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::from_free(self.core_walls.to_vec())
	}
}
