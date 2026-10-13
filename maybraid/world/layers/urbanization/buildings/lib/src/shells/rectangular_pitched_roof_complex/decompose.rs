//! Pre-pass: split awkward authored AABBs into arms the L/T/end solvers handle.
//!
//! - Full + cross → four arm stubs meeting at the ridge crossing.
//! - Coaxial nested (longer+narrower under shorter+wider) → two lower wings
//!   that butt into the higher volume's end gables.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;

use super::geometry::{LongAxis, EPS};
use super::topology::PlanRect;

/// Expand authored volumes (cross / coaxial nested splits).
pub(super) fn decompose_volumes(volumes: &[Aabb3d]) -> Vec<Aabb3d> {
	let n = volumes.len();
	let mut used = vec![false; n];
	let mut out = Vec::with_capacity(n);
	let rects: Vec<PlanRect> = volumes.iter().map(plan_rect).collect();

	for i in 0..n {
		if used[i] {
			continue;
		}
		let mut paired = false;
		for j in (i + 1)..n {
			if used[j] {
				continue;
			}
			if let Some(parts) = try_split_cross(volumes[i], volumes[j], rects[i], rects[j]) {
				used[i] = true;
				used[j] = true;
				out.extend(parts);
				paired = true;
				break;
			}
			if let Some(parts) = try_split_coaxial(volumes[i], volumes[j], rects[i], rects[j]) {
				used[i] = true;
				used[j] = true;
				out.extend(parts);
				paired = true;
				break;
			}
		}
		if !paired {
			used[i] = true;
			out.push(volumes[i]);
		}
	}
	out
}

fn plan_rect(a: &Aabb3d) -> PlanRect {
	let min = Vec3::from(a.min);
	let max = Vec3::from(a.max);
	PlanRect { min_x: min.x, min_z: min.z, max_x: max.x, max_z: max.z }
}

fn try_split_cross(a: Aabb3d, b: Aabb3d, ra: PlanRect, rb: PlanRect) -> Option<[Aabb3d; 4]> {
	let ax = LongAxis::from_extents(ra.max_x - ra.min_x, ra.max_z - ra.min_z);
	let bx = LongAxis::from_extents(rb.max_x - rb.min_x, rb.max_z - rb.min_z);
	if ax == bx {
		return None;
	}

	let (hx, hz, rx, rz) = if ax == LongAxis::X { (a, b, ra, rb) } else { (b, a, rb, ra) };

	let overlap = rx.overlap(rz)?;
	let a_pos = rx.max_x > overlap.max_x + EPS;
	let a_neg = rx.min_x < overlap.min_x - EPS;
	let b_pos = rz.max_z > overlap.max_z + EPS;
	let b_neg = rz.min_z < overlap.min_z - EPS;
	if !(a_pos && a_neg && b_pos && b_neg) {
		return None;
	}

	let cx = 0.5 * (rz.min_x + rz.max_x);
	let cz = 0.5 * (rx.min_z + rx.max_z);
	if rx.max_x - cx < EPS || cx - rx.min_x < EPS || rz.max_z - cz < EPS || cz - rz.min_z < EPS {
		return None;
	}

	let hy0 = Vec3::from(hx.min).y;
	let hy1 = Vec3::from(hx.max).y;
	let zy0 = Vec3::from(hz.min).y;
	let zy1 = Vec3::from(hz.max).y;

	Some([
		Aabb3d::from_min_max(Vec3::new(cx, hy0, rx.min_z), Vec3::new(rx.max_x, hy1, rx.max_z)),
		Aabb3d::from_min_max(Vec3::new(rx.min_x, hy0, rx.min_z), Vec3::new(cx, hy1, rx.max_z)),
		Aabb3d::from_min_max(Vec3::new(rz.min_x, zy0, cz), Vec3::new(rz.max_x, zy1, rz.max_z)),
		Aabb3d::from_min_max(Vec3::new(rz.min_x, zy0, rz.min_z), Vec3::new(rz.max_x, zy1, cz)),
	])
}

/// Longer+narrower under shorter+wider → keep the higher/wider box, split the
/// lower into two wings that butt its end gables.
fn try_split_coaxial(a: Aabb3d, b: Aabb3d, ra: PlanRect, rb: PlanRect) -> Option<Vec<Aabb3d>> {
	let ax = LongAxis::from_extents(ra.max_x - ra.min_x, ra.max_z - ra.min_z);
	let bx = LongAxis::from_extents(rb.max_x - rb.min_x, rb.max_z - rb.min_z);
	if ax != bx {
		return None;
	}

	let (mid_a, mid_b, span_a, span_b, long_a, long_b) = match ax {
		LongAxis::X => (
			0.5 * (ra.min_z + ra.max_z),
			0.5 * (rb.min_z + rb.max_z),
			ra.max_z - ra.min_z,
			rb.max_z - rb.min_z,
			ra.max_x - ra.min_x,
			rb.max_x - rb.min_x,
		),
		LongAxis::Z => (
			0.5 * (ra.min_x + ra.max_x),
			0.5 * (rb.min_x + rb.max_x),
			ra.max_x - ra.min_x,
			rb.max_x - rb.min_x,
			ra.max_z - ra.min_z,
			rb.max_z - rb.min_z,
		),
	};
	if (mid_a - mid_b).abs() > 0.05 {
		return None;
	}
	if (span_a - span_b).abs() < EPS || (long_a - long_b).abs() < EPS {
		return None;
	}

	// Higher/wider = larger short span; lower/runner = longer long span.
	let (cap, run, rc, rr) = if span_a > span_b { (a, b, ra, rb) } else { (b, a, rb, ra) };
	let cap_long = match ax {
		LongAxis::X => rc.max_x - rc.min_x,
		LongAxis::Z => rc.max_z - rc.min_z,
	};
	let run_long = match ax {
		LongAxis::X => rr.max_x - rr.min_x,
		LongAxis::Z => rr.max_z - rr.min_z,
	};
	// Need the runner to extend past the cap on both long ends.
	if run_long <= cap_long + EPS {
		return None;
	}

	let ry0 = Vec3::from(run.min).y;
	let ry1 = Vec3::from(run.max).y;

	match ax {
		LongAxis::X => {
			if rr.min_x >= rc.min_x - EPS || rr.max_x <= rc.max_x + EPS {
				return None;
			}
			let wing_neg = Aabb3d::from_min_max(
				Vec3::new(rr.min_x, ry0, rr.min_z),
				Vec3::new(rc.min_x, ry1, rr.max_z),
			);
			let wing_pos = Aabb3d::from_min_max(
				Vec3::new(rc.max_x, ry0, rr.min_z),
				Vec3::new(rr.max_x, ry1, rr.max_z),
			);
			if rc.min_x - rr.min_x < EPS || rr.max_x - rc.max_x < EPS {
				return None;
			}
			Some(vec![cap, wing_neg, wing_pos])
		}
		LongAxis::Z => {
			if rr.min_z >= rc.min_z - EPS || rr.max_z <= rc.max_z + EPS {
				return None;
			}
			let wing_neg = Aabb3d::from_min_max(
				Vec3::new(rr.min_x, ry0, rr.min_z),
				Vec3::new(rr.max_x, ry1, rc.min_z),
			);
			let wing_pos = Aabb3d::from_min_max(
				Vec3::new(rr.min_x, ry0, rc.max_z),
				Vec3::new(rr.max_x, ry1, rr.max_z),
			);
			if rc.min_z - rr.min_z < EPS || rr.max_z - rc.max_z < EPS {
				return None;
			}
			Some(vec![cap, wing_neg, wing_pos])
		}
	}
}
