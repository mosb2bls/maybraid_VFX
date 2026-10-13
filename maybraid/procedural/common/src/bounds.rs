//! Axis-aligned packing helpers: inflate / inset / max empty rectangle.
//!
//! Typical flow for usage areas:
//! 1. Build exclude AABBs (furniture, counters, …).
//! 2. [`max_empty_rect2`] / [`max_empty_rect2_with_clearance`] for the largest free box.
//! 3. Strip down with [`inset_aabb2`] / [`clamp_min_size2`] as domain policy requires.

use bevy_math::bounding::{Aabb2d, Aabb3d};
use bevy_math::{Vec2, Vec3};

const EPS: f32 = 1e-4;
const MIN_SPAN: f32 = 1e-3;

/// Inflate `a` uniformly by `pad` on both axes.
pub fn inflate_aabb2(a: Aabb2d, pad: f32) -> Aabb2d {
	let pad = pad.max(0.0);
	Aabb2d { min: a.min - Vec2::splat(pad), max: a.max + Vec2::splat(pad) }
}

/// Inset `a` uniformly by `pad`. Returns [`None`] if either axis collapses.
pub fn inset_aabb2(a: Aabb2d, pad: f32) -> Option<Aabb2d> {
	let pad = pad.max(0.0);
	let min = a.min + Vec2::splat(pad);
	let max = a.max - Vec2::splat(pad);
	if max.x - min.x < MIN_SPAN || max.y - min.y < MIN_SPAN {
		return None;
	}
	Some(Aabb2d { min, max })
}

/// Smallest axis-aligned box containing `a` and `b`.
pub fn union_aabb3(a: Aabb3d, b: Aabb3d) -> Aabb3d {
	Aabb3d::from_min_max(a.min.min(b.min), a.max.max(b.max))
}

/// Smallest axis-aligned box containing every item, if any.
pub fn union_aabb3_iter(bounds: impl IntoIterator<Item = Aabb3d>) -> Option<Aabb3d> {
	bounds.into_iter().reduce(union_aabb3)
}

/// True when the open rectangles overlap (strict, with a small epsilon).
pub fn intersects_aabb2(a: Aabb2d, b: Aabb2d) -> bool {
	a.min.x < b.max.x - EPS
		&& b.min.x < a.max.x - EPS
		&& a.min.y < b.max.y - EPS
		&& b.min.y < a.max.y - EPS
}

/// True when rectangles overlap or share an edge (closed contact).
pub fn touches_aabb2(a: Aabb2d, b: Aabb2d) -> bool {
	a.min.x <= b.max.x + EPS
		&& b.min.x <= a.max.x + EPS
		&& a.min.y <= b.max.y + EPS
		&& b.min.y <= a.max.y + EPS
}

/// Area of an axis-aligned rect.
pub fn aabb2_area(a: Aabb2d) -> f32 {
	((a.max.x - a.min.x).max(0.0)) * ((a.max.y - a.min.y).max(0.0))
}

/// Shrink toward center until both extents are ≥ `min_size` (or [`None`] if impossible).
pub fn clamp_min_size2(a: Aabb2d, min_size: Vec2) -> Option<Aabb2d> {
	let size = a.max - a.min;
	if size.x + EPS < min_size.x || size.y + EPS < min_size.y {
		return None;
	}
	Some(a)
}

/// Largest-area empty axis-aligned rect inside `host` that avoids every `exclude`.
///
/// Enumerates candidate corners from the host + exclude edge grid
/// (exact for axis-aligned obstacles; \(O(n^4)\) in exclude count).
pub fn max_empty_rect2(host: Aabb2d, excludes: &[Aabb2d]) -> Option<Aabb2d> {
	max_empty_rect2_by(host, excludes, aabb2_area)
}

