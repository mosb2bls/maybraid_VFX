//! Shared presentable fill for a walled private room with an authored door.

use bevy_math::bounding::Aabb3d;
use building_components::{LabelNode, LabelStyle};

use crate::fit::{Confines, FillRegion, SpaceKind};
use crate::openings::{Opening, OpeningId, Openings};
use crate::paneling::Rectangle;
use crate::usage_areas::label_util::label_filling_aabb;

/// Enclosure panels + door + residual bounds ready for `FillableRegions::within`.
#[derive(Debug, Clone, PartialEq)]
pub struct WalledRoomFill {
	pub bounds: Aabb3d,
	pub walls: Vec<Rectangle>,
	pub door_id: OpeningId,
	pub door: Opening,
	pub label: LabelNode,
}

impl WalledRoomFill {
	pub fn new(
		bounds: Aabb3d,
		walls: Vec<Rectangle>,
		door_id: OpeningId,
		door: Opening,
		style: LabelStyle,
		label_text: &str,
		roll: f32,
	) -> Self {
		Self {
			bounds,
			walls,
			door_id,
			door,
			label: label_filling_aabb(style, label_text, &bounds, roll),
		}
	}

	/// Residual fill region carrying the authored door.
	pub fn to_fill_region(&self, kind: SpaceKind, roll: f32) -> FillRegion {
		let mut openings = Openings::new();
		openings.insert(self.door_id.clone(), self.door.clone());
		FillRegion::new(kind, Confines::new(self.bounds, roll, openings))
	}
}
