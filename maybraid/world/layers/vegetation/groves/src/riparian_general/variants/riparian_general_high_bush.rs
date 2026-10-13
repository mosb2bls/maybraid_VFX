//! [`BuildWithNoise`] for [`RiparianGeneralHighBush`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::anchors::high_bush::{
	DEFAULT_ANCHOR_LIFT_FRACTION, DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
	DEFAULT_SEGMENT_LENGTH_FRACTION_LO, DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
	DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
};
use sbs_geometry::{HighBushFoliageStyle, HighBushShootsShape};

use crate::riparian_general::RiparianGeneralHighBush;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

impl BuildWithNoise<HighBushShootsShape> for RiparianGeneralHighBush {
	fn build_with_noise(&self, noise: NoiseParams) -> HighBushShootsShape {
		let config = NoiseConfig::new(noise);
		let sample_u32 = |range: &std::ops::RangeInclusive<u32>, salt| {
			let lo = *range.start() as usize;
			let hi = (*range.end() as usize).saturating_add(1);
			config.sample_range_usize_4d(lo, hi, 0.0, 0.0, 0.0, salt) as u32
		};

		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let leaf_radius = sample_f32(&config, self.leaf_radius, 2.0).max(0.01);

		HighBushShootsShape {
			height,
			anchor_lift_fraction: DEFAULT_ANCHOR_LIFT_FRACTION,
			shoot_count: sample_u32(&self.shoot_count, 3.0),
			radial_strength: sample_f32(&config, self.radial_strength, 5.0),
			vertical_bias: sample_f32(&config, self.vertical_bias, 6.0),
			branch_depth: sample_u32(&self.branch_depth, 4.0) as usize,
			segment_length_fraction_lo: DEFAULT_SEGMENT_LENGTH_FRACTION_LO,
			segment_length_fraction_hi: DEFAULT_SEGMENT_LENGTH_FRACTION_HI,
			segment_radius_fraction_lo: DEFAULT_SEGMENT_RADIUS_FRACTION_LO,
			segment_radius_fraction_hi: DEFAULT_SEGMENT_RADIUS_FRACTION_HI,
			leaf_radius_fraction: leaf_radius / height,
			foliage_style: HighBushFoliageStyle::PlaneSplay,
			chain_noise: noise,
		}
	}
}
