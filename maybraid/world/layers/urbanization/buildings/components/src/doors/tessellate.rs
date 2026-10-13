//! Private door kit tessellation (not part of the public IR).

use bevy_math::Vec3;

use crate::doors::geometry::DoorGeometry;
use crate::partitions::geometry::PartitionTile;
use crate::placed::{Placed, Placement};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum DoorKit {
	Leaf,
	FramePiece(PartitionTile),
}

impl DoorGeometry {
	pub(crate) fn kit_pieces(&self) -> Vec<Placed<DoorKit>> {
		match self {
			Self::Leaf(_) => vec![Placed::at_origin(DoorKit::Leaf)],
			Self::Frame15(_) => vec![
				Placed::new(
					DoorKit::FramePiece(PartitionTile::LinearSliceSubsegment),
					Vec3::new(0.0, 0.85, 0.0),
					0.0,
				),
				Placed::new(
					DoorKit::FramePiece(PartitionTile::SliceArc15),
					Vec3::new(-0.9, 0.85, 0.0),
					0.0,
				),
				Placed::new(
					DoorKit::FramePiece(PartitionTile::SliceArc15),
					Vec3::new(0.9, 0.85, 0.0),
					std::f32::consts::PI,
				),
				Placed::new(
					DoorKit::FramePiece(PartitionTile::Arc15),
					Vec3::new(-1.0, 0.0, 0.0),
					0.0,
				),
				Placed::new(
					DoorKit::FramePiece(PartitionTile::Arc15),
					Vec3::new(1.0, 0.0, 0.0),
					std::f32::consts::PI,
				),
			],
		}
	}

	pub(crate) fn placed_kits(&self, parent: Placement) -> Vec<Placed<DoorKit>> {
		self.kit_pieces()
			.into_iter()
			.map(|child| Placed {
				geom: child.geom,
				placement: parent.compose_child(child.placement),
			})
			.collect()
	}
}
