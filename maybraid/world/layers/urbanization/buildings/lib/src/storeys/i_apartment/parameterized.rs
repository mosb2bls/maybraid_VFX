//! Noise-sampled I-frame knobs for an I-Apartment floor plan.

use bevy_math::Vec2;
use procedural_common::{NoiseConfig, NoiseParams};

use crate::fit::{aabb_xz_extent, Confines, FitError};
use crate::openings::{fit_windows_on_run, generate_windows, BaySpec, PlacedBay};
use crate::usage_areas::{MAX_HALL_WIDTH, MIN_HALL_WIDTH};

/// Resolved I-frame layout knobs (stem + optional one-sided / two-sided flanges).
#[derive(Debug, Clone, PartialEq)]
pub struct IApartmentParameterized {
	/// Stem width in meters (X). May be narrower than the end bars.
	pub stem_width: f32,
	/// Flange bar depth in meters (Z thickness of end bars); favors apartment scale.
	pub flange_thickness: f32,
	/// Left/right flange length weights / fill shares. `None` ⇒ arm absent.
	///
	/// When both arms of a bar are present these are relative weights (normalized
	/// against leftover X). When only one arm is present this is the fraction of
	/// leftover X that arm takes.
	pub top_left_share: Option<f32>,
	pub top_right_share: Option<f32>,
	pub bottom_left_share: Option<f32>,
	pub bottom_right_share: Option<f32>,
	/// How densely to pack exterior apertures (`0…1`).
	pub opening_density: f32,
	/// Exterior aperture sizes to pack along outer wall edges (catalog order).
	pub windows: Vec<BaySpec>,
	/// Preferred shaft footprint side length (meters), clamped to the 9-pocket.
	pub shaft_side: f32,
	/// Shaft side kept even where the 9-pocket clamp is tighter (meters); the
	/// shaft then slides inward to stay clear of its host rect's walls.
	pub min_shaft_side: f32,
	/// Corridor clear width (meters) for inter-rect passages and HallsToShafts.
	pub hall_width: f32,
}

pub const MIN_STOREY_HEIGHT: f32 = 2.5;
pub const MIN_FOOTPRINT: f32 = 12.0;
/// Stem may run narrower than the end bars, but stays buildable.
pub const MIN_STEM_WIDTH: f32 = 4.0;
/// End bars (leaves) stay fairly large for apartment room.
pub const MIN_FLANGE_THICKNESS: f32 = 7.0;
pub const MIN_CENTRAL_DEPTH: f32 = 2.5;

pub const MIN_SHAFT_SIDE: f32 = 2.0;
pub const MAX_SHAFT_SIDE: f32 = 5.0;
/// Smallest shaft the 9-pocket clamp leaves by default.
pub const MIN_POCKET_SHAFT_SIDE: f32 = 0.7;

const SALT_STEM: f32 = 1.0;
const SALT_FLANGE_T: f32 = 2.0;
const SALT_SHAPE: f32 = 3.0;
const SALT_ARM_A: f32 = 4.0;
const SALT_ARM_B: f32 = 5.0;
const SALT_SHARE_TL: f32 = 6.0;
const SALT_SHARE_TR: f32 = 7.0;
const SALT_SHARE_BL: f32 = 8.0;
const SALT_SHARE_BR: f32 = 9.0;
const SALT_FILL_TOP: f32 = 10.0;
const SALT_FILL_BOT: f32 = 11.0;
const SALT_OPENINGS: f32 = 12.0;
const SALT_SHAFT: f32 = 13.0;
const SALT_HALL: f32 = 14.0;

