//! Circular fitted [`ArcSweep`] → solid [`Partition::arc`] kits.

use bevy_math::Vec3;
use building_components::partitions::{Partition, PartitionNode, PartitionStyle};
use building_components::{BuildingComponents, Layers, Placement};
use lod::scene::LodSceneLevel;

/// Fitted circular arc wall body (no portals / noise).
///
/// Emits one or more [`PartitionNode`]s via [`Partition::arc`] kit decomposition.
/// Not the same type as component IR `partitions::ArcSweep { sweep_degrees }` —
/// see module docs on [`crate::arcs`].
#[derive(Debug, Clone, PartialEq)]
pub struct ArcSweep {
	pub center_xz: Vec3,
	pub radius: f32,
	pub storey_height: f32,
	/// Degrees of arc covered (\((0, 360]\)).
	pub sweep_degrees: f32,
	/// World yaw (radians) of the sweep start.
	pub start_yaw: f32,
	pub style: PartitionStyle,
	pub partitions: Vec<PartitionNode>,
}

impl ArcSweep {
	pub fn new(
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		sweep_degrees: f32,
		start_yaw: f32,
		style: PartitionStyle,
	) -> Self {
		let radius = radius.max(1e-4);
		let storey_height = storey_height.max(1e-4);
		let sweep_degrees = sweep_degrees.clamp(1e-2, 360.0);
		let ring_scale = Vec3::new(radius, storey_height, radius);
		let partitions = if sweep_degrees > 1e-2 {
			vec![PartitionNode::new(
				style,
				Partition::arc(sweep_degrees),
				Placement::new(center_xz, start_yaw).with_scale(ring_scale),
			)]
		} else {
			Vec::new()
		};
		Self { center_xz, radius, storey_height, sweep_degrees, start_yaw, style, partitions }
	}

	pub fn rough_stone(
		center_xz: Vec3,
		radius: f32,
		storey_height: f32,
		sweep_degrees: f32,
		start_yaw: f32,
	) -> Self {
		Self::new(
			center_xz,
			radius,
			storey_height,
			sweep_degrees,
			start_yaw,
			PartitionStyle::RoughStonework,
		)
	}
}

impl BuildingComponents for ArcSweep {
	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::from_free(self.partitions.clone())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn solid_emits_one_partition() {
		let a = ArcSweep::rough_stone(Vec3::ZERO, 4.0, 3.0, 180.0, 0.0);
		assert_eq!(a.partitions.len(), 1);
		assert!(matches!(a.partitions[0].geometry, Partition::Arc(_)));
	}
}
