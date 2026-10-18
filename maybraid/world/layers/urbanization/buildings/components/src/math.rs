//! Small planar (XZ) helpers shared by building layout code.

use bevy_math::Vec2;

const MIN_LENGTH: f32 = 1e-5;

/// Unit direction in the XZ plane, or `None` if `v` is too short.
pub fn normalize_xz(v: Vec2) -> Option<Vec2> {
	let len = v.length();
	if len < MIN_LENGTH {
		None
	} else {
		Some(v / len)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn normalize_xz_rejects_near_zero() {
		assert!(normalize_xz(Vec2::ZERO).is_none());
		assert!(normalize_xz(Vec2::new(1e-6, 0.0)).is_none());
	}

	#[test]
	fn normalize_xz_returns_unit_direction() {
		let v = Vec2::new(3.0, 4.0);
		let n = normalize_xz(v).expect("non-zero vector");
		assert!((n.length() - 1.0).abs() < 1e-6);
		assert!((n.x - 0.6).abs() < 1e-6);
		assert!((n.y - 0.8).abs() < 1e-6);
	}
}
