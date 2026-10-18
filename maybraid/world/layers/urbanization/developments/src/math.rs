//! Small scalar helpers shared by development layouts.

/// Linear interpolation with `t` clamped to `[0, 1]`.
pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
	a + (b - a) * t.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn lerp_clamps_t() {
		assert_eq!(lerp(0.0, 10.0, -1.0), 0.0);
		assert_eq!(lerp(0.0, 10.0, 2.0), 10.0);
	}

	#[test]
	fn lerp_endpoints() {
		assert_eq!(lerp(2.0, 8.0, 0.0), 2.0);
		assert_eq!(lerp(2.0, 8.0, 1.0), 8.0);
		assert_eq!(lerp(2.0, 8.0, 0.5), 5.0);
	}
}
