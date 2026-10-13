//! Stamp Valley Basins (unchained) — [RFC-105 §3.8.1](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#381-jersey-valley-basins-unchained).

use crate::config::{DownhillPair, FractalAnchors, HysteresisSpine};
use crate::modulation::{RegionAffineModulation, RegionGradingModulation, StampModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use crate::stamp::{scale_additive, scale_near_one, StampSemantics, StampSet, StampStrength};
use bevy_math::Vec2;
use procedural_common::{Bounds2, SeededHash};

/// Valley cross-section family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValleyCrossSection {
	/// Sharp thalweg, steeper banks.
	V,
	/// Flatter floor, gentler walls.
	U,
	/// Bias the corridor to one bank.
	Asymmetric,
}

/// Floor readiness for a later hydrology stamp (height-only for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValleyFloorKind {
	/// Dry arroyo profile.
	Arroyo,
	/// Floor that a later water stamp may occupy.
	SpillwayReady,
}

/// Parameters for a single unchained valley basin inside a bound.
#[derive(Debug, Clone, Copy)]
pub struct ValleyBasinParams {
	pub cross_section: ValleyCrossSection,
	pub floor: ValleyFloorKind,
	/// Corridor half-width as a fraction of the shorter bound edge (`0.05..0.45`).
	pub width_frac: f32,
	/// Floor depression (world units); modulated by [`StampStrength`].
	pub depth: f32,
	/// Scale applied to base elevation inside the corridor (`< 1` softens relief).
	pub floor_scale: f32,
}

impl Default for ValleyBasinParams {
	fn default() -> Self {
		Self {
			cross_section: ValleyCrossSection::U,
			floor: ValleyFloorKind::SpillwayReady,
			width_frac: 0.22,
			depth: 12.0,
			// Keep base relief; depth comes from the negative offset.
			floor_scale: 1.0,
		}
	}
}

impl StampStrength for ValleyBasinParams {
	fn with_strength(mut self, strength: f32) -> Self {
		self.depth = scale_additive(self.depth, strength);
		self.floor_scale = scale_near_one(self.floor_scale, strength);
		self
	}
}

/// Profile knobs derived from [`ValleyCrossSection`].
#[derive(Debug, Clone, Copy)]
struct CrossProfile {
	floor_scale: f32,
	depth: f32,
	bank_inner: f32,
	bank_outer: f32,
	width_scale: f32,
}

impl CrossProfile {
	fn from_params(params: &ValleyBasinParams) -> Self {
		match params.cross_section {
			ValleyCrossSection::V => Self {
				floor_scale: params.floor_scale,
				depth: params.depth * 1.15,
				bank_inner: 0.3,
				bank_outer: 0.9,
				width_scale: 1.0,
			},
			ValleyCrossSection::U => Self {
				floor_scale: params.floor_scale,
				depth: params.depth,
				bank_inner: 0.4,
				bank_outer: 1.0,
				width_scale: 1.0,
			},
			ValleyCrossSection::Asymmetric => Self {
				floor_scale: params.floor_scale,
				depth: params.depth,
				bank_inner: 0.35,
				bank_outer: 0.95,
				width_scale: 1.1,
			},
		}
	}
}

/// Constructed valley basin stamp (axis + height ops).
#[derive(Debug, Clone)]
pub struct ValleyBasin {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: ValleyBasinParams,
	pub start: Vec2,
	pub end: Vec2,
	pub path: Vec<Vec2>,
	pub stamp: StampSet,
}

impl ValleyBasin {
	/// Build a fractal-driven valley axis inside `bounds` and emit height modulations.
	///
	/// `height_at` samples the noise base for endpoint grade (typically durham
	/// `BaseTerrainNoise::height_at`). When `None`, endpoints use `0.0`.
	pub fn from_bounds(
		bounds: Bounds2,
		seed: u32,
		params: ValleyBasinParams,
		height_at: Option<&dyn Fn(f32, f32) -> f32>,
	) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let profile = CrossProfile::from_params(&params);
		// Larger leaves: keep floor depth more even along the reach.
		let extent_t = (short / crate::stamp::SOFTMASK_REFERENCE_SHORT).sqrt().clamp(1.0, 3.0);
		let depth_falloff = (0.3 / extent_t).clamp(0.05, 0.3);

		let (start, end) = FractalAnchors::default().sample(bounds, seed, 0);
		let path = HysteresisSpine::default().build(bounds, seed.wrapping_add(17), start, end);
		let a = *path.first().unwrap_or(&start);
		let b = *path.last().unwrap_or(&end);

