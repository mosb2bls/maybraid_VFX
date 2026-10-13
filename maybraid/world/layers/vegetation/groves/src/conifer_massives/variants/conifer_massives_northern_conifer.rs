//! [`BuildWithNoise`] for [`ConiferMassivesNorthernConifer`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::NorthernConiferSbs;

use crate::conifer_massives::ConiferMassivesNorthernConifer;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn dense_density_fraction(canopy_density: f32) -> f32 {
	canopy_density.clamp(0.50, 0.85)
}

/// Playground default from [`NorthernConifer`] (RFC `0.048 × H`).
const NORTHERN_SPLAY_RADIUS_FRACTION: f32 = 0.048;

pub struct NorthernConiferSamples {
	pub geometry: NorthernConiferSbs,
	pub splay_radius_fraction_of_height: f32,
	pub splay_spawn_fraction: f32,
	pub apex_canopy_spawn_fraction: f32,
}

impl BuildWithNoise<NorthernConiferSamples> for ConiferMassivesNorthernConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> NorthernConiferSamples {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 3.0);
		let density = dense_density_fraction(canopy_density);

		let mut geometry = NorthernConiferSbs::default();
		geometry.liams.scale.stalk_height = height;
		geometry.liams.scale.stalk_base_radius = Some(stalk_radius);
		geometry.apply_northern_preset();
		geometry.liams.canopy_noise = noise;

		NorthernConiferSamples {
			geometry,
			splay_radius_fraction_of_height: (canopy_spread / height)
				.clamp(0.02, NORTHERN_SPLAY_RADIUS_FRACTION * 1.35),
			splay_spawn_fraction: (0.45 + density * 0.40).clamp(0.55, 0.92),
			apex_canopy_spawn_fraction: density,
		}
	}
}
