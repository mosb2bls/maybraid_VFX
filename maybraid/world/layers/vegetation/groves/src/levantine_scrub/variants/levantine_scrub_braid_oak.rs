//! [`BuildWithNoise`] for [`LevantineScrubBraidOak`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::BraidOakTreeSbs;

use crate::levantine_scrub::LevantineScrubBraidOak;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.25, 1.20)
}

impl BuildWithNoise<BraidOakTreeSbs> for LevantineScrubBraidOak {
	fn build_with_noise(&self, noise: NoiseParams) -> BraidOakTreeSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(1.8);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = BraidOakTreeSbs::default();
		geometry.apply_braid_preset();
		geometry.scale.tree_height = height;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.82, span * 1.02);
		geometry.canopy_noise = noise;
		geometry
	}
}