impl IApartmentParameterized {
	/// Sample I-frame knobs at the confines center.
	pub fn sample(confines: &Confines, noise: NoiseParams) -> Result<Self, FitError> {
		let footprint = aabb_xz_extent(&confines.bounds);
		let height = (confines.bounds.max.y - confines.bounds.min.y).max(0.0);
		if footprint.x < MIN_FOOTPRINT || footprint.y < MIN_FOOTPRINT {
			return Err(FitError::TooSmall { reason: "footprint" });
		}
		if height < MIN_STOREY_HEIGHT {
			return Err(FitError::TooSmall { reason: "height" });
		}

		let cfg = NoiseConfig::new(noise);
		let c = confines.center();

		// Stem: allow a bit narrower; still leaves most X for flange arms.
		let stem_lo = MIN_STEM_WIDTH.min(footprint.x * 0.2);
		let stem_hi = (footprint.x * 0.32).max(stem_lo + 0.5);
		let stem_width = cfg
			.sample_range_f32_4d(stem_lo, stem_hi, c.x, c.y, c.z, SALT_STEM)
			.clamp(stem_lo, footprint.x * 0.4);

		// Shape family → which arms exist (L needs a single one-sided flange).
		let shape_u = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, SALT_SHAPE);
		let arm_u = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, SALT_ARM_A);
		let arm_v = cfg.sample_range_f32_4d(0.0, 1.0, c.x, c.y, c.z, SALT_ARM_B);
		let (mut tl, mut tr, mut bl, mut br) = sample_arms(shape_u, arm_u, arm_v);

		let flange_bars =
			((tl.is_some() || tr.is_some()) as u8) + ((bl.is_some() || br.is_some()) as u8);
		let max_flange_t = if flange_bars == 0 {
			footprint.y * 0.4
		} else {
			((footprint.y - MIN_CENTRAL_DEPTH) / flange_bars as f32).max(MIN_FLANGE_THICKNESS * 0.5)
		};
		// Leaves favor apartment depth within the Z budget.
		let flange_lo = MIN_FLANGE_THICKNESS.min(max_flange_t);
		let flange_hi = (max_flange_t * 0.95).max(flange_lo);
		let flange_thickness = cfg
			.sample_range_f32_4d(flange_lo, flange_hi, c.x, c.y, c.z, SALT_FLANGE_T)
			.clamp(flange_lo, max_flange_t);

		let weight = |present: bool, salt: f32| -> Option<f32> {
			present.then(|| cfg.sample_range_f32_4d(0.35, 0.65, c.x, c.y, c.z, salt))
		};
		tl = weight(tl.is_some(), SALT_SHARE_TL);
		tr = weight(tr.is_some(), SALT_SHARE_TR);
		bl = weight(bl.is_some(), SALT_SHARE_BL);
		br = weight(br.is_some(), SALT_SHARE_BR);

		// One-arm bars: store how much of leftover X to take (high).
		// Two-arm bars: weights stay relative; fill fraction applied in `flange_lengths`.
		let top_fill = cfg.sample_range_f32_4d(0.75, 0.98, c.x, c.y, c.z, SALT_FILL_TOP);
		let bot_fill = cfg.sample_range_f32_4d(0.75, 0.98, c.x, c.y, c.z, SALT_FILL_BOT);
		normalize_bar_shares(&mut tl, &mut tr, top_fill);
		normalize_bar_shares(&mut bl, &mut br, bot_fill);

		let opening_density = cfg.sample_unit_4d(c.x, c.y, c.z, SALT_OPENINGS);
		let windows = generate_windows(&cfg, c);
		let short = footprint.x.min(footprint.y);
		let shaft_hi = (short * 0.12).clamp(MIN_SHAFT_SIDE, MAX_SHAFT_SIDE);
		let shaft_lo = MIN_SHAFT_SIDE.min(shaft_hi);
		let shaft_side = cfg.sample_range_f32_4d(shaft_lo, shaft_hi, c.x, c.y, c.z, SALT_SHAFT);
		let hall_width =
			cfg.sample_range_f32_4d(MIN_HALL_WIDTH, MAX_HALL_WIDTH, c.x, c.y, c.z, SALT_HALL);

		Ok(Self {
			stem_width,
			flange_thickness,
			top_left_share: tl,
			top_right_share: tr,
			bottom_left_share: bl,
			bottom_right_share: br,
			opening_density,
			windows,
			shaft_side,
			min_shaft_side: MIN_POCKET_SHAFT_SIDE,
			hall_width,
		})
	}

	/// Raise the corridor width and the kept shaft side to a typology's
	/// floors (meters). Larger sampled values stay as sampled.
	pub fn with_minimums(mut self, hall_width: f32, shaft_side: f32) -> Self {
		self.hall_width = self.hall_width.max(hall_width);
		self.shaft_side = self.shaft_side.max(shaft_side);
		self.min_shaft_side = self.min_shaft_side.max(shaft_side);
		self
	}

	pub fn has_top_flange(&self) -> bool {
		self.top_left_share.is_some() || self.top_right_share.is_some()
	}

	pub fn has_bottom_flange(&self) -> bool {
		self.bottom_left_share.is_some() || self.bottom_right_share.is_some()
	}

	/// Resolve concrete flange lengths for a footprint (meters).
	///
	/// Lengths never exceed the leftover X budget (`footprint.x - stem_w`).
	pub fn flange_lengths(
		&self,
		footprint: Vec2,
		stem_w: f32,
	) -> (Option<f32>, Option<f32>, Option<f32>, Option<f32>) {
		let leftover = (footprint.x - stem_w).max(0.0);
		let (tl, tr) = pair_length(self.top_left_share, self.top_right_share, leftover);
		let (bl, br) = pair_length(self.bottom_left_share, self.bottom_right_share, leftover);
		(tl, tr, bl, br)
	}

	/// Pack exterior windows along a wall run (shared façade bay catalog policy).
	pub fn fit_windows_on_run(&self, run_length: f32) -> Vec<PlacedBay> {
		fit_windows_on_run(&self.windows, self.opening_density, run_length)
	}
}

