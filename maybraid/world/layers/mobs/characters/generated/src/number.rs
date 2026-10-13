//! Stable scalar lanes shared by generated character facets.

/// Construct one authored facet from the character's scalar identity.
pub trait FromMobNumber {
	fn from_num(num: f32) -> Self;
}

pub(crate) fn lane(num: f32, salt: u64) -> u64 {
	let mut value = u64::from(num.to_bits()) ^ salt;
	value ^= value >> 30;
	value = value.wrapping_mul(0xBF58_476D_1CE4_E5B9);
	value ^= value >> 27;
	value = value.wrapping_mul(0x94D0_49BB_1331_11EB);
	value ^ (value >> 31)
}

pub(crate) fn index(num: f32, salt: u64, len: usize) -> usize {
	if len <= 1 {
		return 0;
	}
	(lane(num, salt) as usize) % len
}

pub(crate) fn seed(num: f32, salt: u64) -> u64 {
	lane(num, salt).max(1)
}
