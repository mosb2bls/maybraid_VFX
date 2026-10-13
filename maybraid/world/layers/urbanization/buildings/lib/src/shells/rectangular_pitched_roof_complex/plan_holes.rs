//! Subtract axis-aligned plan holes from authored roof massing boxes.
//!
//! Pitch openings clip a face after geometry is solved. Keep footprints need the
//! massing itself to go around the tower, so we CSG the AABBs in \(XZ\) first.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

const EPS: f32 = 1e-4;
const MIN_SPAN: f32 = 0.4;

/// Punch `holes` out of `volumes` in plan (Y of each volume is kept).
pub(super) fn punch_plan_holes(
	volumes: Vec<Aabb3d>,
	holes: impl IntoIterator<Item = Aabb3d>,
) -> Vec<Aabb3d> {
	let mut cur = volumes;
	for hole in holes {
		cur = cur.into_iter().flat_map(|vol| subtract_xz(vol, hole)).collect();
	}
	cur
}

fn subtract_xz(vol: Aabb3d, hole: Aabb3d) -> Vec<Aabb3d> {
	let vmin = Vec3::from(vol.min);
	let vmax = Vec3::from(vol.max);
	let hmin = Vec3::from(hole.min);
	let hmax = Vec3::from(hole.max);

	let hx0 = hmin.x.max(vmin.x);
	let hx1 = hmax.x.min(vmax.x);
	let hz0 = hmin.z.max(vmin.z);
	let hz1 = hmax.z.min(vmax.z);
	if hx1 - hx0 <= EPS || hz1 - hz0 <= EPS {
		return vec![vol];
	}

	let y0 = vmin.y;
	let y1 = vmax.y;
	let mut out = Vec::new();
	let mut push = |x0: f32, x1: f32, z0: f32, z1: f32| {
		if x1 - x0 > MIN_SPAN && z1 - z0 > MIN_SPAN {
			out.push(Aabb3d::from_min_max(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1)));
		}
	};
	push(vmin.x, hx0, vmin.z, vmax.z);
	push(hx1, vmax.x, vmin.z, vmax.z);
	push(hx0, hx1, vmin.z, hz0);
	push(hx0, hx1, hz1, vmax.z);
	out
}

#[cfg(test)]
mod tests {
	use super::*;

	fn xz_covers(volume: &Aabb3d, p: Vec3) -> bool {
		p.x > volume.min.x && p.x < volume.max.x && p.z > volume.min.z && p.z < volume.max.z
	}

	#[test]
	fn corner_hole_leaves_the_rest_of_the_bar() {
		let bar = Aabb3d::from_min_max(Vec3::new(-8.0, 2.0, 4.0), Vec3::new(8.0, 4.0, 8.0));
		let hole = Aabb3d::from_min_max(Vec3::new(4.0, 0.0, 4.0), Vec3::new(8.0, 5.0, 8.0));
		let out = punch_plan_holes(vec![bar], [hole]);
		assert!(!out.iter().any(|v| xz_covers(v, Vec3::new(6.0, 0.0, 6.0))));
		assert!(out.iter().any(|v| xz_covers(v, Vec3::new(0.0, 0.0, 6.0))));
	}

	#[test]
	fn four_quadrant_squares_clear_the_center() {
		let bar = Aabb3d::from_min_max(Vec3::new(-10.0, 1.0, -10.0), Vec3::new(10.0, 3.0, 10.0));
		let r = 3.0;
		let holes = [
			Aabb3d::from_min_max(Vec3::new(-r, 0.0, -r), Vec3::new(0.0, 4.0, 0.0)),
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, -r), Vec3::new(r, 4.0, 0.0)),
			Aabb3d::from_min_max(Vec3::new(-r, 0.0, 0.0), Vec3::new(0.0, 4.0, r)),
			Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(r, 4.0, r)),
		];
		let out = punch_plan_holes(vec![bar], holes);
		assert!(!out.iter().any(|v| xz_covers(v, Vec3::ZERO)));
		assert!(out.iter().any(|v| xz_covers(v, Vec3::new(6.0, 0.0, 6.0))));
	}
}
