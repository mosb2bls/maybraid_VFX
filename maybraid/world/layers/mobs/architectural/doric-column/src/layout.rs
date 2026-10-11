//! Part stack geometry derived from [`DoricColumnSpec`].

use bevy::prelude::*;
use building_components::Placement;

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

/// World [`Placement`] for each part. Assumes meshes authored per [`crate::authoring`] (unit height, unit footprint).
pub fn part_placements(layout: &DoricColumnLayout) -> PartPlacements {
	let plinth_y = 0.0;
	let shaft_y = layout.plinth.height;
	let capital_y = layout.shaft_top_y();
	let capital_xz = layout.abacus.half_side * 2.0;
	let capital_h = layout.capital_height();
	PartPlacements {
		pedestal: square_placement(plinth_y, layout.plinth),
		shaft: round_placement(shaft_y, layout.shaft),
		capital: Placement::new(Vec3::new(0.0, capital_y, 0.0), 0.0)
			.with_scale(Vec3::new(capital_xz, capital_h, capital_xz)),
	}
}

fn square_placement(base_y: f32, part: SquarePartLayout) -> Placement {
	let side = part.half_side * 2.0;
	Placement::new(Vec3::new(0.0, base_y, 0.0), 0.0)
		.with_scale(Vec3::new(side, part.height, side))
}

fn round_placement(base_y: f32, part: RoundPartLayout) -> Placement {
	let diameter = part.radius_bottom.max(part.radius_top) * 2.0;
	Placement::new(Vec3::new(0.0, base_y, 0.0), 0.0)
		.with_scale(Vec3::new(diameter, part.height, diameter))
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::spec::{DoricColumnDimensions, DoricColumnSpec};

	#[test]
	fn assembly_height_is_sum_of_parts() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		assert!(
			(layout.total_height() - spec.dims.total_height() * spec.uniform_scale()).abs() < 1e-3
		);
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
