//! [`BuildWithNoise`] for [`JerrysChaparralFriendsConifer`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::FriendsConiferSbs;

use crate::jerrys_chaparral::JerrysChaparralFriendsConifer;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.25, 1.20)
}

pub struct ConiferSamples {
	pub geometry: FriendsConiferSbs,
	pub apex_canopy_spawn_fraction: f32,
	pub splay_radius_fraction_of_height: f32,
}

impl BuildWithNoise<ConiferSamples> for JerrysChaparralFriendsConifer {
	fn build_with_noise(&self, noise: NoiseParams) -> ConiferSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(1.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 3.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = FriendsConiferSbs::default();
		geometry.scale.stalk_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.projection.length_fraction_of_height =
			UnitRange::new(span * 0.95, (span * 0.35).max(0.03));
		geometry.growth.branch_depth = 2;
		geometry.rings.anchors_per_ring = 3;
		geometry.canopy_noise = noise;

		ConiferSamples {
			geometry,
			apex_canopy_spawn_fraction: canopy_density.clamp(0.25, 0.55),
			splay_radius_fraction_of_height: (canopy_spread / height).clamp(0.014, 0.06),
		}
	}
}
