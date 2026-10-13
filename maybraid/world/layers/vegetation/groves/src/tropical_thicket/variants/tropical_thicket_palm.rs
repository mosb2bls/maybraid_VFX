//! [`BuildWithNoise`] for [`TropicalThicketPalm`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::PalmBushSbs;

use crate::tropical_thicket::TropicalThicketPalm;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn sample_u32(config: &NoiseConfig, range: &std::ops::RangeInclusive<u32>, salt: f32) -> u32 {
	let lo = *range.start() as usize;
	let hi = (*range.end() as usize).saturating_add(1);
	config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
}

impl BuildWithNoise<PalmBushSbs> for TropicalThicketPalm {
	fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.75);
		let frond_count = sample_u32(&config, &self.frond_count, 2.0);
		let frond_length = sample_f32(&config, self.frond_length, 3.0);
		let crown_spread = sample_f32(&config, self.crown_spread, 4.0);
		let frond_world_scale = (frond_length / height.max(0.5)).clamp(0.15, 1.2)
			* (crown_spread / height.max(0.5)).clamp(0.4, 1.5);

		let mut geometry = PalmBushSbs::default()
			.with_height(height)
			.with_frond_world_scale(frond_world_scale)
			.with_noise_params(noise);
		geometry.crown.fronds_per_ring = frond_count;
		geometry.crown.ring_count = 1;
		geometry
	}
}
