//! Configurable column dimensions and assembly spec.

use bevy::prelude::*;

use crate::layout::DoricColumnLayout;
use crate::materials::DoricColumnMaterials;

/// Classical Doric proportions expressed as multiples of lower shaft diameter **D** (meters at unit scale).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoricColumnRatios {
	/// Base through capital, typically **6–7 D**.
	pub column_height_d: f32,
	pub plinth_height_d: f32,
	/// Square plinth side length (**1.3–1.5 D**).
	pub plinth_side_d: f32,
	/// Entire capital (echinus + abacus), typically **~0.5 D**.
	pub capital_height_d: f32,
	pub echinus_height_d: f32,
	pub abacus_height_d: f32,
	/// Square abacus / cornice side (**~1.2 D**).
	pub abacus_side_d: f32,
	/// Upper shaft diameter as multiple of **D** (entasis), typically **~0.8**.
	pub shaft_upper_diameter_d: f32,
	/// Echinus bulge radius at its widest, as multiple of **D** (between shaft top and abacus).
	pub echinus_bulge_radius_d: f32,
	pub chain_anchor_outset_d: f32,
}

impl Default for DoricColumnRatios {
	fn default() -> Self {
		Self {
			column_height_d: 6.5,
			plinth_height_d: 0.4,
			plinth_side_d: 1.4,
			capital_height_d: 0.5,
			echinus_height_d: 0.3,
			abacus_height_d: 0.2,
			abacus_side_d: 1.2,
			shaft_upper_diameter_d: 0.8,
			echinus_bulge_radius_d: 0.52,
			chain_anchor_outset_d: 0.05,
		}
	}
}

impl DoricColumnRatios {
	pub fn shaft_height_d(self) -> f32 {
		self.column_height_d - self.plinth_height_d - self.capital_height_d
	}

	pub fn validate(self) -> Result<(), &'static str> {
		if self.shaft_height_d() <= 0.0 {
			return Err("shaft height must be positive");
		}
		if (self.echinus_height_d + self.abacus_height_d - self.capital_height_d).abs() > 1e-4 {
			return Err("capital height must equal echinus + abacus");
		}
		Ok(())
	}
}

/// Reference kit sizes: **D** is the lower shaft diameter; ratios stay fixed when scaling uniformly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoricColumnDimensions {
	/// Lower shaft diameter **D** in meters at [`DoricColumnSpec::scale`] = 1.
	pub lower_shaft_diameter_d: f32,
	pub ratios: DoricColumnRatios,
}

impl Default for DoricColumnDimensions {
	fn default() -> Self {
		Self {
			// Matches legacy encounter column width (former shaft radius 0.75 m).
			lower_shaft_diameter_d: 1.5,
			ratios: DoricColumnRatios::default(),
		}
	}
}

impl DoricColumnDimensions {
	pub fn d(&self) -> f32 {
		self.lower_shaft_diameter_d
	}

	pub fn shaft_height(&self) -> f32 {
		self.ratios.shaft_height_d() * self.d()
	}

	pub fn total_height(&self) -> f32 {
		self.ratios.column_height_d * self.d()
	}
}

/// Full assembly recipe: dimensions, uniform scale, yaw, and per-part materials.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct DoricColumnSpec {
	pub dims: DoricColumnDimensions,
	/// Uniform scale preserves classical ratios; non-uniform values use the largest component.
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
	/// Single factor applied to every derived length so ratios stay fixed.
	pub fn uniform_scale(&self) -> f32 {
		self.scale.x.max(self.scale.y).max(self.scale.z)
	}

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
		let hx = layout.plinth.half_side.max(layout.abacus.half_side);
		Vec3::new(hx, layout.total_height() * 0.5, hx)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn default_ratios_form_classical_column() {
		let r = DoricColumnRatios::default();
		assert!(r.validate().is_ok());
		assert!((r.column_height_d - 6.5).abs() < 1e-5);
		assert!((r.shaft_upper_diameter_d - 0.8).abs() < 1e-5);
		assert!((r.abacus_side_d - 1.2).abs() < 1e-5);
		assert!(r.plinth_side_d >= 1.3 && r.plinth_side_d <= 1.5);
	}

	#[test]
	fn uniform_scale_multiplies_height() {
		let spec = DoricColumnSpec {
			scale: Vec3::splat(2.0),
			..DoricColumnSpec::default()
		};
		let layout = spec.layout();
		let expected = DoricColumnSpec::default().layout().shaft.height * 2.0;
		assert!((layout.shaft.height - expected).abs() < 1e-4);
	}

	#[test]
	fn shaft_slenderness_is_proportional() {
		let dims = DoricColumnDimensions::default();
		let slenderness = dims.shaft_height() / dims.d();
		assert!(slenderness > 4.0 && slenderness < 7.0);
	}
}
