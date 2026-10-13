//! Pocket-cell guillotine partition — [RFC-127 §3.1.2](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-127-marazion-watersheds#312-pocket-cells).

use crate::authored::noise::n01;
use bevy_math::Vec2;
use procedural_common::Bounds2;

const SPLIT_SALT: u32 = 0x5117_7001;
const CUT_SALT: u32 = 0x0C07_AA11;

#[derive(Debug, Clone, Copy)]
pub struct PocketGuillotineParams {
	pub max_depth: u8,
	/// Minimum leaf span (world units) on either axis.
	/// Band defaults: low-pass ≈200, high-pass ≈800.
	pub min_span: f32,
	/// Cut ratio window `[lo, hi]` along the chosen axis.
	pub cut_lo: f32,
	pub cut_hi: f32,
	pub seed: u32,
}

impl Default for PocketGuillotineParams {
	fn default() -> Self {
		Self { max_depth: 3, min_span: 400.0, cut_lo: 0.25, cut_hi: 0.75, seed: 0 }
	}
}

/// Recursive axis-aligned guillotine leaves tiling `bounds` without gaps.
pub fn guillotine_partition(bounds: Bounds2, params: &PocketGuillotineParams) -> Vec<Bounds2> {
	let mut out = Vec::new();
	partition_rec(bounds, 0, params, &mut out);
	out
}

fn partition_rec(
	rect: Bounds2,
	depth: u8,
	params: &PocketGuillotineParams,
	out: &mut Vec<Bounds2>,
) {
	let w = (rect.max.x - rect.min.x).abs();
	let h = (rect.max.y - rect.min.y).abs();
	if depth >= params.max_depth || w < params.min_span * 2.0 || h < params.min_span * 2.0 {
		out.push(rect);
		return;
	}

	let ll = Vec2::new(rect.min.x, rect.min.y);
	let ix = ll.x.to_bits() as i32;
	let iz = ll.y.to_bits() as i32;
	let vertical = n01(params.seed, SPLIT_SALT.wrapping_add(depth as u32), ix, iz) < 0.5;
	let t = params.cut_lo
		+ (params.cut_hi - params.cut_lo)
			* n01(params.seed, CUT_SALT.wrapping_add(depth as u32 * 17), ix, iz);

	let (a, b) = if vertical {
		let x = rect.min.x + w * t;
		if (x - rect.min.x) < params.min_span || (rect.max.x - x) < params.min_span {
			out.push(rect);
			return;
		}
		(
			Bounds2::from_xz(rect.min.x, rect.min.y, x, rect.max.y),
			Bounds2::from_xz(x, rect.min.y, rect.max.x, rect.max.y),
		)
	} else {
		let z = rect.min.y + h * t;
		if (z - rect.min.y) < params.min_span || (rect.max.y - z) < params.min_span {
			out.push(rect);
			return;
		}
		(
			Bounds2::from_xz(rect.min.x, rect.min.y, rect.max.x, z),
			Bounds2::from_xz(rect.min.x, z, rect.max.x, rect.max.y),
		)
	};

	partition_rec(a, depth + 1, params, out);
	partition_rec(b, depth + 1, params, out);
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn leaves_tile_without_gaps() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 1500.0, 1500.0);
		let params = PocketGuillotineParams::default();
		let leaves = guillotine_partition(bounds, &params);
		assert!(!leaves.is_empty());
		let parent_area = 1500.0 * 1500.0;
		let leaf_area: f32 = leaves.iter().map(|b| (b.max.x - b.min.x) * (b.max.y - b.min.y)).sum();
		assert!((leaf_area - parent_area).abs() < 1.0);
		for leaf in &leaves {
			let w = (leaf.max.x - leaf.min.x).abs();
			let h = (leaf.max.y - leaf.min.y).abs();
			assert!(w + 1e-3 >= params.min_span * 0.99);
			assert!(h + 1e-3 >= params.min_span * 0.99);
		}
		Ok(())
	}
}
