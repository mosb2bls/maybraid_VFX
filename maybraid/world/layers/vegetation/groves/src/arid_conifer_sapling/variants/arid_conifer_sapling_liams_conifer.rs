//! [`BuildWithNoise`] for [`AridConiferSaplingLiamsConifer`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::LiamsConiferSbs;

use crate::arid_conifer_sapling::AridConiferSaplingLiamsConifer;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn sample_sapling_height(config: &NoiseConfig, height: UnitRange) -> f32 {
	sample_f32(config, height, 1.0).max(2.0)
}

impl BuildWithNoise<LiamsConiferSbs> for AridConiferSaplingLiamsConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> LiamsConiferSbs {
		let config = NoiseConfig::new(noise);
		let mut geometry = LiamsConiferSbs::default();
		geometry.rings.spacing = sample_f32(&config, self.canopy_density, 1.5);
		geometry.scale.stalk_height = sample_sapling_height(&config, self.height);
		geometry.scale.stalk_base_radius = Some(sample_f32(&config, self.stalk_radius, 1.5));
		geometry.canopy_noise = noise;
		geometry
	}
}
