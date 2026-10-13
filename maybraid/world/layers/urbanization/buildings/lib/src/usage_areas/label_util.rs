//! Shared [`LabelNode`] helpers for usage-area placeholders (AABB → filled label).

use bevy_math::bounding::{Aabb3d, BoundingVolume};
use bevy_math::Vec3;
use building_components::{LabelNode, LabelStyle};

/// Label filling `aabb` (unit cube scaled to extents).
///
/// Local yaw is identity. The AABB is authored axis-aligned; building heading is
/// the host spawn transform. Applying [`crate::fit::Confines::roll`] here would
/// double-rotate against that pose and leave the wireframe looking world-aligned
/// while the building is yawed.
pub fn label_filling_aabb(style: LabelStyle, text: &str, aabb: &Aabb3d, _yaw: f32) -> LabelNode {
	let center = Vec3::from(aabb.center());
	let extents = Vec3::from(aabb.max - aabb.min).max(Vec3::splat(1e-4));
	LabelNode::rectangle(style, text, center, extents, 0.0)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_4;

	#[test]
	fn filling_an_aabb_keeps_identity_local_yaw() {
		let aabb = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0));
		let label = label_filling_aabb(LabelStyle::Cyan, "Room", &aabb, FRAC_PI_4);
		assert!((label.placement.yaw - 0.0).abs() < 1e-6);
	}
}
