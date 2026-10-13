//! [`BuildWithNoise`] for [`LevantineScrubHedge`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};

use crate::levantine_scrub::LevantineScrubHedge;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

pub struct HedgeSamples {
	pub height: f32,
	pub footprint_xz: f32,
	pub density: f32,
	pub seed: u32,
}

impl BuildWithNoise<HedgeSamples> for LevantineScrubHedge {
	fn build_with_noise(&self, noise: NoiseParams) -> HedgeSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(0.5);
		let width = sample_f32(&config, self.width, 2.0).max(0.4);
		let density = sample_f32(&config, self.density, 3.0).clamp(0.05, 1.0);
		HedgeSamples { height, footprint_xz: width, density, seed: noise.seed as u32 }
	}
}
