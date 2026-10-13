//! [`BuildWithNoise`] for [`JungleLowerMassivesJungleStorybook`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::sbs::jungle_storybook_tree::{
	JUNGLE_ANCHORS_PER_RING, JUNGLE_LEAF_RADIUS_FRACTION, JUNGLE_STALK_BASE_RADIUS_FRACTION,
};
use sbs_geometry::JungleStorybookTreeSbs;

use crate::jungle_lower_massives::JungleLowerMassivesJungleStorybook;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

pub struct JungleStorybookSamples {
	pub geometry: JungleStorybookTreeSbs,
	pub growth_spawn_fraction: f32,
}

impl BuildWithNoise<JungleStorybookSamples> for JungleLowerMassivesJungleStorybook {
	fn build_with_noise(&self, noise: NoiseParams) -> JungleStorybookSamples {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(10.0);
		let canopy_density = sample_f32(&config, self.canopy_density, 2.0);
		let growth_spawn_fraction = sample_f32(&config, self.jungle_growth_density, 3.0);

		let mut geometry = JungleStorybookTreeSbs::default();
		geometry.apply_jungle_preset();
		geometry.storybook.scale.tree_height = height;
		geometry.storybook.scale.stalk_base_radius =
			Some(JUNGLE_STALK_BASE_RADIUS_FRACTION * height);
		geometry.storybook.rings.anchors_per_ring =
			JUNGLE_ANCHORS_PER_RING + (canopy_density * 2.0).round() as u32;
		geometry.storybook.canopy.leaf_radius_fraction =
			JUNGLE_LEAF_RADIUS_FRACTION * (0.85 + canopy_density * 0.25);
		geometry.storybook.canopy_noise = noise;

		JungleStorybookSamples { geometry, growth_spawn_fraction }
	}
}
