//! **D-normalized authoring** + **uniform runtime scale** (matches [#1143](https://github.com/ramate-io/maybraid/pull/1143) `COLUMN_SURFACE.md`).
//!
//! Each part GLB is modeled in **multiples of lower shaft diameter D** (local **D = 1** horizontally on the shaft):
//! bottom-center pivot, **+Y** up, real [`DoricColumnRatios`] proportions baked into the mesh (not unit height 1).
//!
//! Runtime [`crate::layout::part_placements`] sets **`Placement::scale = Vec3::splat(D_world)`** only — never per-axis stretch — so
//! shaft tangents and POM UV density stay valid under [#1143](https://github.com/ramate-io/maybraid/pull/1143).

use bevy::prelude::*;
use building_components::Placement;

use crate::spec::{DoricColumnRatios, DoricColumnSpec};

/// Lower shaft **diameter** in authored part space (**1.0** = one kit diameter).
pub const NORMALIZED_LOWER_SHAFT_DIAMETER: f32 = 1.0;

/// Local mesh extents for the plinth (same units as **D** in the mesh file).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlinthAuthoredExtents {
	pub height_d: f32,
	pub half_side_d: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShaftAuthoredExtents {
	pub height_d: f32,
	pub radius_bottom_d: f32,
	pub radius_top_d: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CapitalAuthoredExtents {
	pub echinus_height_d: f32,
	pub abacus_height_d: f32,
	pub echinus_radius_bottom_d: f32,
	pub echinus_radius_top_d: f32,
	pub abacus_half_side_d: f32,
}

impl CapitalAuthoredExtents {
	pub fn height_d(self) -> f32 {
		self.echinus_height_d + self.abacus_height_d
	}
}

pub fn authored_extents(
	ratios: DoricColumnRatios,
) -> (PlinthAuthoredExtents, ShaftAuthoredExtents, CapitalAuthoredExtents) {
	let plinth = PlinthAuthoredExtents {
		height_d: ratios.plinth_height_d,
		half_side_d: ratios.plinth_side_d * 0.5,
	};
	let shaft = ShaftAuthoredExtents {
		height_d: ratios.shaft_height_d(),
		radius_bottom_d: NORMALIZED_LOWER_SHAFT_DIAMETER * 0.5,
		radius_top_d: ratios.shaft_upper_diameter_d * 0.5,
	};
	let capital = CapitalAuthoredExtents {
		echinus_height_d: ratios.echinus_height_d,
		abacus_height_d: ratios.abacus_height_d,
		echinus_radius_bottom_d: shaft.radius_top_d,
		echinus_radius_top_d: ratios.echinus_bulge_radius_d,
		abacus_half_side_d: ratios.abacus_side_d * 0.5,
	};
	(plinth, shaft, capital)
}

/// World metres per authored unit (**D_world**).
pub fn uniform_placement_scale(spec: &DoricColumnSpec) -> f32 {
	spec.dims.d() * spec.uniform_scale()
}

pub fn placement_scale_vec(spec: &DoricColumnSpec) -> Vec3 {
	Vec3::splat(uniform_placement_scale(spec))
}

/// Top **Y** in assembly space after uniform scale (mesh local height in **D** units).
pub fn world_top_y(placement: &Placement, authored_height_d: f32) -> f32 {
	let d = placement.scale.x;
	placement.translation.y + d * authored_height_d
}

/// **GLB vs spec:** gameplay anchors and cover colliders follow [`DoricColumnLayout`] from the spec.
/// Authored GLBs must match [`DoricColumnRatios`]. When a loaded GLB’s bounds or named empties disagree
/// with the spec, the runtime logs a warning and **named anchor nodes in the GLB** (`ground_contact`, `shaft_top`, …)
/// override spec-computed anchor transforms; cover colliders remain spec-based until the asset is fixed.

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::{names, DoricColumnAnchors};
	use crate::colliders::cover_aabb;
	use crate::layout::part_placements;
	use crate::spec::DoricColumnSpec;

	#[test]
	fn d_normalized_mesh_with_uniform_scale_matches_layout_anchors_and_colliders() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		let ratios = spec.dims.ratios;
		let (plinth, shaft, capital) = authored_extents(ratios);
		let parts = part_placements(&spec);
		let anchors = DoricColumnAnchors::for_spec(&spec);
		let (cover_min, cover_max) = cover_aabb(&layout);
		let d = uniform_placement_scale(&spec);
		let scale = Vec3::splat(d);

		for (label, p) in
			[("pedestal", &parts.pedestal), ("shaft", &parts.shaft), ("capital", &parts.capital)]
		{
			assert!(
				(p.scale - scale).length() < 1e-4,
				"{label} must use uniform scale D only (got {:?})",
				p.scale
			);
		}

		assert!(
			(world_top_y(&parts.pedestal, plinth.height_d) - layout.plinth.height).abs() < 1e-3
		);
		assert!((world_top_y(&parts.shaft, shaft.height_d) - layout.shaft_top_y()).abs() < 1e-3);
		assert!(
			(world_top_y(&parts.capital, capital.height_d()) - layout.total_height()).abs() < 1e-3
		);

		assert!(
			(anchors.get(names::PEDESTAL_TOP).unwrap().translation.y - layout.plinth.height).abs()
				< 1e-3
		);
		assert!(
			(anchors.get(names::SHAFT_TOP).unwrap().translation.y - layout.shaft_top_y()).abs()
				< 1e-3
		);
		assert!(
			(anchors.get(names::CAPITAL_TOP).unwrap().translation.y - layout.total_height()).abs()
				< 1e-3
		);

		assert!((cover_max.y - layout.total_height()).abs() < 1e-3);
		assert!(cover_min.y.abs() < 1e-4);

		let wrong_top = parts.shaft.translation.y + d * d * shaft.height_d;
		assert!(
			wrong_top > layout.shaft_top_y() * 1.2,
			"uniform D scale must not be confused with baking world metres into the mesh"
		);
	}

	#[test]
	fn summon_uniform_scale_still_single_d_factor() {
		let spec = DoricColumnSpec { scale: Vec3::splat(0.85), ..DoricColumnSpec::default() };
		let parts = part_placements(&spec);
		let d = uniform_placement_scale(&spec);
		assert!((parts.shaft.scale.x - d).abs() < 1e-4);
		let shaft = authored_extents(spec.dims.ratios).1;
		assert!(
			(world_top_y(&parts.shaft, shaft.height_d) - spec.layout().shaft_top_y()).abs() < 1e-3
		);
	}
}
