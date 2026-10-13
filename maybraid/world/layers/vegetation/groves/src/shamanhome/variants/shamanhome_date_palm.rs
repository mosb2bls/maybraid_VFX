//! [`BuildWithNoise`] for [`ShamanhomeDatePalm`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::DatePalmSbs;

use crate::shamanhome::ShamanhomeDatePalm;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

impl BuildWithNoise<DatePalmSbs> for ShamanhomeDatePalm {
	fn build_with_noise(&self, noise: NoiseParams) -> DatePalmSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(3.5);
		let crown_density = sample_f32(&config, self.crown_density, 2.0);

		let mut geometry = DatePalmSbs::default();
		geometry.scale.stalk_height = height;
		geometry.crown.ring_count = 2 + (crown_density * 2.0).round() as u32;
		geometry.crown.fronds_per_ring = 5 + (crown_density * 5.0).round() as u32;
		geometry.frond_world_scale = 0.28 + crown_density * 0.24;
		geometry.crown_tuft_scale_factor = 0.04 + crown_density * 0.02;
		geometry.trunk_noise = noise;
		geometry
	}
}
