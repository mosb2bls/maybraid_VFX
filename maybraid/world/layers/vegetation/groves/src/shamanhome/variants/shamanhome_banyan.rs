//! [`BuildWithNoise`] for [`ShamanhomeBanyan`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::SopesBanyanSbs;

use crate::shamanhome::ShamanhomeBanyan;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

pub struct SopeBanyanSamples {
	pub geometry: SopesBanyanSbs,
}

impl BuildWithNoise<SopeBanyanSamples> for ShamanhomeBanyan {
	fn build_with_noise(&self, noise: NoiseParams) -> SopeBanyanSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(4.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let descender_threshold = sample_f32(&config, self.descender_density, 3.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = SopesBanyanSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.canopy_height = height * 2.0;
		geometry.scale.stalk_base_radius = stalk_radius;
		geometry.projection.length_fraction_of_height = UnitRange::new(span * 0.05, span * 0.18);
		geometry.growth.descender_threshold = descender_threshold;
		geometry.leaf_ball_factor = 0.25 + canopy_density * 0.35;
		geometry.canopy_noise = noise;

		SopeBanyanSamples { geometry }
	}
}
