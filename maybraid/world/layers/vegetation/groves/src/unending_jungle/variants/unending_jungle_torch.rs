//! [`BuildWithNoise`] for [`UnendingJungleTorch`].

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::PenmarchTorchSbs;

use crate::unending_jungle::UnendingJungleTorch;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

const LOWER_CANOPY_RING_SPACING_SCALE: f32 = 1.25;
const LOWER_CANOPY_ANCHORS_PER_RING: u32 = 5;

fn lower_canopy_ring_spacing(base: f32) -> f32 {
	base * LOWER_CANOPY_RING_SPACING_SCALE
}

impl BuildWithNoise<PenmarchTorchSbs> for UnendingJungleTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> PenmarchTorchSbs {
		let config = NoiseConfig::new(noise);
		let height = sample_f32(&config, self.height, 1.0).max(2.5);
		let stalk_radius = sample_f32(&config, self.stalk_radius, 1.5);
		let canopy_spread = sample_f32(&config, self.canopy_spread, 2.0);
		let _canopy_density = sample_f32(&config, self.canopy_density, 4.0);
		let span = span_fraction(canopy_spread, height);

		let mut geometry = PenmarchTorchSbs::default();
		geometry.scale.tree_height = height;
		geometry.scale.stalk_base_radius = Some(stalk_radius);
		geometry.rings.spacing = lower_canopy_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring = LOWER_CANOPY_ANCHORS_PER_RING;
		geometry.projection.span_fraction_of_height = UnitRange::new(span * 0.88, span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}
