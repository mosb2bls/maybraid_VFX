//! Shared XZ plan helpers for usage-area packing (AABB lifts, passages, noise).

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};
use procedural_common::NoiseParams;

use crate::fit::Confines;
use crate::openings::{Opening, OpeningId, OpeningLabel, Openings};
use crate::paneling::DEFAULT_PANEL_THICKNESS;

const EPS: f32 = 1e-3;

// Re-export residential defaults — canonical defs live in [`super::plan_access`].
pub use super::plan_access::{
	door_leaf_height, DOOR_HEIGHT, DOOR_SIZE_SCALE, DOOR_WIDTH, DOOR_WIDTH_MAX, MIN_ROOM,
};

/// Floor XZ of an axis-aligned 3D AABB (`y` up).
pub fn host_xz(bounds: &Aabb3d) -> Aabb2d {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) }
}

/// Lift an XZ rect into confines at height `[y0, y1]`.
pub fn confines_from_xz(xz: Aabb2d, y0: f32, y1: f32, roll: f32, openings: &Openings) -> Confines {
	Confines::new(
		Aabb3d::from_min_max(Vec3::new(xz.min.x, y0, xz.min.y), Vec3::new(xz.max.x, y1, xz.max.y)),
		roll,
		openings.clone(),
	)
}

/// Deterministic per-cell noise offset.
pub fn noise_for_cell(noise: NoiseParams, cell: i32) -> NoiseParams {
	NoiseParams { seed: noise.seed.wrapping_add(cell.wrapping_mul(97)), ..noise }
}

/// Approximate equality for plan AABBs (entry carve bookkeeping).
pub fn aabb2_near_eq(a: Aabb2d, b: Aabb2d) -> bool {
	(a.min.x - b.min.x).abs() < 0.05
		&& (a.min.y - b.min.y).abs() < 0.05
		&& (a.max.x - b.max.x).abs() < 0.05
		&& (a.max.y - b.max.y).abs() < 0.05
}

/// Passage opening centered on a shared edge span between two plan rects.
pub fn connecting_passage(
	scope: &str,
	kind: &str,
	along_x: bool,
	lo: f32,
	hi: f32,
	mid: f32,
	y0: f32,
	y1: f32,
	id_tag: impl AsRef<str>,
) -> Option<(OpeningId, Opening)> {
	let shared = hi - lo;
	if shared < DOOR_WIDTH * 0.7 + EPS {
		return None;
	}
	let clear = DOOR_WIDTH.min(shared - 0.1).max(0.7);
	let center = 0.5 * (lo + hi);
	let half = clear * 0.5;
	let door_lo = (center - half).max(lo);
	let door_hi = (center + half).min(hi);
	let half_d = (DEFAULT_PANEL_THICKNESS * 0.5 + 0.06).max(0.12);
	let door_h = door_leaf_height(y1 - y0).min(DOOR_HEIGHT);
	let bounds = if along_x {
		Aabb3d::from_min_max(
			Vec3::new(door_lo, y0, mid - half_d),
			Vec3::new(door_hi, y0 + door_h, mid + half_d),
		)
	} else {
		Aabb3d::from_min_max(
			Vec3::new(mid - half_d, y0, door_lo),
			Vec3::new(mid + half_d, y0 + door_h, door_hi),
		)
	};
	Some((OpeningId::scoped(scope, kind, id_tag), Opening::new(bounds, OpeningLabel::Passage)))
}

/// South-face synthetic passage so a rect always has a port for open packing.
pub fn synthetic_edge_passage(
	scope: &str,
	kind: &str,
	xz: Aabb2d,
	y0: f32,
	y1: f32,
	id_tag: impl AsRef<str>,
) -> (OpeningId, Opening) {
	let door_w = DOOR_WIDTH.min(xz.max.x - xz.min.x - 0.2).max(0.7);
	let half = door_w * 0.5;
	let cx = 0.5 * (xz.min.x + xz.max.x);
	let door_h = door_leaf_height(y1 - y0);
	let half_d = 0.12_f32;
	(
		OpeningId::scoped(scope, kind, id_tag),
		Opening::new(
			Aabb3d::from_min_max(
				Vec3::new(cx - half, y0, xz.min.y - half_d),
				Vec3::new(cx + half, y0 + door_h, xz.min.y + half_d),
			),
			OpeningLabel::Passage,
		),
	)
}
