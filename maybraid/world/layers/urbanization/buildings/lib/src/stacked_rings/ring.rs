//! One circular wall storey: two 180° arcs scaled to radius and floor height.

use bevy_math::Vec3;
use building_components::partitions::{Partition, PartitionNode};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;

/// A single ring of outer circular walls.
#[derive(Debug, Clone, PartialEq)]
pub struct StackedRing {
	pub base_y: f32,
	pub floor_height: f32,
	pub radius: f32,
	pub outer_walls: [PartitionNode; 2],
}

impl StackedRing {
	/// Place two 180° arcs at `base_y`, scaled to `(radius, floor_height, radius)`.
	pub fn new(base_y: f32, floor_height: f32, radius: f32) -> Self {
		let translation = Vec3::new(0.0, base_y, 0.0);
		let scale = Vec3::new(radius, floor_height, radius);
		Self {
			base_y,
			floor_height,
			radius,
			outer_walls: [
				PartitionNode::rough_stone(
					Partition::arc(180.0),
					Placement::new(translation, 0.0).with_scale(scale),
				),
				PartitionNode::rough_stone(
					Partition::arc(180.0),
					Placement::new(translation, std::f32::consts::PI).with_scale(scale),
				),
			],
		}
	}
}

impl BuildingComponents for StackedRing {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::from_free(self.outer_walls.to_vec())
	}
}
