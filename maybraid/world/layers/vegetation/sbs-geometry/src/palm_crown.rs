//! Shared palm-crown ring math ([RFC §3.1.6.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-183-chico-vegetation/03-01-stalk-and-ball-stick-trees/06-well-known-component-constructions/01-palm-crown/README.md)).

/// Normalized ring index `u = ring / (ring_count - 1)` in `[0, 1]`.
pub fn ring_mix_u(ring: u32, ring_count: u32) -> f32 {
	let n = ring_count.max(1);
	if n <= 1 {
		return 0.0;
	}
	ring as f32 / (n - 1) as f32
}

/// RFC vertical bias mix: `mix(low, high, u)` for ring index `ring`.
pub fn vertical_bias_mix(ring: u32, ring_count: u32, low: f32, high: f32) -> f32 {
	let u = ring_mix_u(ring, ring_count);
	low + (high - low) * u
}

/// World ring spacing from height scale and fraction of `H`.
pub fn ring_spacing_world(height: f32, spacing_fraction: f32) -> f32 {
	height.max(1e-6) * spacing_fraction
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn vertical_bias_mix_endpoints() {
		assert!((vertical_bias_mix(0, 8, -0.20, 0.35) - (-0.20)).abs() < 1e-5);
		assert!((vertical_bias_mix(7, 8, -0.20, 0.35) - 0.35).abs() < 1e-5);
	}

	#[test]
	fn ring_mix_u_single_ring_is_zero() {
		assert!((ring_mix_u(0, 1) - 0.0).abs() < 1e-5);
	}
}
