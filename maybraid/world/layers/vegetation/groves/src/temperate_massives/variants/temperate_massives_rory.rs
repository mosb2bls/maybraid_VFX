//! [`BuildWithNoise`] for [`TemperateMassivesRory`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::RorysHeadTrainedSbs;

use crate::temperate_massives::TemperateMassivesRory;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

impl BuildWithNoise<RorysHeadTrainedSbs> for TemperateMassivesRory {
	fn build_with_noise(&self, noise: NoiseParams) -> RorysHeadTrainedSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(50.0);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);

		let mut geometry = RorysHeadTrainedSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.canopy_noise = noise;
		let span = span_fraction(canopy_spread, height);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.95, span * 1.15);
		geometry
	}
}
