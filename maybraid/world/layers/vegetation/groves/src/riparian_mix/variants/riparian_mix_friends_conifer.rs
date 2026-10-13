//! [`BuildWithNoise`] for [`RiparianMixFriendsConifer`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::FriendsConiferSbs;

use crate::riparian_mix::RiparianMixFriendsConifer;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

pub struct FriendsConiferSamples {
	pub geometry: FriendsConiferSbs,
	pub apex_canopy_spawn_fraction: f32,
	pub splay_radius_fraction_of_height: f32,
}

impl BuildWithNoise<FriendsConiferSamples> for RiparianMixFriendsConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> FriendsConiferSamples {
		let config = NoiseConfig::new(noise);
		let height =
			sample_f32(&config, self.height, 1.0).max(self.height.start.min(self.height.end));
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 3.0);

		let mut geometry = FriendsConiferSbs::default();
		geometry.projection.child_count_range = procedural_common::UsizeRange::new(1, 2);
		geometry.scale.stalk_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.canopy_noise = noise;

		FriendsConiferSamples {
			geometry,
			apex_canopy_spawn_fraction: canopy_density.clamp(0.35, 0.75),
			splay_radius_fraction_of_height: (canopy_spread / height).clamp(0.014, 0.08),
		}
	}
}
