//! Grove and bump-out stream radii (metres).

/// Forest selection generate ring around the camera (metres).
pub const GROVE_GENERATE_RADIUS_M: f32 = 3_000.0;

/// Grove geometry present ring around the camera (metres).
pub const GROVE_PRESENT_RADIUS_M: f32 = 1_000.0;

#[cfg(test)]
mod tests {
	use super::*;
	use crate::bump_out::BUMP_OUT_OUTER_RADIUS_M;

	#[test]
	fn bump_out_radii_keep_the_grove_fill_hole() {
		assert!((crate::BUMP_OUT_INNER_RADIUS_M - GROVE_PRESENT_RADIUS_M).abs() < 1e-3);
		assert!((BUMP_OUT_OUTER_RADIUS_M - 5_000.0).abs() < 1e-3);
	}
}
