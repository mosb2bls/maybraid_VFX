//! Shared panel-space frame for a world triangle \(A,B,C\).
//!
//! Origin at \(A\), \(+X\) along \(B-A\), \(+Y\) = unit normal \((B-A)\times(C-A)\),
//! \(+Z\) completes the right-handed basis. Panel coordinates are \((X, Z)\) as [`Vec2`].

use bevy_math::{EulerRot, Mat3, Quat, Vec2, Vec3};
use building_components::Placement;

/// Orthonormal frame + projected outer corners for a non-degenerate triangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelPlaneFrame {
	pub origin: Vec3,
	pub e0: Vec3,
	pub normal: Vec3,
	pub e1: Vec3,
	/// \(B\) in panel \(XZ\).
	pub b2: Vec2,
	/// \(C\) in panel \(XZ\).
	pub c2: Vec2,
}

/// Build the panel frame for world corners \(A,B,C\), or [`None`] if degenerate.
pub fn panel_plane_frame(a: Vec3, b: Vec3, c: Vec3) -> Option<PanelPlaneFrame> {
	let ab = b - a;
	let ac = c - a;
	let ab_len = ab.length();
	if ab_len < 1e-8 {
		return None;
	}
	let e0 = ab / ab_len;
	let n = ab.cross(ac);
	let n_len = n.length();
	if n_len < 1e-12 {
		return None;
	}
	let normal = n / n_len;
	let e1 = e0.cross(normal);
	let b2 = Vec2::new(ab_len, 0.0);
	let c2 = Vec2::new(ac.dot(e0), ac.dot(e1));
	if (b2.x * c2.y - b2.y * c2.x).abs() < 1e-12 {
		return None;
	}
	Some(PanelPlaneFrame { origin: a, e0, normal, e1, b2, c2 })
}

impl PanelPlaneFrame {
	/// Project world \(p\) into panel \(XZ\).
	pub fn project(&self, p: Vec3) -> Vec2 {
		let d = p - self.origin;
		Vec2::new(d.dot(self.e0), d.dot(self.e1))
	}

	/// Unproject panel \(XZ\) back to world on the plane.
	pub fn unproject(&self, p: Vec2) -> Vec3 {
		self.origin + self.e0 * p.x + self.e1 * p.y
	}

	/// Outer corners in panel space: \(A=\mathbf{0}\), \(B\), \(C\).
	pub fn outer_2d(&self) -> [Vec2; 3] {
		[Vec2::ZERO, self.b2, self.c2]
	}

	/// Parent [`Placement`] encoding this frame (YXZ euler).
	pub fn placement(&self) -> Placement {
		let rotation = Quat::from_mat3(&Mat3::from_cols(self.e0, self.normal, self.e1));
		let (yaw, pitch, roll) = rotation.to_euler(EulerRot::YXZ);
		Placement { translation: self.origin, yaw, pitch, roll, scale: Vec3::ONE }
	}
}
