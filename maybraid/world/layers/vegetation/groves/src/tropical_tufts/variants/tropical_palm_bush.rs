//! [`BuildWithNoise`] for [`TropicalPalmBush`].

use procedural_common::{BuildWithNoise, NoiseParams};
use sbs_geometry::PalmBushSbs;

use crate::tropical_tufts::TropicalPalmBush;

impl BuildWithNoise<PalmBushSbs> for TropicalPalmBush {
	fn build_with_noise(&self, noise: NoiseParams) -> PalmBushSbs {
		// TODO: sample the authored `height` / `frond_count` / `frond_length` /
		// `crown_spread` ranges from `noise` instead of fixed companion values.
		PalmBushSbs::default()
			.with_height(2.4)
			.with_frond_world_scale(0.6)
			.with_noise_params(noise)
	}
}
