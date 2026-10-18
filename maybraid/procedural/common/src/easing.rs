//! Scalar easing helpers for procedural falloff and blending.

/// Hermite smoothstep on the unit interval: clamps `t` to `[0, 1]`, then `t²(3 − 2t)`.
pub struct Smoothstep01;

impl Smoothstep01 {
	#[inline]
	pub fn eval(t: f32) -> f32 {
		let t = t.clamp(0.0, 1.0);
		t * t * (3.0 - 2.0 * t)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn endpoints() {
		assert!(Smoothstep01::eval(0.0).abs() < 1e-5);
		assert!((Smoothstep01::eval(1.0) - 1.0).abs() < 1e-5);
	}

	#[test]
	fn clamps_out_of_range() {
		assert!(Smoothstep01::eval(-0.5).abs() < 1e-5);
		assert!((Smoothstep01::eval(1.5) - 1.0).abs() < 1e-5);
	}

	#[test]
	fn midpoint() {
		assert!((Smoothstep01::eval(0.5) - 0.5).abs() < 1e-5);
	}
}
