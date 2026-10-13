//! Label geometry variants.

use bevy_math::Vec3;

/// Continuous label form. One variant for now: an axis-aligned box.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LabelGeometry {
	/// Local extents (full edge lengths) for a centered rectangle prism.
	Rectangle { extents: Vec3 },
}

impl Default for LabelGeometry {
	fn default() -> Self {
		Self::rectangle(Vec3::ONE)
	}
}

impl LabelGeometry {
	pub fn rectangle(extents: Vec3) -> Self {
		Self::Rectangle { extents: extents.max(Vec3::splat(1e-4)) }
	}

	pub fn extents(self) -> Vec3 {
		match self {
			Self::Rectangle { extents } => extents,
		}
	}
}
