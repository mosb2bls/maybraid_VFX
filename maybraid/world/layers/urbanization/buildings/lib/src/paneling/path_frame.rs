//! Orthonormal frames along a polyline centerline.
//!
//! At each station, inbound/outbound segments define an average tangent; the
//! unbanked basis keeps `up` increasing world Y in the ⊥ plane; `roll` banks
//! about that tangent.

use bevy_math::{Quat, Vec3};

/// Local orthonormal axes in the average perpendicular plane after roll.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeFrame {
	/// Average tangent (normal to the cross-section plane).
	pub tangent: Vec3,
	pub right: Vec3,
	pub up: Vec3,
}

/// Average of inbound/outbound unit tangents at `positions[index]`.
pub fn average_path_tangent(positions: &[Vec3], index: usize) -> Vec3 {
	let p = positions[index];
	let inbound =
		if index > 0 { Some((p - positions[index - 1]).normalize_or_zero()) } else { None };
	let outbound = if index + 1 < positions.len() {
		Some((positions[index + 1] - p).normalize_or_zero())
	} else {
		None
	};
	match (inbound, outbound) {
		(Some(a), Some(b)) => {
			let sum = a + b;
			let n = sum.normalize_or_zero();
			if n.length_squared() > 0.0 {
				n
			} else if a.length_squared() > 0.0 {
				a
			} else {
				b
			}
		}
		(Some(a), None) => {
			if a.length_squared() > 0.0 {
				a
			} else {
				Vec3::Z
			}
		}
		(None, Some(b)) => {
			if b.length_squared() > 0.0 {
				b
			} else {
				Vec3::Z
			}
		}
		(None, None) => Vec3::Z,
	}
}

/// Unbanked basis in the plane ⊥ `tangent`: `up` increases world Y in that plane.
pub fn zero_roll_basis(tangent: Vec3) -> (Vec3, Vec3) {
	let t = tangent.normalize_or_zero();
	let t = if t.length_squared() > 0.0 { t } else { Vec3::Z };
	let mut up = Vec3::Y - t * t.dot(Vec3::Y);
	if up.length_squared() < 1e-10 {
		up = Vec3::X - t * t.dot(Vec3::X);
	}
	if up.length_squared() < 1e-10 {
		up = Vec3::Z - t * t.dot(Vec3::Z);
	}
	let up = up.normalize_or_zero();
	let right = up.cross(t).normalize_or_zero();
	// Re-orthogonalize up in case of numerical drift.
	let up = t.cross(right).normalize_or_zero();
	(right, up)
}

/// Average tangent and zero-roll basis, then apply `roll` about the tangent.
pub fn path_frame(positions: &[Vec3], index: usize, roll: f32) -> TubeFrame {
	let t = average_path_tangent(positions, index);
	let (right0, up0) = zero_roll_basis(t);
	if roll.abs() < 1e-8 {
		return TubeFrame { tangent: t, right: right0, up: up0 };
	}
	let q = Quat::from_axis_angle(t, roll);
	TubeFrame { tangent: t, right: q * right0, up: q * up0 }
}
