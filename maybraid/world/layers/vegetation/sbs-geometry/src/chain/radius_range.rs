//! Noisy radii from half-open [`std::ops::Range`]`<f32>`.

use std::ops::Range;

use procedural_common::NoiseConfig;

use crate::BallStickNode;

pub fn sample_f32(
	noise: &NoiseConfig,
	range: Range<f32>,
	parent: &BallStickNode,
	segment_index: usize,
	child_index: u32,
) -> f32 {
	let t = noise.sample_unit_3d(
		parent.position.x + child_index as f32 * 0.17,
		parent.position.y + segment_index as f32 * 0.23,
		parent.position.z,
	);
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	lo + t * (hi - lo)
}
