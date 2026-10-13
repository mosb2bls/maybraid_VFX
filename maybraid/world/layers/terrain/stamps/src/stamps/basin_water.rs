//! Stamp Basin Waters (large hydrology chains) — [RFC-105 §3.8.5](https://github.com/ramate-io/maybraid/tree/main/rfc/rfc-000-000-105-procedural-terrain#385-jersey-basin-waters-large-hydrology-chains).
//!
//! Height shaping for a macro lake body plus branched outlet / tributary stubs.
//! Wet rendering and full reach records are deferred to Watershed follow-on.

use crate::config::{FractalAnchors, HysteresisSpine, JitteredCenter, SoftmaskAlongSpine};
use crate::modulation::{RegionAffineModulation, StampModulation};
use crate::region::{CircleRegion, Region2D, RegionNoise};
use crate::stamp::{StampSemantics, StampSet};
use bevy_math::Vec2;
use procedural_common::{Bounds2, HysteresisConfig, HysteresisGraph, SeededHash};

#[derive(Debug, Clone, Copy)]
pub struct BasinWaterParams {
	pub lake_frac: f32,
	pub lake_depth: f32,
	pub outlet_count: usize,
	pub channel_width_frac: f32,
	pub channel_depth: f32,
}

impl Default for BasinWaterParams {
	fn default() -> Self {
		Self {
			lake_frac: 0.32,
			lake_depth: 16.0,
			outlet_count: 2,
			channel_width_frac: 0.08,
			channel_depth: 8.0,
		}
	}
}

#[derive(Debug, Clone)]
pub struct BasinWater {
	pub bounds: Bounds2,
	pub seed: u32,
	pub params: BasinWaterParams,
	pub drainage_id: u32,
	pub lake_center: Vec2,
	pub outlets: Vec<Vec<Vec2>>,
	pub stamp: StampSet,
}

impl BasinWater {
	pub fn from_bounds(bounds: Bounds2, seed: u32, params: BasinWaterParams) -> Self {
		let hash = SeededHash::new(seed);
		let short = bounds.extent().min_element().max(1.0);
		let drainage_id = seed.wrapping_mul(0x85EB_CA6B);
		let lake_center = JitteredCenter::default().sample(bounds, seed, 600);
		let lake_r = short * params.lake_frac.clamp(0.15, 0.45);
		let lake_noise = RegionNoise::from_seed(seed.wrapping_add(1), 0.012, lake_r * 0.08);

		let mut modulations = vec![StampModulation::Affine(
			RegionAffineModulation::new(
				Region2D::Circle(CircleRegion { center: lake_center, radius: lake_r }),
				0.35,
				-params.lake_depth,
				lake_r * 0.4,
				lake_r * 0.95,
			)
			.with_noise(lake_noise),
		)];

		let outlet_n = params.outlet_count.clamp(1, 4);
		let channel_w = short * params.channel_width_frac.clamp(0.04, 0.16);
		let channel_noise = RegionNoise::from_seed(seed.wrapping_add(2), 0.03, channel_w * 0.1);
		let mut outlets = Vec::with_capacity(outlet_n);
		let mut spine = vec![lake_center];

		for i in 0..outlet_n {
			let (start_hint, end) =
				FractalAnchors::default().sample(bounds, seed, 610 + i as u32 * 17);
			let start = lake_center.lerp(start_hint, 0.55);
			let path = if i == 0 {
				HysteresisSpine::default().build(
					bounds,
					seed.wrapping_add(40 + i as u32),
					start,
					end,
				)
			} else {
				// Branched degree-2 spur from the lake toward a far tip.
				let tip = bounds.project(
					lake_center
						+ (end - lake_center).normalize_or_zero()
							* short * (0.35 + 0.25 * hash.unit(i as u32 + 3)),
				);
				HysteresisGraph::degree2(
					bounds,
					seed.wrapping_add(50 + i as u32),
					start,
					tip,
					&HysteresisConfig {
						max_segments: 16,
						step_len: channel_w * 1.2,
						..HysteresisConfig::default()
					},
				)
				.primary_polyline()
			};
			modulations.extend(SoftmaskAlongSpine::default().build(
				&path,
				channel_w * (1.0 - 0.15 * i as f32),
				0.5,
				-params.channel_depth,
				0.2,
				0.65,
				&channel_noise,
				Vec2::ZERO,
			));
			spine.extend(path.iter().copied());
			outlets.push(path);
		}

		Self {
			bounds,
			seed,
			params,
			drainage_id,
			lake_center,
			outlets,
			stamp: StampSet {
				modulations,
				spine,
				semantics: StampSemantics::default()
					.with_drainage_id(drainage_id)
					.with_tag("basin_water")
					.with_tag("lake")
					.with_tag("outlet")
					.with_tag("tributary")
					.with_tag("junction")
					.with_tag("pour_point")
					.with_tag("water_surface_target"),
			},
		}
	}

	pub fn from_bounds_default(bounds: Bounds2, seed: u32) -> Self {
		Self::from_bounds(bounds, seed, BasinWaterParams::default())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn basin_has_outlets() -> anyhow::Result<()> {
		let b = BasinWater::from_bounds_default(Bounds2::from_xz(0.0, 0.0, 640.0, 640.0), 4);
		assert!(!b.outlets.is_empty());
		assert_eq!(b.stamp.semantics.drainage_id, Some(b.drainage_id));
		Ok(())
	}
}
