//! Hashed instance seeds. Do not cast large `u64` values directly to `f32`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_SEED: AtomicU64 = AtomicU64::new(1);

pub const LAYER_FLASH: u32 = 1;
pub const LAYER_FIRE: u32 = 2;
pub const LAYER_SMOKE: u32 = 3;
pub const LAYER_SPARKS: u32 = 4;

pub fn mix64(mut x: u64) -> u64 {
	x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
	x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
	x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
	x ^ (x >> 31)
}

pub fn stream(seed: u64, layer: u32, index: u32) -> u64 {
	mix64(seed ^ (u64::from(layer) << 32) ^ u64::from(index).wrapping_mul(0x9e37_79b9))
}

pub fn unit(seed: u64) -> f32 {
	let bits = mix64(seed) >> 40;
	(bits as f32) / ((1u32 << 24) as f32)
}

pub fn signed(seed: u64) -> f32 {
	unit(seed) * 2.0 - 1.0
}

pub fn unit_salted(seed: u64, salt: u32) -> f32 {
	unit(mix64(seed ^ u64::from(salt).wrapping_mul(0x45d9_f3bb)))
}

pub fn signed_salted(seed: u64, salt: u32) -> f32 {
	unit_salted(seed, salt) * 2.0 - 1.0
}

/// New seed when the caller omitted one.
pub fn generate() -> u64 {
	let tick = NEXT_SEED.fetch_add(1, Ordering::Relaxed);
	let nanos = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_nanos() as u64)
		.unwrap_or(0);
	mix64(tick.wrapping_mul(0x9e37_79b9_7f4a_7c15) ^ nanos)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn large_seeds_stay_distinct() {
		let a = unit(1u64 << 40);
		let b = unit((1u64 << 40) + 1);
		assert!((a - b).abs() > 1e-6);
	}

	#[test]
	fn streams_differ_by_layer_and_index() {
		assert_ne!(stream(7, LAYER_FIRE, 0), stream(7, LAYER_SMOKE, 0));
		assert_ne!(stream(7, LAYER_FIRE, 0), stream(7, LAYER_FIRE, 1));
	}
}
