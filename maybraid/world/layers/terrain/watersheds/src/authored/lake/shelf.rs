//! Lake centroid planning and shelf-anchor survey.

use crate::authored::lake::budget::{shelf_survey_radius, LakeBandBudget};
use crate::authored::lake::LakeParams;
use crate::authored::noise::{n01_at, n01_freq, n11_at};
use bevy_math::Vec2;
use procedural_common::Bounds2;

const WATER_SCALE_SALT: u32 = 0x1A7E_512E;
const RIM_WIDTH_SALT: u32 = 0x1A7E_71B7;
const ASPECT_SALT: u32 = 0x1A7E_A5EC;
const ROTATION_SALT: u32 = 0x1A7E_207A;
const SHELF_AMP_SALT: u32 = 0x1A7E_50F1;

pub(crate) fn water_scale_u01(seed: u32, leaf_min: Vec2, params: LakeParams) -> f32 {
	n01_freq(seed, WATER_SCALE_SALT, leaf_min, params.water_scale_freq)
}

pub(crate) fn rim_width_u01(seed: u32, leaf_min: Vec2, params: LakeParams) -> f32 {
	n01_freq(seed, RIM_WIDTH_SALT, leaf_min, params.rim_width_freq)
}

pub(crate) fn aspect_u01(seed: u32, leaf_min: Vec2) -> f32 {
	n01_at(seed, ASPECT_SALT, leaf_min)
}

pub(crate) fn rotation_u11(seed: u32, leaf_min: Vec2) -> f32 {
	n11_at(seed, ROTATION_SALT, leaf_min)
}

/// Median of `samples` (in-place select). Empty → `0.0`.
fn median_f32(samples: &mut [f32]) -> f32 {
	let n = samples.len();
	if n == 0 {
		return 0.0;
	}
	let mid = n / 2;
	let (_, med, _) = samples.select_nth_unstable_by(mid, |a, b| a.total_cmp(b));
	*med
}

/// Sample pre-watershed height around `center` to set the shelf / rim base.
///
/// With `count ≤ 1` (or no sampler), returns the centroid height. Otherwise
/// takes the **median** of `count` evenly spaced samples on a ring at
/// `sample_radius` so a single spike or dip at the center does not own `W`.
pub fn shelf_base_height(
	center: Vec2,
	sample_radius: f32,
	count: u8,
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
) -> f32 {
	let Some(f) = height_at else {
		return 0.0;
	};
	let n = usize::from(count.max(1));
	if n == 1 || sample_radius <= 1e-3 {
		return f(center.x, center.y);
	}
	let mut hs = Vec::with_capacity(n);
	for i in 0..n {
		let ang = (i as f32) * std::f32::consts::TAU / (n as f32);
		let p = center + Vec2::new(ang.cos(), ang.sin()) * sample_radius;
		hs.push(f(p.x, p.y));
	}
	median_f32(&mut hs)
}

/// Jittered lake centroid used by lake stamp construction (for shelf survey).
pub fn planned_center(bounds: Bounds2, seed: u32, params: LakeParams) -> Vec2 {
	let min = bounds.min;
	let max = bounds.max;
	let cell_c = Vec2::new((min.x + max.x) * 0.5, (min.y + max.y) * 0.5);
	let half = Vec2::new((max.x - min.x) * 0.5, (max.y - min.y) * 0.5);
	let short_half = half.x.min(half.y).max(1.0);
	let u = water_scale_u01(seed, min, params);
	let rim_u = rim_width_u01(seed, min, params);
	let Some(probe) = LakeBandBudget::try_from_short_half(short_half, params, u, rim_u) else {
		return cell_c;
	};
	let shore_amp = probe.water_radius() * params.shore_indent_frac.clamp(0.0, 0.45);
	let apron_indent =
		params.apron.indent_frac_min.max(params.apron.indent_frac_max).clamp(0.0, 0.5);
	let apron_amp = probe.apron_width * apron_indent;
	let outer = probe.plateau_radius() + probe.apron_width + shore_amp.max(apron_amp);
	let lo = min + Vec2::splat(outer.max(probe.mu));
	let hi = max - Vec2::splat(outer.max(probe.mu));
	let ox = n11_at(seed, 0x1A7E_C001, min) * params.centroid_jitter * short_half;
	let oz = n11_at(seed, 0x1A7E_C002, min) * params.centroid_jitter * short_half;
	Vec2::new(
		(cell_c.x + ox).clamp(lo.x.min(hi.x), lo.x.max(hi.x)),
		(cell_c.y + oz).clamp(lo.y.min(hi.y), lo.y.max(hi.y)),
	)
}

/// Vertical shelf levels derived from a terrain survey + params.
pub(crate) struct ShelfLevels {
	/// Surveyed shelf / plateau anchor used for rim bank (`+ rim_lift`).
	pub shelf_anchor: f32,
	pub water_level: f32,
}

pub(crate) fn shelf_levels(
	seed: u32,
	anchor: Vec2,
	center: Vec2,
	budget: &LakeBandBudget,
	params: LakeParams,
	height_at: Option<&dyn Fn(f32, f32) -> f32>,
) -> ShelfLevels {
	let base_h = shelf_base_height(
		center,
		shelf_survey_radius(budget),
		params.shelf_sample_count,
		height_at,
	);
	let shelf_anchor = base_h + n11_at(seed, SHELF_AMP_SALT, anchor) * params.shelf_amp;
	ShelfLevels { shelf_anchor, water_level: shelf_anchor - params.water_sink.max(0.0) }
}
