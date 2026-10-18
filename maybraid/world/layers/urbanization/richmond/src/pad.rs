//! Union-first building-pad primitives + broadphase helpers.
//!
//! Developments plan their pads as [`urbanization_developments::PadPlan`]s;
//! this module realizes them. Skirt ease lives on [`PadParams`]. Terrain blend
//! lives on [`crate::pad::node::PadNode`]: flatten interiors stay exact;
//! overlapping grades and skirts mix by occupancy (hydro shore / softmax analog).

pub mod complex;
pub mod elevation;
pub mod footprint;
mod index;
pub mod node;

pub use complex::PadComplex;
pub use elevation::PadElevation;
pub use footprint::{PadFootprint, PadReach, PadRect};
pub use node::{PadNode, PadStage};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec2;
use procedural_common::Bounds2;
use urbanization_developments::PadParams;

/// One pad leaf: footprint + local elevation field.
#[derive(Debug, Clone)]
pub struct PadPrimitive {
	pub footprint: PadFootprint,
	pub elevation: PadElevation,
	/// Extra AABB pad for broadphase / ease support (world units).
	pub influence_pad: f32,
}

impl PadPrimitive {
	pub fn aabb(&self) -> (Vec2, Vec2) {
		self.footprint.aabb()
	}

	pub fn phi(&self, p: Vec2) -> f32 {
		self.footprint.sdf(p)
	}

	pub fn height_at(&self, p: Vec2) -> f32 {
		match self.elevation {
			PadElevation::Flatten { height } => height,
			PadElevation::Grade { height_a, height_b } => {
				let z = self.footprint.reach_progress(p).unwrap_or(0.5);
				height_a + (height_b - height_a) * z
			}
		}
	}
}

/// XZ bounds of a development (or terrain) cell.
pub fn cell_bounds2(cell: Aabb3d) -> Bounds2 {
	Bounds2::from_xz(cell.min.x, cell.min.z, cell.max.x, cell.max.z)
}

