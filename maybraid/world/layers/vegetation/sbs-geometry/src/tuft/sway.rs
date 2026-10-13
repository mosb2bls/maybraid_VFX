//! Per-strand sway sampling for ribbon / prism tuft builders.

use procedural_common::NoiseConfig;

/// Lateral offset in the strand bend plane (width axis, forward axis).
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct StrandSway {
	pub right: f32,
	pub forward: f32,
}

/// Sample deterministic sway at normalized height `t` ∈ [0, 1] along a strand.
pub(crate) fn strand_sway_at(
	noise: &NoiseConfig,
	seed: i32,
	t: f32,
	noise_frequency: f32,
	noise_amplitude: f32,
	max_sway: f32,
) -> StrandSway {
	let coord = t * noise_frequency;
	let nx = seed as f32 + 0.13;
	let nz = seed as f32 + 29.7;
	let right = (noise.raw_3d(nx, coord, nz) * noise_amplitude).clamp(-max_sway, max_sway);
	let forward =
		(noise.raw_3d(nx + 5.1, coord, nz + 2.3) * noise_amplitude).clamp(-max_sway, max_sway);
	StrandSway { right, forward }
}
