//! Pad support geometry: rectangular terraces and graded connecting reaches.

use bevy::math::Vec2;
use urbanization_developments::yawed_plan_aabb_extent;

/// Building-skirt and path-grade support geometry.
#[derive(Debug, Clone)]
pub enum PadFootprint {
	/// Rounded rectangle in a yawed local frame (building plan).
	Rect(PadRect),
	/// Capsule / stadium for one graded path segment (hydro-reach analog).
	Reach(PadReach),
}

/// Capsule / stadium connecting two pad sites.
#[derive(Debug, Clone, Copy)]
pub struct PadReach {
	pub a: Vec2,
	pub b: Vec2,
	pub half_width: f32,
}

/// Rounded rectangle: `half_extents` in the local frame after [`Self::yaw`].
#[derive(Debug, Clone, Copy)]
pub struct PadRect {
	pub center: Vec2,
	pub half_extents: Vec2,
	/// Radians about \(+Y\), same sense as [`urbanization_developments::yaw_about_xz`].
	pub yaw: f32,
	pub round: f32,
}

impl PadFootprint {
	pub fn sdf(&self, p: Vec2) -> f32 {
		match self {
			Self::Rect(r) => r.sdf(p),
			Self::Reach(r) => r.sdf(p),
		}
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		match self {
			Self::Rect(r) => r.aabb(),
			Self::Reach(r) => r.aabb(),
		}
	}

	/// Unit travel \(z \in [0,1]\) along a reach; `None` for rectangles.
	pub fn reach_progress(&self, p: Vec2) -> Option<f32> {
		match self {
			Self::Reach(r) => Some(r.frame(p).0),
			Self::Rect(_) => None,
		}
	}
}

impl PadRect {
	/// World point → building-local frame (inverse of [`urbanization_developments::yaw_about_xz`]).
	///
	/// Bevy `Quat::from_rotation_y` maps local \((x,z)\) to
	/// \((x\cos\theta + z\sin\theta,\; -x\sin\theta + z\cos\theta)\).
	pub fn local(&self, p: Vec2) -> Vec2 {
		let (s, c) = self.yaw.sin_cos();
		let d = p - self.center;
		Vec2::new(c * d.x - s * d.y, s * d.x + c * d.y)
	}

	/// Rounded-box SDF in the yawed frame (IQ `sdRoundedBox`).
	pub fn sdf(&self, p: Vec2) -> f32 {
		let local = self.local(p);
		let round = self.round.max(0.0);
		let half = self.half_extents.max(Vec2::splat(round.max(1e-3)));
		let q = local.abs() - half + Vec2::splat(round);
		let outside = q.max(Vec2::ZERO).length() - round;
		let inside = q.x.max(q.y).min(0.0);
		outside + inside
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		let full = self.half_extents * 2.0;
		let occ = yawed_plan_aabb_extent(full.x, full.y, self.yaw);
		let half = occ * 0.5 + Vec2::splat(self.round.max(0.0));
		(self.center - half, self.center + half)
	}
}

impl PadReach {
	pub fn sdf(&self, p: Vec2) -> f32 {
		segment_distance(p, self.a, self.b) - self.half_width.max(1e-3)
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		let hw = self.half_width.max(1e-3);
		let mn = Vec2::new(self.a.x.min(self.b.x), self.a.y.min(self.b.y)) - Vec2::splat(hw);
		let mx = Vec2::new(self.a.x.max(self.b.x), self.a.y.max(self.b.y)) + Vec2::splat(hw);
		(mn, mx)
	}

	/// Unit travel \(z \in [0,1]\) and signed cross-track \(x\).
	pub fn frame(&self, p: Vec2) -> (f32, f32) {
		let ab = self.b - self.a;
		let len = ab.length();
		if len <= 1e-6 {
			return (0.0, p.distance(self.a));
		}
		let dir = ab / len;
		let rel = p - self.a;
		let z = (rel.dot(dir) / len).clamp(0.0, 1.0);
		let perp = Vec2::new(-dir.y, dir.x);
		(z, rel.dot(perp))
	}
}

fn segment_distance(p: Vec2, a: Vec2, b: Vec2) -> f32 {
	let ab = b - a;
	let len2 = ab.length_squared();
	if len2 <= 1e-12 {
		return p.distance(a);
	}
	let t = ((p - a).dot(ab) / len2).clamp(0.0, 1.0);
	(a + ab * t).distance(p)
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::f32::consts::FRAC_PI_2;

	#[test]
	fn rect_sdf_negative_inside() {
		let r = PadRect {
			center: Vec2::ZERO,
			half_extents: Vec2::new(10.0, 4.0),
			yaw: 0.0,
			round: 0.0,
		};
		assert!(r.sdf(Vec2::ZERO) < -1.0);
		assert!(r.sdf(Vec2::new(9.0, 0.0)) < 0.0);
		assert!(r.sdf(Vec2::new(11.0, 0.0)) > 0.0);
		assert!(r.sdf(Vec2::new(0.0, 5.0)) > 0.0);
	}

	#[test]
	fn yawed_rect_tracks_local_x() {
		let r = PadRect {
			center: Vec2::ZERO,
			half_extents: Vec2::new(10.0, 4.0),
			yaw: FRAC_PI_2,
			round: 0.0,
		};
		// Local +x is world −z after +π/2 yaw (`Quat::from_rotation_y`).
		assert!(r.sdf(Vec2::new(0.0, -9.0)) < 0.0);
		assert!(r.sdf(Vec2::new(0.0, -11.0)) > 0.0);
		assert!(r.sdf(Vec2::new(5.0, 0.0)) > 0.0);
	}

	#[test]
	fn reach_sdf_is_a_capsule() {
		let r = PadReach { a: Vec2::ZERO, b: Vec2::new(20.0, 0.0), half_width: 4.0 };
		assert!(r.sdf(Vec2::new(10.0, 0.0)) < -3.0);
		assert!(r.sdf(Vec2::new(10.0, 3.0)) < 0.0);
		assert!(r.sdf(Vec2::new(10.0, 5.0)) > 0.0);
		assert!((r.frame(Vec2::new(10.0, 2.0)).0 - 0.5).abs() < 1e-4);
	}
}