/// Cell-centered XZ sample used as the building / pad origin.
pub fn cell_center_xz(cell: Aabb3d) -> Vec2 {
	Vec2::new((cell.min.x + cell.max.x) * 0.5, (cell.min.z + cell.max.z) * 0.5)
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::cell::BUILDING_INSET;
	use bevy::math::Vec3;
	use std::f32::consts::FRAC_PI_4;
	use std::sync::Arc;
	use urbanization_developments::{
		yaw_about_xz, ShepherdsBuilding, ShepherdsHut, ShepherdsVillageBuilding, PAD_BERM,
		PAD_EDGE_EASE,
	};

	fn skirt(half: Vec2, yaw: f32, height: f32) -> PadComplex {
		PadComplex::building_skirt(Vec2::ZERO, half, yaw, height, PadParams::default())
	}

	#[test]
	fn berm_plus_ease_matches_building_inset() {
		assert!((BUILDING_INSET - (PAD_BERM + PAD_EDGE_EASE)).abs() < 1e-6);
	}

	#[test]
	fn flatten_center_is_constant_height() {
		let pad = skirt(Vec2::new(20.0, 12.0), 0.0, 42.0);
		assert!((pad.modify_elevation(10.0, 0.0, 0.0) - 42.0).abs() < 1e-3);
		assert!((pad.modify_elevation(90.0, 0.0, 0.0) - 42.0).abs() < 1e-3);
		assert_eq!(pad.classification_at(0.0, 0.0), Some(PadStage::Flatten));
	}

	#[test]
	fn far_sample_is_identity() {
		let pad = skirt(Vec2::new(20.0, 12.0), 0.0, 42.0);
		let h = pad.modify_elevation(17.0, 70.0, 70.0);
		assert!((h - 17.0).abs() < 1e-3, "far sample {h}");
	}

	#[test]
	fn ease_band_sits_between_pad_and_terrain() {
		let half_x = 20.0;
		let pad = skirt(Vec2::new(half_x, 12.0), 0.0, 40.0);
		let berm = PadParams::default().berm;
		let x_ease = half_x + berm + 3.0;
		let h = pad.modify_elevation(10.0, x_ease, 0.0);
		assert_eq!(pad.classification_at(x_ease, 0.0), Some(PadStage::Ease));
		assert!(h > 10.0 + 1.0, "ease should lift toward pad: {h}");
		assert!(h < 40.0 - 1.0, "ease should not be full flatten: {h}");
	}

	#[test]
	fn yawed_flatten_follows_building_x() {
		let half = Vec2::new(18.0, 8.0);
		let pad = skirt(half, FRAC_PI_4, 30.0);
		let (s, c) = FRAC_PI_4.sin_cos();
		let local_x = 10.0;
		let p = Vec2::new(c * local_x, -s * local_x);
		assert_eq!(pad.classification_at(p.x, p.y), Some(PadStage::Flatten));
		assert!((pad.modify_elevation(1.0, p.x, p.y) - 30.0).abs() < 1e-3);
		let away = Vec2::new(s * 40.0, c * 40.0);
		let h = pad.modify_elevation(7.0, away.x, away.y);
		assert!((h - 7.0).abs() < 1e-3, "lateral far field {h}");
	}

	#[test]
	fn overlapping_flatten_prefers_tightest_terrace() {
		let outer = PadNode::rectangular_flatten(
			Vec2::ZERO,
			Vec2::new(20.0, 20.0),
			0.0,
			10.0,
			PadParams { berm: 0.0, ease: 4.0, round: 0.0 },
		);
		let inner = PadNode::rectangular_flatten(
			Vec2::ZERO,
			Vec2::new(6.0, 6.0),
			0.0,
			22.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		let complex = PadComplex::from_nodes(vec![outer, inner]);
		assert!((complex.modify_elevation(0.0, 0.0, 0.0) - 22.0).abs() < 1e-3);
	}

	#[test]
	fn yawed_flatten_is_not_the_unrotated_aabb() {
		let half = Vec2::new(18.0, 8.0);
		let pad = PadComplex::building_skirt(
			Vec2::ZERO,
			half,
			FRAC_PI_4,
			30.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		// Inside the unrotated 36×16 AABB, but outside the yawed 36×16 OBB.
		assert_ne!(pad.classification_at(half.x - 0.5, 0.0), Some(PadStage::Flatten));
		let span = pad.bounds.max - pad.bounds.min;
		assert!(span.x < 50.0 && span.y < 50.0, "pad support must not be a cell terrace: {span:?}");
	}

	#[test]
	fn pad_contains_spawned_building_corner() {
		let center = Vec2::new(250.0, -100.0);
		let half = Vec2::new(18.0, 8.0);
		let yaw = FRAC_PI_4;
		let pad = PadComplex::building_skirt(
			center,
			half,
			yaw,
			12.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		let corner = yaw_about_xz(center, yaw).transform_point(Vec3::new(
			center.x + half.x - 0.5,
			0.0,
			center.y + half.y - 0.5,
		));
		assert_eq!(pad.classification_at(corner.x, corner.z), Some(PadStage::Flatten));
	}

	#[test]
	fn graded_reach_lerps_between_pad_heights() {
		let pad = PadComplex::graded_polyline(
			&[Vec2::ZERO, Vec2::new(40.0, 0.0)],
			&[10.0, 20.0],
			4.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		assert_eq!(pad.classification_at(20.0, 0.0), Some(PadStage::Grade));
		assert!((pad.modify_elevation(0.0, 20.0, 0.0) - 15.0).abs() < 1e-3);
		assert!((pad.modify_elevation(0.0, 0.0, 0.0) - 10.0).abs() < 1e-3);
		assert!((pad.modify_elevation(0.0, 40.0, 0.0) - 20.0).abs() < 1e-3);
	}

	#[test]
	fn flatten_terrace_wins_over_connecting_grade() {
		let grade = PadNode::graded_reach(
			Vec2::new(-20.0, 0.0),
			Vec2::new(20.0, 0.0),
			6.0,
			8.0,
			8.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		let terrace = PadNode::rectangular_flatten(
			Vec2::ZERO,
			Vec2::new(4.0, 4.0),
			0.0,
			22.0,
			PadParams { berm: 0.0, ease: 2.0, round: 0.0 },
		);
		let complex = PadComplex::from_nodes(vec![grade, terrace]);
		assert!((complex.modify_elevation(0.0, 0.0, 0.0) - 22.0).abs() < 1e-3);
		assert!((complex.modify_elevation(0.0, 12.0, 0.0) - 8.0).abs() < 1e-3);
	}

	#[test]
	fn unioned_pads_keep_flatten_over_neighbor_ease() {
		let params = PadParams { berm: 0.0, ease: 8.0, round: 0.0 };
		let terrace = PadNode::rectangular_flatten(Vec2::ZERO, Vec2::splat(4.0), 0.0, 20.0, params);
		let neighbor =
			PadNode::rectangular_flatten(Vec2::new(10.0, 0.0), Vec2::splat(4.0), 0.0, 4.0, params);
		let first = PadComplex::from_nodes(vec![terrace]);
		let second = PadComplex::from_nodes(vec![neighbor]);
		let after_first = first.modify_elevation(0.0, 0.0, 0.0);
		assert!((after_first - 20.0).abs() < 1e-3);
		let sequential = second.modify_elevation(after_first, 0.0, 0.0);
		assert!(
			sequential + 1e-3 >= 19.0,
			"ease must not cut a pit into the already-flattened terrace: {sequential}"
		);
		let unioned = PadComplex::union_all([first, second]);
		assert!((unioned.modify_elevation(0.0, 0.0, 0.0) - 20.0).abs() < 1e-3);
		assert_eq!(unioned.classification_at(0.0, 0.0), Some(PadStage::Flatten));
	}

	#[test]
	fn shepherds_berm_keeps_hut_corner_on_flatten() {
		let half = Vec2::new(2.0, 2.0);
		let pad = PadComplex::building_skirt(Vec2::ZERO, half, 0.0, 12.0, PadParams::shepherds());
		let corner = Vec2::new(half.x - 0.1, half.y - 0.1);
		assert_eq!(pad.classification_at(corner.x, corner.y), Some(PadStage::Flatten));
		assert!((pad.modify_elevation(0.0, corner.x, corner.y) - 12.0).abs() < 1e-3);
		let past_wall = half.x + 2.0;
		assert_eq!(pad.classification_at(past_wall, 0.0), Some(PadStage::Flatten));
	}

	#[test]
	fn overlapping_grades_occupancy_blend_instead_of_jumping() {
		let params = PadParams { berm: 0.0, ease: 8.0, round: 0.0 };
		let low = PadNode::graded_reach(Vec2::ZERO, Vec2::new(40.0, 0.0), 6.0, 10.0, 10.0, params);
		let high = PadNode::graded_reach(
			Vec2::new(0.0, 8.0),
			Vec2::new(40.0, 8.0),
			6.0,
			30.0,
			30.0,
			params,
		);
		let complex = PadComplex::from_nodes(vec![low, high]);
		let a = complex.modify_elevation(0.0, 20.0, 3.0);
		let b = complex.modify_elevation(0.0, 20.0, 5.0);
		assert!(
			(a - b).abs() < 12.0,
			"adjacent samples on overlapping grades should not needle: {a} vs {b}"
		);
		let mid = complex.modify_elevation(0.0, 20.0, 4.0);
		assert!(mid > 12.0 && mid < 28.0, "overlap should mix the two grades, got {mid}");
	}

	#[test]
	fn ease_skirt_does_not_cut_a_pit_into_higher_terrain() {
		let pad = PadComplex::building_skirt(
			Vec2::ZERO,
			Vec2::new(4.0, 4.0),
			0.0,
			8.0,
			PadParams { berm: 0.0, ease: 8.0, round: 0.0 },
		);
		let hillside = 24.0;
		let h = pad.modify_elevation(hillside, 8.0, 0.0);
		assert!(
			h + 1e-3 >= hillside,
			"ease must not drop below incoming terrain (pit wall): {h} < {hillside}"
		);
		assert!((pad.modify_elevation(hillside, 0.0, 0.0) - 8.0).abs() < 1e-3);
	}

	#[test]
	fn realized_pad_follows_the_spawned_hut() -> anyhow::Result<()> {
		use buildings::{Confines, Fit, Openings};
		use procedural_common::NoiseParams;

		let center = Vec2::new(80.0, 120.0);
		let yaw = FRAC_PI_4;
		let confines = Confines::new(
			Aabb3d::from_min_max(
				Vec3::new(center.x - 3.0, 10.0, center.y - 4.0),
				Vec3::new(center.x + 3.0, 16.0, center.y + 4.0),
			),
			yaw,
			Openings::new(),
		);
		let (hut, _) = ShepherdsHut::fit_to_confines(&confines, NoiseParams::default())?;
		let placed = ShepherdsVillageBuilding {
			center_xz: center,
			yaw,
			footprint: Vec2::new(6.0, 8.0),
			ground_height: 10.0,
			building: ShepherdsBuilding::Hut(Arc::new(hut)),
		};
		let pad = PadComplex::from(&placed.pad_plan(PadParams::shepherds()));
		let local = Vec2::new(2.5, 3.5);
		let (s, c) = yaw.sin_cos();
		let spawned = center + Vec2::new(c * local.x + s * local.y, -s * local.x + c * local.y);
		assert_eq!(pad.classification_at(spawned.x, spawned.y), Some(PadStage::Flatten));
		Ok(())
	}
}
