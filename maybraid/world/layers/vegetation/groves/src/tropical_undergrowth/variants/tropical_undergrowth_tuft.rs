//! [`BuildWithNoise`] for [`TropicalUndergrowthTuft`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::BladeTuftShape;

use crate::tropical_undergrowth::TropicalUndergrowthTuft;

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

impl BuildWithNoise<BladeTuftShape> for TropicalUndergrowthTuft {
	fn build_with_noise(&self, noise: NoiseParams) -> BladeTuftShape {
		let config = NoiseConfig::new(noise);
		let blade_length = sample_f32(&config, self.height, 1.0).max(0.05);
		let blade_width = blade_length * sample_f32(&config, self.width_factor, 2.0);

		BladeTuftShape {
			blade_count: sample_u32(&config, &self.blade_count, 3.0),
			blade_length,
			blade_width,
			max_tilt_radians: sample_f32(&config, self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&config, &self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..BladeTuftShape::default()
		}
	}
}
