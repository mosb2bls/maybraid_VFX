//! Tessellated fill of a planar triangle with posed unit right-triangle kits.
//!
//! Corners are **2D panel-space** points \((X, Z)\). Mapping that plane into 3D
//! (pitch, wall stand-up, etc.) is a higher-order concern via parent [`Placement`].
//!
//! Kit footprint: \(X \in [0, 1]\), \(Z \in [-1, 0]\) (right angle at the origin;
//! third corner at local \((0, 0, -1)\)). Matches urban panel right-triangle GLBs.
//!
//! Decomposition:
//! 1. If a corner is already a right angle, emit one scaled kit there.
//! 2. Otherwise altitude-split on the longest edge into two right-triangle kits.

use bevy_math::{Vec2, Vec3};

use crate::panels::geometry::{PanelGeometry, RightTriangle};
use crate::panels::placement::yaw_along_xz;
use crate::placed::{Placed, Placement};

/// Cosine near zero → right angle (relative to unit edge directions).
const RIGHT_ANGLE_COS_EPS: f32 = 1e-4;

/// Three panel-space corners \((X, Z)\) filled by right-triangle kits.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TessellatedTriangle {
	pub a: Vec2,
	pub b: Vec2,
	pub c: Vec2,
}

impl Default for TessellatedTriangle {
	fn default() -> Self {
		Self { a: Vec2::ZERO, b: Vec2::new(1.0, 0.0), c: Vec2::new(0.0, -1.0) }
	}
}

impl TessellatedTriangle {
	pub fn new(a: Vec2, b: Vec2, c: Vec2) -> Self {
		Self { a, b, c }
	}

	/// Expand into posed [`RightTriangle`] leaves (identity parent).
	pub fn decompose(self) -> Vec<Placed<PanelGeometry>> {
		let ab = self.b - self.a;
		let ac = self.c - self.a;
		if (ab.x * ac.y - ab.y * ac.x).abs() < 1e-12 {
			return Vec::new();
		}

		// Prefer an existing right-angle vertex → one kit (legs along the two edges).
		let corners = [self.a, self.b, self.c];
		for i in 0..3 {
			let at = corners[i];
			let p = corners[(i + 1) % 3];
			let q = corners[(i + 2) % 3];
			if is_right_angle(at, p, q) {
				return place_right_triangle(at, p, q);
			}
		}

		// General case: altitude to the longest edge → two right-triangle kits.
		let edges = [(self.a, self.b, self.c), (self.b, self.c, self.a), (self.c, self.a, self.b)];
		let (ei, _) = edges
			.iter()
			.enumerate()
			.max_by(|(_, (p0, p1, _)), (_, (q0, q1, _))| {
				(*p1 - *p0)
					.length_squared()
					.partial_cmp(&(*q1 - *q0).length_squared())
					.unwrap_or(std::cmp::Ordering::Equal)
			})
			.expect("three edges");

		let (p0, p1, apex) = edges[ei];
		let edge = p1 - p0;
		let edge_len2 = edge.length_squared();
		if edge_len2 < 1e-12 {
			return Vec::new();
		}
		let t = ((apex - p0).dot(edge) / edge_len2).clamp(0.0, 1.0);
		let foot = p0 + edge * t;

		let mut out = Vec::new();
		out.extend(place_right_triangle(foot, p0, apex));
		out.extend(place_right_triangle(foot, p1, apex));
		out
	}
}

fn is_right_angle(at: Vec2, p: Vec2, q: Vec2) -> bool {
	let u = p - at;
	let v = q - at;
	let denom = u.length() * v.length();
	if denom < 1e-8 {
		return false;
	}
	(u.dot(v) / denom).abs() < RIGHT_ANGLE_COS_EPS
}

