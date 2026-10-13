//! Continuous arc sweep geometry.

use bevy_math::Vec3;

use crate::arc_kit::decompose_arc_sweep;
use crate::partitions::geometry::{slice_tile, PartitionTile};
use crate::placed::Placed;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArcSweep {
	pub sweep_degrees: f32,
}

impl Default for ArcSweep {
	fn default() -> Self {
		Self { sweep_degrees: 90.0 }
	}
}

impl ArcSweep {
	pub fn tiles(self, slice: bool) -> Vec<Placed<PartitionTile>> {
		decompose_arc_sweep(self.sweep_degrees)
			.into_iter()
			.map(|(kit, yaw)| {
				let tile = if slice { slice_tile(kit) } else { PartitionTile::from(kit) };
				Placed::new(tile, Vec3::ZERO, yaw)
			})
			.collect()
	}
}
