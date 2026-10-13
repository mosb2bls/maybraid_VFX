//! [`BuildWithNoise`] for [`ConiferMassivesTemperateConifer`].

use procedural_common::UsizeRange;
use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::FriendsConiferSbs;

use crate::conifer_massives::ConiferMassivesTemperateConifer;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn moderate_density_fraction(canopy_density: f32) -> f32 {
	canopy_density.clamp(0.35, 0.65)
}

pub struct TemperateConiferSamples {
	pub geometry: FriendsConiferSbs,
	pub fronds_per_joint: UnitRange,
	pub frond_length_fraction: UnitRange,
	pub frond_spawn_fraction: f32,
	pub frond_world_scale: f32,
	pub apex_canopy_spawn_fraction: f32,
}

impl BuildWithNoise<TemperateConiferSamples> for ConiferMassivesTemperateConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> TemperateConiferSamples {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let canopy_density = sample_f32(&config, self.canopy_density, 2.0);
		let density = moderate_density_fraction(canopy_density);

		let mut inner = FriendsConiferSbs::default();
		inner.apply_temperate_preset();
		inner.scale.stalk_height = height;
		inner.scale.stalk_base_radius = Some((height * 0.025).clamp(0.35, 2.50));
		inner.projection.child_count_range = UsizeRange::new(1, 3);
		inner.canopy_noise = noise;

		let frond_spawn_fraction = (0.45 + density * 0.45).clamp(0.45, 0.95);
		let fronds_hi = 1.0 + (density * 1.2).round();
		let frond_len_lo = 0.030 + density * 0.010;
		let frond_len_hi = 0.045 + density * 0.030;

		TemperateConiferSamples {
			geometry: inner,
			fronds_per_joint: UnitRange::new(1.0, fronds_hi),
			frond_length_fraction: UnitRange::new(frond_len_lo, frond_len_hi),
			frond_spawn_fraction,
			frond_world_scale: 0.85 + density * 0.35,
			apex_canopy_spawn_fraction: 0.72 * (0.65 + density * 0.35),
		}
	}
}
