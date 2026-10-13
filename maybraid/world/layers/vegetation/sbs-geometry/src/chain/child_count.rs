//! Noisy child counts from half-open [`std::ops::Range`]`<usize>`.

use std::ops::Range;

use procedural_common::NoiseConfig;

use crate::BallStickNode;

pub fn sample_usize(
	noise: &NoiseConfig,
	range: Range<usize>,
	parent: &BallStickNode,
	segment_index: usize,
) -> usize {
	let span = range.end.saturating_sub(range.start);
	if span == 0 {
		return range.start.max(1);
	}
	let t = noise.sample_unit_3d(
		parent.position.x + segment_index as f32 * 0.19,
		parent.position.y,
		parent.position.z,
	);
	let idx = (t * span as f32).floor() as usize;
	range.start + idx.min(span - 1)
}
