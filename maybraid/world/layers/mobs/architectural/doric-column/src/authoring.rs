//! Authoring convention: **unit-normalized part meshes** scaled once by [`crate::layout::part_placements`].
//!
//! GLBs and procedural placeholders model each part in **local part space** with:
//! - **Y ∈ [0, 1]** along the part height (bottom pivot at **Y = 0**).
//! - Horizontal extent **1** in the kit’s normalized frame (square side = 1, shaft lower diameter = 1, etc.).
//!
//! Runtime [`building_components::Placement::scale`] carries **world metres** from [`DoricColumnLayout`].
//! Do **not** bake [`DoricColumnDimensions`] metre sizes into the mesh; that would double-scale with placement.

use bevy::prelude::*;
use building_components::Placement;

/// Authored mesh height before placement scale (every part).
pub const UNIT_PART_HEIGHT: f32 = 1.0;

/// Plinth / abacus unit square side in local X and Z.
pub const UNIT_SQUARE_SIDE: f32 = 1.0;

/// Shaft unit lower diameter (mesh radius 0.5 at **Y = 0**).
pub const UNIT_SHAFT_LOWER_DIAMETER: f32 = 1.0;

/// Shaft unit upper diameter at **Y = 1** (matches default `shaft_upper_diameter_d = 0.8`).
pub const UNIT_SHAFT_UPPER_DIAMETER: f32 = 0.8;

/// World-space axis-aligned bounds for a **unit-normalized** mesh under `placement`.
pub fn world_aabb_from_unit_mesh_placement(placement: &Placement) -> (Vec3, Vec3) {
	let min = placement.translation
		- Vec3::new(placement.scale.x * 0.5, 0.0, placement.scale.z * 0.5);
	let max = min + placement.scale;
	(min, max)
}

/// Top **Y** of a unit-height part after placement (assembly space).
pub fn world_top_y(placement: &Placement) -> f32 {
	placement.translation.y + placement.scale.y * UNIT_PART_HEIGHT
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::anchors::{names, DoricColumnAnchors};
	use crate::colliders::cover_aabb;
	use crate::layout::part_placements;
	use crate::spec::DoricColumnSpec;

	#[test]
	fn simulated_authored_unit_mesh_matches_layout_anchors_and_colliders() {
		let spec = DoricColumnSpec::default();
		let layout = spec.layout();
		let parts = part_placements(&layout);
		let anchors = DoricColumnAnchors::for_spec(&spec);
		let (cover_min, cover_max) = cover_aabb(&layout);

		assert!(
			(parts.pedestal.scale.y - layout.plinth.height).abs() < 1e-4,
			"plinth placement must scale unit mesh once to layout height"
		);
		assert!(
			(parts.shaft.scale.y - layout.shaft.height).abs() < 1e-4,
			"shaft placement must scale unit mesh once to layout height"
		);
		assert!(
			(parts.capital.scale.y - layout.capital_height()).abs() < 1e-4,
			"capital placement must scale unit mesh once to layout height"
		);

		let pedestal_top = world_top_y(&parts.pedestal);
		assert!((pedestal_top - layout.plinth.height).abs() < 1e-4);
		assert!(
			(anchors.get(names::PEDESTAL_TOP).unwrap().translation.y - pedestal_top).abs() < 1e-4
		);

		let shaft_top = world_top_y(&parts.shaft);
		assert!((shaft_top - layout.shaft_top_y()).abs() < 1e-4);
		assert!(
			(anchors.get(names::SHAFT_TOP).unwrap().translation.y - shaft_top).abs() < 1e-4
		);

		let capital_top = world_top_y(&parts.capital);
		assert!((capital_top - layout.total_height()).abs() < 1e-4);
		assert!(
			(anchors.get(names::CAPITAL_TOP).unwrap().translation.y - capital_top).abs() < 1e-4
		);

		assert!((cover_max.y - layout.total_height()).abs() < 1e-4);
		assert!(cover_min.y.abs() < 1e-4);
		assert!((cover_max.y - capital_top).abs() < 1e-4);

		// Baking layout metres into the mesh *and* applying placement scale double-counts height.
		let double_scaled_shaft_top =
			parts.shaft.translation.y + parts.shaft.scale.y * layout.shaft.height;
		assert!(
			double_scaled_shaft_top > layout.shaft_top_y() * 1.5,
			"authored-at-metres GLB + placement scale must not be used together"
		);
	}

	#[test]
	fn summon_uniform_scale_still_single_mesh_scale() {
		let spec = DoricColumnSpec {
			scale: Vec3::splat(0.85),
			..DoricColumnSpec::default()
		};
		let layout = spec.layout();
		let parts = part_placements(&layout);
		assert!((world_top_y(&parts.shaft) - layout.shaft_top_y()).abs() < 1e-3);
	}
}
