//! FNV-1a 64-bit digests for stable cache and catalog keys.

/// Standard FNV-1a over 64-bit (`offset_basis` / `prime` from the FNV spec).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Fnv1a64;

impl Fnv1a64 {
	const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
	const PRIME: u64 = 0x0100_0000_01b3;

	/// Digest `bytes` with FNV-1a (xor then multiply per byte).
	pub fn digest(bytes: &[u8]) -> u64 {
		let mut hash = Self::OFFSET;
		for byte in bytes {
			hash ^= u64::from(*byte);
			hash = hash.wrapping_mul(Self::PRIME);
		}
		hash
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn empty_input_is_offset_basis() {
		assert_eq!(Fnv1a64::digest(b""), Fnv1a64::OFFSET);
	}

	#[test]
	fn hello_matches_reference_digest() {
		assert_eq!(Fnv1a64::digest(b"hello"), 0xa430_d846_80aa_bd0b);
	}
}