/// Inflate each exclude by `clearance`, then [`max_empty_rect2`].
pub fn max_empty_rect2_with_clearance(
	host: Aabb2d,
	excludes: &[Aabb2d],
	clearance: f32,
) -> Option<Aabb2d> {
	let cuts: Vec<Aabb2d> = excludes.iter().copied().map(|e| inflate_aabb2(e, clearance)).collect();
	max_empty_rect2(host, &cuts)
}

/// Like [`max_empty_rect2`], but picks the candidate maximizing `score`.
pub fn max_empty_rect2_by(
	host: Aabb2d,
	excludes: &[Aabb2d],
	score: impl Fn(Aabb2d) -> f32,
) -> Option<Aabb2d> {
	let mut xs = vec![host.min.x, host.max.x];
	let mut ys = vec![host.min.y, host.max.y];
	for ex in excludes {
		xs.push(ex.min.x.clamp(host.min.x, host.max.x));
		xs.push(ex.max.x.clamp(host.min.x, host.max.x));
		ys.push(ex.min.y.clamp(host.min.y, host.max.y));
		ys.push(ex.max.y.clamp(host.min.y, host.max.y));
	}
	xs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	ys.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
	xs.dedup_by(|a, b| (*a - *b).abs() < EPS);
	ys.dedup_by(|a, b| (*a - *b).abs() < EPS);

	let mut best: Option<(f32, Aabb2d)> = None;
	for i in 0..xs.len() {
		for j in (i + 1)..xs.len() {
			for k in 0..ys.len() {
				for l in (k + 1)..ys.len() {
					if xs[j] - xs[i] < MIN_SPAN || ys[l] - ys[k] < MIN_SPAN {
						continue;
					}
					let cand =
						Aabb2d { min: Vec2::new(xs[i], ys[k]), max: Vec2::new(xs[j], ys[l]) };
					if excludes.iter().any(|e| intersects_aabb2(cand, *e)) {
						continue;
					}
					let s = score(cand);
					if best.map(|(b, _)| s > b + 1e-6).unwrap_or(true) {
						best = Some((s, cand));
					}
				}
			}
		}
	}
	best.map(|(_, r)| r)
}

/// Which two axes of an [`Aabb3d`] form the plan rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanAxes {
	/// \(X\) × \(Z\) (typical building plan; \(Y\) is height).
	XZ,
	/// \(X\) × \(Y\).
	XY,
	/// \(Y\) × \(Z\).
	YZ,
}

/// Project `a` onto [`PlanAxes`].
pub fn aabb3_to_plan(a: &Aabb3d, axes: PlanAxes) -> Aabb2d {
	let min = Vec3::from(a.min);
	let max = Vec3::from(a.max);
	match axes {
		PlanAxes::XZ => Aabb2d { min: Vec2::new(min.x, min.z), max: Vec2::new(max.x, max.z) },
		PlanAxes::XY => Aabb2d { min: Vec2::new(min.x, min.y), max: Vec2::new(max.x, max.y) },
		PlanAxes::YZ => Aabb2d { min: Vec2::new(min.y, min.z), max: Vec2::new(max.y, max.z) },
	}
}

/// Embed a plan rect into `host`, keeping the unused axis from `host`.
pub fn plan_to_aabb3(host: &Aabb3d, plan: Aabb2d, axes: PlanAxes) -> Aabb3d {
	let hmin = Vec3::from(host.min);
	let hmax = Vec3::from(host.max);
	match axes {
		PlanAxes::XZ => Aabb3d::from_min_max(
			Vec3::new(plan.min.x, hmin.y, plan.min.y),
			Vec3::new(plan.max.x, hmax.y, plan.max.y),
		),
		PlanAxes::XY => Aabb3d::from_min_max(
			Vec3::new(plan.min.x, plan.min.y, hmin.z),
			Vec3::new(plan.max.x, plan.max.y, hmax.z),
		),
		PlanAxes::YZ => Aabb3d::from_min_max(
			Vec3::new(hmin.x, plan.min.x, plan.min.y),
			Vec3::new(hmax.x, plan.max.x, plan.max.y),
		),
	}
}

