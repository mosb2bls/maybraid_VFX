//! [`BuildWithNoise`] for [`AlpineLiamsConifer`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::LiamsConiferSbs;

use crate::alpine::AlpineLiamsConifer;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

impl BuildWithNoise<LiamsConiferSbs> for AlpineLiamsConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> LiamsConiferSbs {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_density = sample_f32(&config, self.canopy_density, 2.0);

		let mut geometry = LiamsConiferSbs::default();
		geometry.rings.spacing = (0.02 + canopy_density * 0.04).clamp(0.02, 0.06);
		geometry.scale.stalk_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.canopy_noise = noise;
		geometry
	}
}
