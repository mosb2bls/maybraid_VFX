//! Kit capabilities for a panel look (not a user tessellation preference).

use crate::floors::style::FloorStyle;
use crate::partitions::style::PartitionStyle;
use crate::roofs::style::RoofStyle;

/// When [`Self::has_rectangle`] is false, rectangular body regions are filled with
/// complementary right-triangle pairs. Domain / panel material styles map into this
/// via [`From`]; see [`crate::panels::PanelStyle`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PanelKitCaps {
	pub has_rectangle: bool,
}

impl PanelKitCaps {
	pub const WITH_RECTANGLE: Self = Self { has_rectangle: true };
	pub const TRIANGLES_ONLY: Self = Self { has_rectangle: false };
}

impl From<RoofStyle> for PanelKitCaps {
	fn from(style: RoofStyle) -> Self {
		match style {
			RoofStyle::ShepherdsThatch => Self::WITH_RECTANGLE,
		}
	}
}

impl From<PartitionStyle> for PanelKitCaps {
	fn from(style: PartitionStyle) -> Self {
		match style {
			PartitionStyle::RoughStonework => Self::WITH_RECTANGLE,
		}
	}
}

impl From<FloorStyle> for PanelKitCaps {
	fn from(style: FloorStyle) -> Self {
		match style {
			FloorStyle::RoughStonework | FloorStyle::Wood => Self::WITH_RECTANGLE,
		}
	}
}
