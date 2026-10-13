//! Rotated elliptical disc (lake body).

use bevy_math::Vec2;

/// Rotated elliptical disc.
#[derive(Debug, Clone)]
pub struct Ellipse {
	pub center: Vec2,
	pub radii: Vec2,
	pub rotation: f32,
}

impl Ellipse {
	pub fn sdf(&self, p: Vec2) -> f32 {
		// Approximate analytic SDF (IQ style).
		let (s, c) = self.rotation.sin_cos();
		let d = p - self.center;
		let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
		let ab = self.radii.max(Vec2::splat(1e-3));
		let k0 = (local / ab).length();
		if k0 < 1e-8 {
			return -ab.min_element();
		}
		let k1 = (local / (ab * ab)).length();
		k0 * (k0 - 1.0) / k1.max(1e-6)
	}

	pub fn aabb(&self) -> (Vec2, Vec2) {
		// Conservative AABB of rotated ellipse.
		let (s, c) = self.rotation.sin_cos();
		let rx = self.radii.x.max(1e-3);
		let rz = self.radii.y.max(1e-3);
		let ex = (c * rx).abs() + (s * rz).abs();
		let ez = (s * rx).abs() + (c * rz).abs();
		(self.center - Vec2::new(ex, ez), self.center + Vec2::new(ex, ez))
	}

	/// Ellipse-normalized radial coordinate (0 at center, 1 on the rim).
	pub fn radial_norm(&self, p: Vec2) -> f32 {
		let (s, c) = self.rotation.sin_cos();
		let d = p - self.center;
		let local = Vec2::new(c * d.x + s * d.y, -s * d.x + c * d.y);
		let rx = self.radii.x.max(1e-3);
		let rz = self.radii.y.max(1e-3);
		(local / Vec2::new(rx, rz)).length()
	}
}