		let half_width = short * params.width_frac.clamp(0.05, 0.45) * profile.width_scale;
		let axis = (b - a).normalize_or_zero();
		let perp = Vec2::new(-axis.y, axis.x);
		let side = if hash.unit(9) > 0.5 { 1.0 } else { -1.0 };
		let lateral = if params.cross_section == ValleyCrossSection::Asymmetric {
			perp * (half_width * 0.35 * side)
		} else {
			Vec2::ZERO
		};

		let (start_pt, start_h, end_pt, end_h) = DownhillPair::order(a, b, height_at);

		let bank_noise = RegionNoise::from_seed(seed.wrapping_add(41), 0.012, half_width * 0.08);
		let inner_r = half_width * profile.bank_inner;
		let outer_r = half_width * profile.bank_outer;

		// Denser circle softmasks along the spine — relative incision only.
		let mut modulations = Vec::new();
		let sample_stride = ((path.len() / 4).max(1)).min(2);
		for (i, p) in path.iter().enumerate().step_by(sample_stride) {
			let center = *p + lateral;
			let region = Region2D::Circle(CircleRegion { center, radius: half_width });
			let depth_t = i as f32 / path.len().saturating_sub(1).max(1) as f32;
			let local_depth = profile.depth * (1.0 - depth_falloff * depth_t);
			modulations.push(StampModulation::Affine(
				RegionAffineModulation::new(
					region,
					profile.floor_scale,
					-local_depth,
					inner_r,
					outer_r,
				)
				.with_noise(bank_noise.clone()),
			));
		}

		// Downhill floor bias that never raises natural lows.
		let grade_center = (start_pt + end_pt) * 0.5 + lateral;
		let grade_region =
			Region2D::Circle(CircleRegion { center: grade_center, radius: half_width * 1.5 });
		modulations.push(StampModulation::Grading(
			RegionGradingModulation::new(
				grade_region,
				start_pt,
				start_h - profile.depth * 0.35,
				end_pt,
				end_h - profile.depth * 0.15,
				inner_r * 0.9,
				outer_r * 1.15,
			)
			.with_noise(bank_noise)
			.depression_only(),
		));

		let mut semantics = StampSemantics::default().with_tag("bank");
		semantics = match params.floor {
			ValleyFloorKind::Arroyo => semantics.with_tag("arroyo"),
			ValleyFloorKind::SpillwayReady => semantics.with_tag("spillway_ready"),
		};

		Self {
			bounds,
			seed,
			params,
			start: start_pt,
			end: end_pt,
			path: path.clone(),
			stamp: StampSet { modulations, spine: path, semantics },
		}
	}

	/// Convenience: default params, no height oracle.
	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, ValleyBasinParams::default(), None)
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn valley_builds_spine_and_modulations() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let basin = ValleyBasin::from_bounds_default(bounds, 42);
		assert!(basin.path.len() >= 2);
		assert!(basin.stamp.modulations.len() >= 2);
		assert!(basin.stamp.semantics.tags.contains(&"bank"));
		assert!(basin.stamp.semantics.tags.contains(&"spillway_ready"));
		Ok(())
	}

	#[test]
	fn valley_depresses_center_relative_to_outside() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let basin = ValleyBasin::from_bounds(
			bounds,
			7,
			ValleyBasinParams {
				cross_section: ValleyCrossSection::V,
				floor: ValleyFloorKind::Arroyo,
				width_frac: 0.2,
				depth: 20.0,
				floor_scale: 1.0,
			},
			Some(&|_, _| 100.0),
		);
		let mid = (basin.start + basin.end) * 0.5;
		let outside = bounds.min + Vec2::new(5.0, 5.0);
		let h_mid = basin.stamp.apply_elevation(100.0, mid.x, mid.y);
		let h_out = basin.stamp.apply_elevation(100.0, outside.x, outside.y);
		assert!(h_mid < h_out);
		assert!(basin.stamp.semantics.tags.contains(&"arroyo"));
		Ok(())
	}

	#[test]
	fn valley_does_not_raise_natural_lows() -> anyhow::Result<()> {
		let bounds = Bounds2::from_xz(0.0, 0.0, 400.0, 400.0);
		let basin =
			ValleyBasin::from_bounds(bounds, 7, ValleyBasinParams::default(), Some(&|_, _| 80.0));
		let mid = (basin.start + basin.end) * 0.5;
		assert!(basin.stamp.apply_elevation(15.0, mid.x, mid.y) <= 15.0);
		Ok(())
	}
}