/// For a single bar: one arm ⇒ share is leftover fraction; two arms ⇒ relative weights
/// already scaled so `left + right == fill` (see [`normalize_bar_shares`]).
fn pair_length(left: Option<f32>, right: Option<f32>, leftover: f32) -> (Option<f32>, Option<f32>) {
	match (left, right) {
		(Some(l), Some(r)) => {
			let sum = (l + r).max(1e-4);
			// Shares were normalized to sum ≈ fill ∈ [0,1].
			let budget = leftover * sum.min(1.0);
			(Some(budget * l / sum), Some(budget * r / sum))
		}
		(Some(l), None) => (Some(leftover * l.clamp(0.0, 1.0)), None),
		(None, Some(r)) => (None, Some(leftover * r.clamp(0.0, 1.0))),
		(None, None) => (None, None),
	}
}

/// One arm: rewrite share as fill fraction of leftover. Two arms: scale weights to sum to `fill`.
fn normalize_bar_shares(left: &mut Option<f32>, right: &mut Option<f32>, fill: f32) {
	let fill = fill.clamp(0.0, 1.0);
	match (*left, *right) {
		(Some(l), Some(r)) => {
			let sum = (l + r).max(1e-4);
			*left = Some(fill * l / sum);
			*right = Some(fill * r / sum);
		}
		(Some(_), None) => *left = Some(fill),
		(None, Some(_)) => *right = Some(fill),
		(None, None) => {}
	}
}

/// Map unit noise → arm presence.
///
/// Rough mix: I ~60%, T ~15%, L ~15%, Z ~5%, stem ~5%.
fn sample_arms(
	shape_u: f32,
	arm_u: f32,
	arm_v: f32,
) -> (Option<f32>, Option<f32>, Option<f32>, Option<f32>) {
	let on = Some(1.0);
	if shape_u < 0.60 {
		(on, on, on, on)
	} else if shape_u < 0.75 {
		if arm_u < 0.5 {
			(on, on, None, None)
		} else {
			(None, None, on, on)
		}
	} else if shape_u < 0.90 {
		let top = arm_u < 0.5;
		let left = arm_v < 0.5;
		match (top, left) {
			(true, true) => (on, None, None, None),
			(true, false) => (None, on, None, None),
			(false, true) => (None, None, on, None),
			(false, false) => (None, None, None, on),
		}
	} else if shape_u < 0.95 {
		if arm_u < 0.5 {
			(on, None, None, on)
		} else {
			(None, on, on, None)
		}
	} else {
		(None, None, None, None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy_math::bounding::Aabb3d;
	use bevy_math::Vec3;

	fn large_confines() -> Confines {
		Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-22.0, 0.0, -18.0),
			Vec3::new(22.0, 3.5, 18.0),
		))
	}

	#[test]
	fn sample_accepts_large_footprint() {
		let p = IApartmentParameterized::sample(&large_confines(), NoiseParams::default()).unwrap();
		assert!(p.stem_width >= MIN_STEM_WIDTH * 0.5);
		assert!(p.flange_thickness >= MIN_FLANGE_THICKNESS * 0.5);
	}

	#[test]
	fn sample_rejects_tiny() {
		let confines = Confines::from_bounds(Aabb3d::from_min_max(
			Vec3::new(-2.0, 0.0, -2.0),
			Vec3::new(2.0, 3.0, 2.0),
		));
		assert!(matches!(
			IApartmentParameterized::sample(&confines, NoiseParams::default()),
			Err(FitError::TooSmall { .. })
		));
	}

	#[test]
	fn sample_can_produce_l() {
		let confines = large_confines();
		let mut saw_l = false;
		for seed in 0..200 {
			let p = IApartmentParameterized::sample(
				&confines,
				NoiseParams { seed, ..NoiseParams::default() },
			)
			.unwrap();
			let arms = [
				p.top_left_share.is_some(),
				p.top_right_share.is_some(),
				p.bottom_left_share.is_some(),
				p.bottom_right_share.is_some(),
			]
			.into_iter()
			.filter(|&a| a)
			.count();
			let one_bar = p.has_top_flange() ^ p.has_bottom_flange();
			if one_bar && arms == 1 {
				saw_l = true;
				break;
			}
		}
		assert!(saw_l, "expected an L among seeds 0..200");
	}

	#[test]
	fn flange_lengths_stay_within_leftover() {
		let confines = large_confines();
		let footprint = aabb_xz_extent(&confines.bounds);
		for seed in 0..80 {
			let p = IApartmentParameterized::sample(
				&confines,
				NoiseParams { seed, ..NoiseParams::default() },
			)
			.unwrap();
			let leftover = (footprint.x - p.stem_width).max(0.0);
			let (tl, tr, bl, br) = p.flange_lengths(footprint, p.stem_width);
			let top = tl.unwrap_or(0.0) + tr.unwrap_or(0.0);
			let bot = bl.unwrap_or(0.0) + br.unwrap_or(0.0);
			assert!(top <= leftover + 1e-3, "seed={seed} top {top} > leftover {leftover}");
			assert!(bot <= leftover + 1e-3, "seed={seed} bot {bot} > leftover {leftover}");
		}
	}
}
