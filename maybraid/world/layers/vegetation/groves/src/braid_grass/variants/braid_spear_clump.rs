//! [`BuildWithNoise`] for [`BraidSpearClump`].
//!
//! The belly half-width is **length-proportional** (`length * belly_factor`); the base tapers
//! to roughly a third of the belly, keeping the authored belly→tip ribbon profile.

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::SpearTuftShape;

use crate::braid_grass::BraidSpearClump;

impl BuildWithNoise<SpearTuftShape> for BraidSpearClump {
	fn build_with_noise(&self, noise: NoiseParams) -> SpearTuftShape {
		let config = NoiseConfig::new(noise);
		let sample_f32 = |range: UnitRange, salt| {
			let lo = range.start.min(range.end);
			let hi = range.start.max(range.end);
			config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
		};
		let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
			let lo = *range.start() as usize;
			let hi = (*range.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
		};

		let spear_length = sample_f32(self.height, 1.0).max(0.1);
		let belly_half_width = spear_length * sample_f32(self.belly_factor, 2.0);

		SpearTuftShape {
			spear_count: sample_u32(&self.spear_count, 3.0),
			spear_length,
			base_half_width: belly_half_width * 0.35,
			belly_half_width,
			max_tilt_radians: sample_f32(self.max_tilt_radians, 4.0).max(0.01),
			bend_segments: sample_u32(&self.bend_segments, 5.0).max(1),
			seed: noise.seed,
			..SpearTuftShape::default()
		}
	}
}