/// One kit for the right triangle with right angle at `right_angle`, legs to `leg_u` / `leg_v`.
///
/// Kit \(+X\) maps along one leg; kit \(-Z\) (local \((0,0,-1)\)) along the other.
/// Panel \(+Y\) is thickness; yaw alone orients the kit in the \(XZ\) plane.
fn place_right_triangle(right_angle: Vec2, leg_u: Vec2, leg_v: Vec2) -> Vec<Placed<PanelGeometry>> {
	let u = leg_u - right_angle;
	let v = leg_v - right_angle;
	let u_len = u.length();
	let v_len = v.length();
	if u_len < 1e-6 || v_len < 1e-6 {
		return Vec::new();
	}

	// After yaw aligns \(+X\) with `x_leg`, local \(-Z\) follows the clockwise
	// perpendicular in \(XZ\). Choose `x_leg` so that perpendicular matches the
	// other leg (GLB is \(Z \in [-1, 0]\), not \(+Z\)).
	let cross = u.x * v.y - u.y * v.x;
	let (x_leg, sx, sz) = if cross <= 0.0 { (u, u_len, v_len) } else { (v, v_len, u_len) };
	let x_dir = x_leg / sx;

	let yaw = yaw_along_xz(x_dir.x, x_dir.y);
	vec![Placed::with_placement(
		PanelGeometry::RightTriangle(RightTriangle { mirror: None }),
		Placement::new(Vec3::new(right_angle.x, 0.0, right_angle.y), yaw)
			.with_scale(Vec3::new(sx, 1.0, sz)),
	)]
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::scene_children::pose;

	/// Kit corners \((0,0)\), \((1,0)\), \((0,-1)\) after placement TRS (panel \(X,Z\)).
	fn kit_xz_corners(placed: &Placed<PanelGeometry>) -> [Vec2; 3] {
		let t = pose(placed.placement);
		let map = |lx: f32, lz: f32| {
			let p = t * Vec3::new(lx, 0.0, lz);
			Vec2::new(p.x, p.z)
		};
		[map(0.0, 0.0), map(1.0, 0.0), map(0.0, -1.0)]
	}

	fn assert_same_triangle(got: [Vec2; 3], want: [Vec2; 3]) {
		// Match by nearest corner — bit-sort is unstable when TRS leaves ~1e-7 noise
		// (e.g. 1.9999998 sorts before 2.0 and pairwise compare fails).
		for w in want {
			assert!(
				got.iter().any(|g| (*g - w).length() < 1e-4),
				"missing corner {w:?} in got {got:?} (want {want:?})"
			);
		}
		for g in got {
			assert!(
				want.iter().any(|w| (*w - g).length() < 1e-4),
				"unexpected corner {g:?} in got {got:?} (want {want:?})"
			);
		}
	}

	#[test]
	fn playground_right_triangle_is_single_kit() {
		// `/show tessellated-triangle --a 0,0 --b 3,0 --c 0,2`
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(3.0, 0.0), Vec2::new(0.0, 2.0));
		let pieces = t.decompose();
		assert_eq!(pieces.len(), 1);
		assert!(matches!(pieces[0].geom, PanelGeometry::RightTriangle(_)));
		assert_same_triangle(
			kit_xz_corners(&pieces[0]),
			[Vec2::ZERO, Vec2::new(3.0, 0.0), Vec2::new(0.0, 2.0)],
		);
	}

	#[test]
	fn unit_right_triangle_matches_kit_footprint() {
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0));
		let pieces = t.decompose();
		assert_eq!(pieces.len(), 1);
		assert!((pieces[0].scale() - Vec3::ONE).length() < 1e-4);
		assert!(pieces[0].yaw().abs() < 1e-4);
		assert_same_triangle(
			kit_xz_corners(&pieces[0]),
			[Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(0.0, -1.0)],
		);
	}

	#[test]
	fn right_angle_not_at_origin_still_one_kit() {
		// Right angle at B=(2,0): legs to A=(0,0) and C=(2,3).
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(2.0, 3.0));
		let pieces = t.decompose();
		assert_eq!(pieces.len(), 1);
		assert_same_triangle(
			kit_xz_corners(&pieces[0]),
			[Vec2::ZERO, Vec2::new(2.0, 0.0), Vec2::new(2.0, 3.0)],
		);
	}

	#[test]
	fn obtuse_playground_two_kits_cover_corners() {
		// `/show tessellated-triangle --a 0,0 --b 3,0 --c -1,1`
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(3.0, 0.0), Vec2::new(-1.0, 1.0));
		let pieces = t.decompose();
		assert_eq!(pieces.len(), 2);

		let mut covered = Vec::new();
		for p in &pieces {
			covered.extend(kit_xz_corners(p));
		}
		for want in [t.a, t.b, t.c] {
			assert!(
				covered.iter().any(|g| (*g - want).length() < 1e-3),
				"missing corner {want:?} in kit corners {covered:?}"
			);
		}

		// Both kit centroids lie inside / on the original triangle (no flipped lambda).
		for p in &pieces {
			let c = kit_xz_corners(p);
			let mid = (c[0] + c[1] + c[2]) / 3.0;
			assert!(
				point_inside_or_on(mid, [t.a, t.b, t.c]),
				"kit centroid {mid:?} outside triangle (lambda / flipped half)"
			);
		}
	}

	#[test]
	fn acute_non_right_altitude_splits_into_two() {
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(4.0, 0.0), Vec2::new(1.5, 3.0));
		let pieces = t.decompose();
		assert_eq!(pieces.len(), 2);
		assert!(pieces.iter().all(|p| matches!(p.geom, PanelGeometry::RightTriangle(_))));
		assert!(pieces.iter().all(|p| p.scale().x > 1e-4 && p.scale().z > 1e-4));

		let f0 = kit_xz_corners(&pieces[0])[0];
		let f1 = kit_xz_corners(&pieces[1])[0];
		assert!((f0 - f1).length() < 1e-4);

		let corners = [t.a, t.b, t.c];
		for p in &pieces {
			let mid = {
				let c = kit_xz_corners(p);
				(c[0] + c[1] + c[2]) / 3.0
			};
			assert!(
				point_inside_or_on(mid, corners),
				"kit centroid {mid:?} escaped triangle {corners:?}"
			);
		}
	}

	#[test]
	fn degenerate_is_empty() {
		let t = TessellatedTriangle::new(Vec2::ZERO, Vec2::new(1.0, 0.0), Vec2::new(2.0, 0.0));
		assert!(t.decompose().is_empty());
	}

	fn point_inside_or_on(p: Vec2, corners: [Vec2; 3]) -> bool {
		let [a, b, c] = corners;
		let area = |u: Vec2, v: Vec2, w: Vec2| (v - u).perp_dot(w - u);
		let a0 = area(a, b, c);
		if a0.abs() < 1e-12 {
			return false;
		}
		let a1 = area(p, b, c);
		let a2 = area(a, p, c);
		let a3 = area(a, b, p);
		(a1.signum() == a0.signum() || a1.abs() < 1e-3)
			&& (a2.signum() == a0.signum() || a2.abs() < 1e-3)
			&& (a3.signum() == a0.signum() || a3.abs() < 1e-3)
			&& (a1 + a2 + a3 - a0).abs() < 1e-2
	}
}
