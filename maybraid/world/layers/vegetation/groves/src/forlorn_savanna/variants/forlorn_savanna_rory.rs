//! [`BuildWithNoise`] for [`ForlornSavannaRory`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::RorysHeadTrainedSbs;

use crate::forlorn_savanna::ForlornSavannaRory;

/// Savanna umbrellas carry a slightly heavier head than the playground default.
const SAVANNA_LEAF_RADIUS_FRACTION: f32 = 0.156;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

impl BuildWithNoise<RorysHeadTrainedSbs> for ForlornSavannaRory {
	fn build_with_noise(&self, noise: NoiseParams) -> RorysHeadTrainedSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.75);
		let stalk_radius_fraction = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);

		let mut geometry = RorysHeadTrainedSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some((stalk_radius_fraction * height).max(0.35));
		geometry.canopy.leaf_radius_fraction = SAVANNA_LEAF_RADIUS_FRACTION;
		geometry.canopy_noise = noise;
		let span = span_fraction(canopy_spread, height);
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.95, span * 1.15);
		geometry
	}
}
