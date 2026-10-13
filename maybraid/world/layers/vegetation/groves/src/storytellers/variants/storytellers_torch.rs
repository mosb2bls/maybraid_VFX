//! [`BuildWithNoise`] for [`StorytellersTorch`] (Penmarch and Kamakura forms).

use procedural_common::{BuildWithNoise, NoiseConfig, NoiseParams, UnitRange};
use sbs_geometry::{KamakuraTorchSbs, PenmarchTorchSbs};

use crate::storytellers::StorytellersTorch;

fn sample_f32(config: &NoiseConfig, range: UnitRange, salt: f32) -> f32 {
	let lo = range.start.min(range.end);
	let hi = range.start.max(range.end);
	config.sample_range_f32_4d(lo, hi, 0.0, 0.0, 0.0, salt)
}

fn span_fraction(canopy_spread: f32, height: f32) -> f32 {
	(canopy_spread / height.max(0.5)).clamp(0.35, 1.20)
}

const STORYTELLERS_RING_SPACING_SCALE: f32 = 1.25;
const STORYTELLERS_ANCHORS_PER_RING: u32 = 5;

fn storytellers_ring_spacing(base: f32) -> f32 {
	base * STORYTELLERS_RING_SPACING_SCALE
}

struct TorchSamples {
	height: f32,
	stalk_radius: f32,
	span: f32,
}

fn sample_torch(torch: &StorytellersTorch, noise: NoiseParams) -> TorchSamples {
	let config = NoiseConfig::new(noise);
	let height =
		sample_f32(&config, torch.height, 1.0).max(torch.height.start.min(torch.height.end));
	let stalk_radius = sample_f32(&config, torch.stalk_radius, 1.5);
	let canopy_spread = sample_f32(&config, torch.canopy_spread, 2.0);
	let span = span_fraction(canopy_spread, height);
	TorchSamples { height, stalk_radius, span }
}

impl BuildWithNoise<PenmarchTorchSbs> for StorytellersTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> PenmarchTorchSbs {
		let s = sample_torch(self, noise);
		let mut geometry = PenmarchTorchSbs::default();
		geometry.scale.tree_height = s.height;
		geometry.scale.stalk_base_radius = Some(s.stalk_radius);
		geometry.rings.spacing = storytellers_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring = STORYTELLERS_ANCHORS_PER_RING;
		geometry.projection.span_fraction_of_height = UnitRange::new(s.span * 0.88, s.span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}

impl BuildWithNoise<KamakuraTorchSbs> for StorytellersTorch {
	fn build_with_noise(&self, noise: NoiseParams) -> KamakuraTorchSbs {
		let s = sample_torch(self, noise);
		let mut geometry = KamakuraTorchSbs::default();
		geometry.scale.tree_height = s.height;
		geometry.scale.stalk_base_radius = Some(s.stalk_radius);
		geometry.rings.spacing = storytellers_ring_spacing(geometry.rings.spacing);
		geometry.rings.anchors_per_ring = STORYTELLERS_ANCHORS_PER_RING;
		geometry.projection.span_fraction_of_height = UnitRange::new(s.span * 0.88, s.span * 1.08);
		geometry.canopy_noise = noise;
		geometry
	}
}
