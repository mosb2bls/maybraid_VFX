//! [`BuildWithNoise`] for [`TradeWindsWaialeaPalm`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::WaialeaPalmSbs;

use crate::trade_winds::TradeWindsWaialeaPalm;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn waialea_height_floor(palm: &TradeWindsWaialeaPalm) -> f32 {
	palm.height.start.min(palm.height.end)
}

impl BuildWithNoise<WaialeaPalmSbs> for TradeWindsWaialeaPalm {
	fn build_with_noise(&self, noise: NoiseParams) -> WaialeaPalmSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(waialea_height_floor(self));
		let crown_density = sample_f32(&config, self.crown_density, 2.0);

		let mut geometry = WaialeaPalmSbs::default();
		geometry.scale.stalk_height = height;
		geometry.crown.ring_count = 2 + (crown_density * 2.0).round() as u32;
		geometry.crown.fronds_per_ring = 8 + (crown_density * 7.0).round() as u32;
		geometry.frond_world_scale = 0.55 + crown_density * 0.35;
		geometry.trunk_noise = noise;
		geometry
	}
}