/// Max empty plan box in `host` avoiding `excludes`, with uniform plan clearance.
pub fn max_empty_aabb3_plan(
	host: &Aabb3d,
	excludes: &[Aabb3d],
	axes: PlanAxes,
	clearance: f32,
) -> Option<Aabb3d> {
	let host2 = aabb3_to_plan(host, axes);
	let cuts: Vec<Aabb2d> = excludes.iter().map(|e| aabb3_to_plan(e, axes)).collect();
	let plan = max_empty_rect2_with_clearance(host2, &cuts, clearance)?;
	Some(plan_to_aabb3(host, plan, axes))
}

fn y_overlap_open(a: Aabb2d, b: Aabb2d) -> bool {
	a.min.y < b.max.y - EPS && b.min.y < a.max.y - EPS
}

fn x_overlap_open(a: Aabb2d, b: Aabb2d) -> bool {
	a.min.x < b.max.x - EPS && b.min.x < a.max.x - EPS
}

/// Expand `seed` inside `host` until blocked by `excludes` or the host rim.
///
/// Assumes `seed` ⊂ `host` and does not intersect `excludes`. Result always
/// contains `seed`. Iterates axis expansions so growing one axis can unlock another.
pub fn grow_aabb2(host: Aabb2d, seed: Aabb2d, excludes: &[Aabb2d]) -> Aabb2d {
	let mut r = Aabb2d { min: seed.min.max(host.min), max: seed.max.min(host.max) };
	if r.max.x - r.min.x < MIN_SPAN || r.max.y - r.min.y < MIN_SPAN {
		return seed;
	}
	for _ in 0..16 {
		let prev = r;
		// −X
		let mut lim = host.min.x;
		for e in excludes {
			if y_overlap_open(r, *e) && e.max.x < r.min.x + EPS {
				lim = lim.max(e.max.x);
			}
		}
		r.min.x = lim.min(r.min.x);
		// +X
		lim = host.max.x;
		for e in excludes {
			if y_overlap_open(r, *e) && e.min.x > r.max.x - EPS {
				lim = lim.min(e.min.x);
			}
		}
		r.max.x = lim.max(r.max.x);
		// −Y
		lim = host.min.y;
		for e in excludes {
			if x_overlap_open(r, *e) && e.max.y < r.min.y + EPS {
				lim = lim.max(e.max.y);
			}
		}
		r.min.y = lim.min(r.min.y);
		// +Y
		lim = host.max.y;
		for e in excludes {
			if x_overlap_open(r, *e) && e.min.y > r.max.y - EPS {
				lim = lim.min(e.min.y);
			}
		}
		r.max.y = lim.max(r.max.y);
		if (r.min - prev.min).length_squared() < EPS * EPS
			&& (r.max - prev.max).length_squared() < EPS * EPS
		{
			break;
		}
	}
	r
}

