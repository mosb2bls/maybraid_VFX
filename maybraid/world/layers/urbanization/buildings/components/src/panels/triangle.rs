//! Shared triangle geometry helpers (normals, dihedral kink).

use bevy_math::Vec3;

/// Unit normal of triangle \(ABC\), or [`None`] if degenerate.
pub fn triangle_normal(a: Vec3, b: Vec3, c: Vec3) -> Option<Vec3> {
	let n = (b - a).cross(c - a);
	let len = n.length();
	if len < 1e-12 {
		None
	} else {
		Some(n / len)
	}
}

/// Dihedral kink (radians) between two unit normals.
///
/// \(0\) when coplanar with matching orientation; grows toward \(\pi\) as the fold opens.
pub fn dihedral_kink(n0: Vec3, n1: Vec3) -> f32 {
	n0.dot(n1).clamp(-1.0, 1.0).acos()
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn right_angle_fold_is_half_pi() {
		let n0 = triangle_normal(Vec3::ZERO, Vec3::X, Vec3::Z).unwrap();
		let n1 = triangle_normal(Vec3::ZERO, Vec3::Z, Vec3::Y).unwrap();
		let k = dihedral_kink(n0, n1);
		assert!((k - std::f32::consts::FRAC_PI_2).abs() < 1e-3, "got {k}");
	}
}
