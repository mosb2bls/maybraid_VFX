//! Stable per-node mixing keys for canopy component selection.

use bevy_math::Vec3;

/// Hash from graph index and node position (shared by canopy, tuft, and growth picks).
pub fn node_mix_seed(node_idx: usize, position: Vec3) -> u32 {
	(node_idx as u32)
		.wrapping_mul(0x9E37_79B9)
		.wrapping_add(position.x.to_bits())
		.wrapping_add(position.y.to_bits().rotate_left(3))
		.wrapping_add(position.z.to_bits().rotate_left(7))
}

/// Unit interval gate: `node_mix_seed / u32::MAX < fraction`.
pub fn mix_seed_below_fraction(node_idx: usize, position: Vec3, fraction: f32) -> bool {
	let fraction = fraction.clamp(0.0, 1.0);
	if fraction <= 0.0 {
		return false;
	}
	if fraction >= 1.0 {
		return true;
	}
	(node_mix_seed(node_idx, position) as f32) / (u32::MAX as f32) < fraction
}
