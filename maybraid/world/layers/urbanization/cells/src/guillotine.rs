//! Local adaptive guillotine partition for urbanization leaves.
//!
//! Does not depend on marazion; noise is deterministic from leaf-corner bits + seed
//! (same spirit as pocket-cell partitions).

use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

const SPLIT_SALT: u32 = 0x5117_7001;
const CUT_SALT: u32 = 0x0C07_AA11;

/// Adaptive guillotine knobs for a 1600 m urbanization cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UrbanizationGuillotineParams {
	pub max_depth: u8,
	/// Minimum leaf span (world units) on either axis.
	pub min_span: f32,
	/// Prefer splitting while a side exceeds this span.
	pub max_span: f32,
	/// Cut ratio window `[lo, hi]` along the chosen axis.
	pub cut_lo: f32,
	pub cut_hi: f32,
	pub seed: u32,
}

impl Default for UrbanizationGuillotineParams {
	fn default() -> Self {
		Self { max_depth: 5, min_span: 200.0, max_span: 600.0, cut_lo: 0.35, cut_hi: 0.65, seed: 0 }
	}
}

impl UrbanizationGuillotineParams {
	pub fn with_seed(mut self, seed: u32) -> Self {
		self.seed = seed;
		self
	}
}

fn n01(seed: u32, salt: u32, ix: i32, iz: i32) -> f32 {
	SeededHash::new(seed.wrapping_add(salt)).unit_i32(ix, iz)
}

/// Recursive axis-aligned guillotine leaves tiling `bounds` without gaps.
///
/// Splits while `depth < max_depth`, at least one side exceeds `max_span`, and
/// children can still meet `min_span`. Prefers the longer axis when a side is
/// oversized.
pub fn guillotine_partition(
	bounds: Bounds2,
	params: &UrbanizationGuillotineParams,
) -> Vec<Bounds2> {
	let mut out = Vec::new();
	partition_rec(bounds, 0, params, &mut out);
	out
}

fn partition_rec(
	rect: Bounds2,
	depth: u8,
	params: &UrbanizationGuillotineParams,
	out: &mut Vec<Bounds2>,
) {
	let w = (rect.max.x - rect.min.x).abs();
	let h = (rect.max.y - rect.min.y).abs();
	let oversized = w > params.max_span || h > params.max_span;
	if depth >= params.max_depth || !oversized {
		out.push(rect);
		return;
	}

	let ll = Vec2::new(rect.min.x, rect.min.y);
	let ix = ll.x.to_bits() as i32;
	let iz = ll.y.to_bits() as i32;
	let axis_noise = n01(params.seed, SPLIT_SALT.wrapping_add(depth as u32), ix, iz);
	let Some(split_x) = choose_split_axis(w, h, params, axis_noise) else {
		out.push(rect);
		return;
	};

	let t = params.cut_lo
		+ (params.cut_hi - params.cut_lo)
			* n01(params.seed, CUT_SALT.wrapping_add(depth as u32 * 17), ix, iz);

	let (a, b) = if split_x {
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

/// `Some(true)` = split on X (vertical cut); `Some(false)` = split on Z.
fn choose_split_axis(
	w: f32,
	h: f32,
	params: &UrbanizationGuillotineParams,
	axis_noise: f32,
) -> Option<bool> {
	let can_w = w >= params.min_span * 2.0;
	let can_h = h >= params.min_span * 2.0;
	if !can_w && !can_h {
		return None;
	}
	let w_over = w > params.max_span;
	let h_over = h > params.max_span;
	if w_over && h_over {
		// Prefer the longer axis; noise breaks near-ties.
		let prefer_w = if (w - h).abs() < 1e-3 { axis_noise < 0.5 } else { w > h };
		if prefer_w {
			if can_w {
				Some(true)
			} else if can_h {
				Some(false)
			} else {
				None
			}
		} else if can_h {
			Some(false)
		} else if can_w {
			Some(true)
		} else {
			None
		}
	} else if w_over {
		can_w.then_some(true)
	} else if h_over {
		can_h.then_some(false)
	} else {
		None
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::Result;

	#[test]
	fn leaves_tile_1600_cell_without_gaps() -> Result<()> {
		let bounds = Bounds2::from_xz(-800.0, -800.0, 800.0, 800.0);
		let params = UrbanizationGuillotineParams::default();
		let leaves = guillotine_partition(bounds, &params);
		assert!(!leaves.is_empty());
		let parent_area = 1600.0 * 1600.0;
		let leaf_area: f32 = leaves.iter().map(|b| (b.max.x - b.min.x) * (b.max.y - b.min.y)).sum();
		assert!((leaf_area - parent_area).abs() < 1.0);
		Ok(())
	}

	#[test]
	fn leaves_generally_lie_in_min_max_span() -> Result<()> {
		let bounds = Bounds2::from_xz(-800.0, -800.0, 800.0, 800.0);
		let params = UrbanizationGuillotineParams::default().with_seed(7);
		let leaves = guillotine_partition(bounds, &params);
		assert!(!leaves.is_empty());
		let mut within = 0usize;
		for leaf in &leaves {
			let w = (leaf.max.x - leaf.min.x).abs();
			let h = (leaf.max.y - leaf.min.y).abs();
			assert!(w + 1e-3 >= params.min_span * 0.99);
			assert!(h + 1e-3 >= params.min_span * 0.99);
			if w <= params.max_span + 1e-2 && h <= params.max_span + 1e-2 {
				within += 1;
			}
		}
		// Adaptive stop can leave a rare oversize leaf at max depth; most should fit.
		assert!(within * 2 >= leaves.len(), "within={within} total={}", leaves.len());
		Ok(())
	}

	#[test]
	fn partition_is_deterministic() -> Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 1600.0, 1600.0);
		let params = UrbanizationGuillotineParams::default().with_seed(99);
		let a = guillotine_partition(bounds, &params);
		let b = guillotine_partition(bounds, &params);
		assert_eq!(a.len(), b.len());
		for (left, right) in a.iter().zip(b.iter()) {
			assert!((left.min.x - right.min.x).abs() < 1e-5);
			assert!((left.min.y - right.min.y).abs() < 1e-5);
			assert!((left.max.x - right.max.x).abs() < 1e-5);
			assert!((left.max.y - right.max.y).abs() < 1e-5);
		}
		Ok(())
	}
}
