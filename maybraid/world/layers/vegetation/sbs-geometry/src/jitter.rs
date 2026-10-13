//! Precision-safe deterministic jitter from integer seeds.
//!
//! Direction caps previously derived per-element variation by casting `seed + i` to `f32`
//! and feeding it to trig. For seeds near `2^30` (e.g. grove placement seeds XOR-mixed from
//! position float bits) the f32 ulp is 64, so small index offsets round onto the *same*
//! float and all per-element and per-ring variation collapses, flattening crowns to one
//! biased fan. Mixing in integer space keeps full variation for seeds of any magnitude.

/// Deterministic sample in `[0, 1)` from a seed and a decorrelation lane.
///
/// Lanes separate independent random streams drawn from the same seed (e.g. tilt vs
/// spread vs azimuth of the same element).
pub(crate) fn unit_jitter(seed: i32, lane: u32) -> f32 {
	// lowbias32 (https://github.com/skeeto/hash-prospector) over seed and lane.
	let mut h = (seed as u32) ^ lane.wrapping_mul(0x9E37_79B9);
	h ^= h >> 16;
	h = h.wrapping_mul(0x7FEB_352D);
	h ^= h >> 15;
	h = h.wrapping_mul(0x846C_A68B);
	h ^= h >> 16;
	(h >> 8) as f32 / (1 << 24) as f32
}

/// Deterministic sample in `[-1, 1)` from a seed and a decorrelation lane.
pub(crate) fn signed_jitter(seed: i32, lane: u32) -> f32 {
	unit_jitter(seed, lane) * 2.0 - 1.0
}

#[cfg(test)]
mod tests {
	use super::*;

	/// Regression for the f32 ulp collapse: near `2^30`, increments smaller than 64
	/// vanish under an `as f32` cast, but must stay distinct under integer mixing.
	#[test]
	fn near_ulp_seeds_stay_distinct() {
		let seed = 1_073_127_521;
		assert_eq!((seed + 18) as f32, seed as f32, "expected f32 collapse at this magnitude");
		let a = unit_jitter(seed, 1);
		let b = unit_jitter(seed + 18, 1);
		assert!((a - b).abs() > 1e-6, "jitter collapsed: {a} vs {b}");
	}

	#[test]
	fn lanes_decorrelate() {
		let a = unit_jitter(7, 1);
		let b = unit_jitter(7, 2);
		assert!((a - b).abs() > 1e-6, "lanes collided: {a} vs {b}");
	}

	#[test]
	fn samples_stay_in_unit_range() {
		for lane in 0..256 {
			let u = unit_jitter(i32::MIN, lane);
			assert!((0.0..1.0).contains(&u), "out of range: {u}");
		}
	}
}
