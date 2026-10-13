//! Stacked circular wall rings — a minimal building used to validate kit scaling.
//!
//! Each storey is two 180° rough-stone arcs placed at the ring base and scaled
//! from the normalized kit (radius \(1\), height \(1\)) to `(radius, floor_height)`.

pub mod ring;

pub use ring::StackedRing;

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::partitions::PartitionNode;
use building_components::{BuildingComponents, Layers};
use lod::scene::LodSceneLevel;

use crate::CellConstraints;

/// Vertical stack of circular wall rings.
#[derive(Debug, Clone, PartialEq)]
pub struct StackedRings {
	pub constraints: CellConstraints,
	pub floor_count: u32,
	pub floor_height: f32,
	pub radius: f32,
	pub rings: Vec<StackedRing>,
}

impl StackedRings {
	/// Build `floor_count` rings of height `floor_height` and radius `radius`.
	///
	/// The footprint AABB is axis-aligned around the origin:
	/// \([-R, R] \times [0, N \cdot H] \times [-R, R]\).
	pub fn new(floor_count: u32, floor_height: f32, radius: f32) -> Self {
		let floor_count = floor_count.max(1);
		let floor_height = floor_height.max(1e-4);
		let radius = radius.max(1e-4);
		let total_height = floor_count as f32 * floor_height;
		let constraints = CellConstraints::cell_owned(Aabb3d::from_min_max(
			Vec3::new(-radius, 0.0, -radius),
			Vec3::new(radius, total_height, radius),
		));

		let rings = (0..floor_count)
			.map(|i| {
				let base_y = i as f32 * floor_height;
				StackedRing::new(base_y, floor_height, radius)
			})
			.collect();

		Self { constraints, floor_count, floor_height, radius, rings }
	}
}

impl BuildingComponents for StackedRings {
	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		let mut out = Layers::new();
		for ring in &self.rings {
			out.extend(ring.partition_nodes_for_level(level));
		}
		out
	}
}