/// Alternate expanding two seeds into free space. Each treats the other as an
/// exclude; `hard_a` / `hard_b` are permanent obstacles for that side.
///
/// Useful after placing seed boxes (max-empty / policy) so leftover dead space
/// is absorbed without a full guillotine partition.
pub fn grow_aabb2_pair(
	host: Aabb2d,
	a: Aabb2d,
	b: Aabb2d,
	hard_a: &[Aabb2d],
	hard_b: &[Aabb2d],
	rounds: usize,
) -> (Aabb2d, Aabb2d) {
	let mut a = a;
	let mut b = b;
	for _ in 0..rounds.max(1) {
		let mut ex_a: Vec<Aabb2d> = hard_a.to_vec();
		ex_a.push(b);
		let a_next = grow_aabb2(host, a, &ex_a);
		let mut ex_b: Vec<Aabb2d> = hard_b.to_vec();
		ex_b.push(a_next);
		let b_next = grow_aabb2(host, b, &ex_b);
		let stable = (a_next.min - a.min).length_squared() < EPS * EPS
			&& (a_next.max - a.max).length_squared() < EPS * EPS
			&& (b_next.min - b.min).length_squared() < EPS * EPS
			&& (b_next.max - b.max).length_squared() < EPS * EPS;
		a = a_next;
		b = b_next;
		if stable {
			break;
		}
	}
	(a, b)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn max_empty_claims_gap_and_band_behind_two_excludes() {
		let host = Aabb2d { min: Vec2::ZERO, max: Vec2::new(10.0, 6.0) };
		let excludes = [
			Aabb2d { min: Vec2::new(1.0, 0.0), max: Vec2::new(2.5, 1.0) },
			Aabb2d { min: Vec2::new(5.5, 0.0), max: Vec2::new(7.0, 1.0) },
		];
		let free = max_empty_rect2_with_clearance(host, &excludes, 1.0).unwrap();
		// Behind both (y ≥ 2) should win as a wide band.
		assert!(free.max.x - free.min.x >= 9.0, "width {}", free.max.x - free.min.x);
		assert!(free.min.y >= 1.9, "should sit above clearance");
	}

	#[test]
	fn xz_plan_roundtrip_preserves_height() {
		let host = Aabb3d::from_min_max(Vec3::ZERO, Vec3::new(4.0, 3.0, 5.0));
		let obstacle = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(4.0, 3.0, 1.0));
		let free = max_empty_aabb3_plan(&host, &[obstacle], PlanAxes::XZ, 1.0).unwrap();
		assert!((Vec3::from(free.min).y - 0.0).abs() < 1e-4);
		assert!((Vec3::from(free.max).y - 3.0).abs() < 1e-4);
		assert!(Vec3::from(free.min).z >= 1.9);
	}

	#[test]
	fn union_aabb3_spans_both_boxes() {
		let a = Aabb3d::from_min_max(Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 1.0));
		let b = Aabb3d::from_min_max(Vec3::new(2.0, -1.0, 3.0), Vec3::new(4.0, 2.0, 5.0));
		let u = union_aabb3(a, b);
		assert_eq!(Vec3::from(u.min), Vec3::new(0.0, -1.0, 0.0));
		assert_eq!(Vec3::from(u.max), Vec3::new(4.0, 2.0, 5.0));
	}

	#[test]
	fn inset_collapses_to_none() {
		let a = Aabb2d { min: Vec2::ZERO, max: Vec2::new(1.0, 1.0) };
		assert!(inset_aabb2(a, 0.6).is_none());
	}

	#[test]
	fn grow_fills_dead_space_beside_seed() {
		let host = Aabb2d { min: Vec2::ZERO, max: Vec2::new(10.0, 6.0) };
		let wall = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(10.0, 1.0) };
		let seed = Aabb2d { min: Vec2::new(4.0, 1.0), max: Vec2::new(6.0, 2.0) };
		let grown = grow_aabb2(host, seed, &[wall]);
		assert!((grown.min.x - 0.0).abs() < 1e-3);
		assert!((grown.max.x - 10.0).abs() < 1e-3);
		assert!((grown.min.y - 1.0).abs() < 1e-3);
		assert!((grown.max.y - 6.0).abs() < 1e-3);
	}

	#[test]
	fn grow_pair_splits_remainder() {
		let host = Aabb2d { min: Vec2::ZERO, max: Vec2::new(10.0, 6.0) };
		let a = Aabb2d { min: Vec2::new(0.0, 0.0), max: Vec2::new(2.0, 2.0) };
		let b = Aabb2d { min: Vec2::new(8.0, 4.0), max: Vec2::new(10.0, 6.0) };
		let (ga, gb) = grow_aabb2_pair(host, a, b, &[], &[], 8);
		// Together they should cover the host (axis-aligned pair grow).
		assert!(aabb2_area(ga) + aabb2_area(gb) >= 10.0 * 6.0 - 1.0);
		assert!(!intersects_aabb2(ga, gb));
	}
}
