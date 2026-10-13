//! Capsule / stadium support for one channel reach.

use bevy_math::Vec2;

/// Capsule / stadium for one reach segment.
#[derive(Debug, Clone)]
pub struct ReachSegment {
	pub a: Vec2,
	pub b: Vec2,
	pub half_width: f32,
}

impl ReachSegment {
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
		let x = rel.dot(perp);
		(z, x)
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
