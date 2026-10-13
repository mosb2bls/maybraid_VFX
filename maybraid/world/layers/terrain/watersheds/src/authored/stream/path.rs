//! Stream path placement and graded water levels.

use crate::authored::noise::n01_at;
use bevy_math::Vec2;

pub(crate) const DEGENERATE_VERTEX_EPS: f32 = 1e-3;

pub(crate) const ENDPOINT_A_SALT: u32 = 0x57EA_E001;
pub(crate) const ENDPOINT_B_SALT: u32 = 0x57EA_E002;

pub(crate) fn sample_endpoint(seed: u32, salt: u32, lo: Vec2, hi: Vec2) -> Vec2 {
	let ux = n01_at(seed, salt, lo);
	let uz = n01_at(seed, salt.wrapping_add(1), lo);
	Vec2::new(lo.x + (hi.x - lo.x) * ux, lo.y + (hi.y - lo.y) * uz)
}

/// Sample per-node water elevations along a path (pre-watershed heights − sink).
///
/// Local segment pitches follow the samples; uphill chords are clamped so each
/// node is non-increasing vs its upstream neighbor. If the whole reach is nearly
/// flat, the toe is pulled down by `min_drop`.
pub(crate) fn node_water_levels(
	path: &[Vec2],
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
	sink: f32,
	min_drop: f32,
) -> Vec<f32> {
	let sink = sink.max(0.0);
	let min_drop = min_drop.max(0.0);
	let mut levels: Vec<f32> = path
		.iter()
		.map(|p| height_at.map(|f| f(p.x, p.y)).unwrap_or(0.0) - sink)
		.collect();
	for i in 1..levels.len() {
		levels[i] = levels[i].min(levels[i - 1]);
	}
	if levels.len() >= 2 {
		let head = levels[0];
		let last = levels.len() - 1;
		if head - levels[last] < min_drop {
			levels[last] = head - min_drop;
			for i in (1..last).rev() {
				levels[i] = levels[i].min(levels[i - 1]).max(levels[last]);
			}
		}
	}
	levels
}

/// Drop zero-length hysteresis vertices so node-pitch blend cannot explode grades.
pub(crate) fn collapse_degenerate_vertices(path: &mut Vec<Vec2>, levels: &mut Vec<f32>, eps: f32) {
	let n = path.len().min(levels.len());
	if n < 2 {
		path.truncate(n);
		levels.truncate(n);
		return;
	}
	let eps = eps.max(0.0);
	let mut out_p = Vec::with_capacity(n);
	let mut out_l = Vec::with_capacity(n);
	out_p.push(path[0]);
	out_l.push(levels[0]);
	for i in 1..n {
		if path[i].distance(*out_p.last().expect("non-empty")) <= eps {
			// Keep the lower water (downstream-friendly) on the shared vertex.
			let last = out_l.len() - 1;
			out_l[last] = out_l[last].min(levels[i]);
			continue;
		}
		out_p.push(path[i]);
		out_l.push(levels[i]);
	}
	*path = out_p;
	*levels = out_l;
}
