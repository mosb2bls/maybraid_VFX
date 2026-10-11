//! Configurable column dimensions and assembly spec.

use bevy::prelude::*;

use crate::layout::DoricColumnLayout;
use crate::materials::DoricColumnMaterials;

/// Reference kit sizes in meters (authored GLBs match these extents).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoricColumnDimensions {
	pub pedestal_height: f32,
	pub pedestal_radius_bottom: f32,
	pub pedestal_radius_top: f32,
	pub shaft_height: f32,
	pub shaft_radius: f32,
	pub capital_height: f32,
	pub capital_radius: f32,
	/// Radial offset beyond the shaft radius for chain anchor empties.
	pub chain_anchor_outset: f32,
}

impl Default for DoricColumnDimensions {
	fn default() -> Self {
		Self {
			pedestal_height: 1.2,
			pedestal_radius_bottom: 0.95,
			pedestal_radius_top: 0.75,
			shaft_height: 14.0,
			shaft_radius: 0.75,
			capital_height: 1.0,
			capital_radius: 0.95,
			chain_anchor_outset: 0.08,
		}
	}
}

/// Full assembly recipe: dimensions, non-uniform scale, yaw, and per-part materials.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct DoricColumnSpec {
	pub dims: DoricColumnDimensions,
	/// Applied on top of [`Self::dims`] for gameplay tuning (encounter layout vs summons).
	pub scale: Vec3,
	pub yaw: f32,
	pub materials: DoricColumnMaterials,
}

impl Default for DoricColumnSpec {
	fn default() -> Self {
		Self {
			dims: DoricColumnDimensions::default(),
			scale: Vec3::ONE,
			yaw: 0.0,
			materials: DoricColumnMaterials::default(),
		}
	}
}

impl DoricColumnSpec {
	pub fn layout(&self) -> DoricColumnLayout {
		DoricColumnLayout::from_spec(self)
	}

	pub fn root_transform(&self, translation: Vec3) -> Transform {
		Transform::from_translation(translation)
			.with_rotation(Quat::from_rotation_y(self.yaw))
			.with_scale(Vec3::ONE)
	}

	pub fn assembly_aabb_half_extents(&self) -> Vec3 {
		let layout = self.layout();
		let r = layout
			.pedestal
			.radius_bottom
			.max(layout.shaft.radius_bottom)
			.max(layout.capital.radius_top);
		Vec3::new(r, layout.total_height() * 0.5, r)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn uniform_scale_multiplies_height() {
		let spec = DoricColumnSpec {
			dims: DoricColumnDimensions { shaft_height: 10.0, ..DoricColumnDimensions::default() },
			scale: Vec3::splat(1.5),
			yaw: 0.0,
			materials: Default::default(),
		};
		let layout = spec.layout();
		assert!((layout.shaft.height - 15.0).abs() < 1e-4);
	}
}
