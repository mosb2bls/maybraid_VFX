//! Part stack geometry derived from [`DoricColumnSpec`].

use bevy::prelude::*;
use building_components::Placement;

use crate::authoring::{authored_extents, placement_scale_vec, uniform_placement_scale};
use crate::spec::DoricColumnSpec;

/// Axis-aligned square footprint: faces perpendicular to **±X** and **±Z**, centered on the shaft axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SquarePartLayout {
	pub height: f32,
	/// Half of the square side in X and Z.
	pub half_side: f32,
}

/// Circular footprint (shaft or echinus).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoundPartLayout {
	pub height: f32,
	pub radius_bottom: f32,
	pub radius_top: f32,
}

/// Resolved world-unit sizes after [`DoricColumnSpec::uniform_scale`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DoricColumnLayout {
	pub plinth: SquarePartLayout,
	pub shaft: RoundPartLayout,
	pub echinus: RoundPartLayout,
	pub abacus: SquarePartLayout,
	pub chain_anchor_outset: f32,
}

impl DoricColumnLayout {
	pub fn from_spec(spec: &DoricColumnSpec) -> Self {
		let k = spec.uniform_scale();
		let d = spec.dims.d() * k;
		let r = spec.dims.ratios;
		let shaft_r_bot = d * 0.5;
		let shaft_r_top = d * r.shaft_upper_diameter_d * 0.5;
		Self {
			plinth: SquarePartLayout {
				height: r.plinth_height_d * d,
				half_side: r.plinth_side_d * d * 0.5,
			},
			shaft: RoundPartLayout {
				height: r.shaft_height_d() * d,
				radius_bottom: shaft_r_bot,
				radius_top: shaft_r_top,
			},
			echinus: RoundPartLayout {
				height: r.echinus_height_d * d,
				radius_bottom: shaft_r_top,
				radius_top: d * r.echinus_bulge_radius_d,
			},
			abacus: SquarePartLayout {
				height: r.abacus_height_d * d,
				half_side: r.abacus_side_d * d * 0.5,
			},
			chain_anchor_outset: r.chain_anchor_outset_d * d,
		}
	}

	pub fn total_height(&self) -> f32 {
		self.plinth.height + self.shaft.height + self.echinus.height + self.abacus.height
	}

	pub fn capital_height(&self) -> f32 {
		self.echinus.height + self.abacus.height
	}

	pub fn shaft_top_y(&self) -> f32 {
		self.plinth.height + self.shaft.height
	}

	pub fn max_plan_half_extent(&self) -> f32 {
		self.plinth.half_side.max(self.abacus.half_side)
	}
}

pub struct PartPlacements {
	pub pedestal: Placement,
	pub shaft: Placement,
	pub capital: Placement,
}

/// World [`Placement`] for D-normalized meshes: **uniform** `scale = D_world`, translations in metres.
pub fn part_placements(spec: &DoricColumnSpec) -> PartPlacements {
	let r = spec.dims.ratios;
	let d = uniform_placement_scale(spec);
	let scale = placement_scale_vec(spec);
	let (plinth, shaft, _) = authored_extents(r);
	let shaft_base_y = plinth.height_d * d;
	let capital_base_y = (plinth.height_d + shaft.height_d) * d;
	PartPlacements {
		pedestal: Placement::new(Vec3::new(0.0, 0.0, 0.0), 0.0).with_scale(scale),
		shaft: Placement::new(Vec3::new(0.0, shaft_base_y, 0.0), 0.0).with_scale(scale),
		capital: Placement::new(Vec3::new(0.0, capital_base_y, 0.0), 0.0).with_scale(scale),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::spec::DoricColumnSpec;

	#[test]
	fn assembly_height_is_sum_of_parts() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		assert!(
			(layout.total_height() - spec.dims.total_height() * spec.uniform_scale()).abs() < 1e-3
		);
	}

	#[test]
	fn placements_use_uniform_scale_only() {
		let spec = DoricColumnSpec::default();
		let parts = part_placements(&spec);
		let s = parts.pedestal.scale;
		assert!((parts.shaft.scale - s).length() < 1e-5);
		assert!((parts.capital.scale - s).length() < 1e-5);
		assert!((s.x - s.y).abs() < 1e-5 && (s.y - s.z).abs() < 1e-5);
	}

	#[test]
	fn scaled_spec_preserves_height_ratios() {
		let base = DoricColumnSpec::default().layout();
		let scaled = DoricColumnSpec {
			scale: Vec3::splat(0.85),
			..DoricColumnSpec::default()
		}
		.layout();
		let ratio = scaled.shaft.height / scaled.plinth.height;
		let base_ratio = base.shaft.height / base.plinth.height;
		assert!((ratio - base_ratio).abs() < 1e-4);
	}

	#[test]
	fn square_plinth_and_abacus_share_axes() {
		let layout = DoricColumnSpec::default().layout();
		assert!(layout.plinth.half_side > layout.shaft.radius_bottom);
		assert!(layout.abacus.half_side > layout.echinus.radius_top);
	}
}
