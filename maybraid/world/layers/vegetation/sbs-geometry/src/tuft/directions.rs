//! Golden-angle direction caps for tuft element placement.

use bevy_math::Vec3;

use crate::jitter::unit_jitter;

/// Lane block reserved for [`CapDirections::length_scale`] so it never reuses a direction lane.
const LENGTH_LANE_BASE: u32 = 0x8000_0000;

/// Even azimuth spacing on a tuft hemisphere or droop cap.
pub(crate) struct CapDirections;

impl CapDirections {
	const GOLDEN_ANGLE: f32 = 2.399_963_229_728_653_32;

	/// Upward-biased directions on a spherical cap (shared anchor, mostly +Y).
	pub(crate) fn upward(count: u32, seed: i32, max_tilt_radians: f32) -> Vec<Vec3> {
		let n = count.max(1);
		let phase = unit_jitter(seed, 0) * std::f32::consts::TAU;
		(0..n)
			.map(|i| {
				let fi = i as f32;
				let azimuth = Self::GOLDEN_ANGLE.mul_add(fi, phase);
				let tilt = max_tilt_radians * (0.55 + 0.45 * unit_jitter(seed, 2 * i + 1));
				Vec3::new(tilt.sin() * azimuth.cos(), tilt.cos(), tilt.sin() * azimuth.sin())
					.normalize_or_zero()
			})
			.collect()
	}

	/// Per-element length multiplier in `[min, max]` (deterministic from seed).
	pub(crate) fn length_scale(index: u32, seed: i32, min: f32, max: f32) -> f32 {
		min + (max - min) * unit_jitter(seed, LENGTH_LANE_BASE | index)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Grove placement seeds land near `2^30`; per-element variation must survive
	/// (see `crate::jitter` for the f32 ulp collapse this guards against).
	#[test]
	fn per_element_variation_survives_large_seeds() {
		let seed = 1_073_127_521;
		let lengths: Vec<f32> =
			(0..8).map(|i| CapDirections::length_scale(i, seed, 0.8, 1.2)).collect();
		let min_l = lengths.iter().copied().fold(f32::INFINITY, f32::min);
		let max_l = lengths.iter().copied().fold(f32::NEG_INFINITY, f32::max);
		assert!(max_l - min_l > 1e-3, "lengths collapsed: {lengths:?}");
	}

	#[test]
	fn upward_cap_stays_above_horizon_within_tilt() {
		for d in CapDirections::upward(8, 42, 0.42) {
			assert!(d.y > 0.0, "upward cap dipped below horizon: {d:?}");
		}
	}
}
