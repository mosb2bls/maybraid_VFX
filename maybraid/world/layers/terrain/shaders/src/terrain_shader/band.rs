//! One frequency band: FBM seed/scale, inter-band weight, and eight swatches.

use bevy::{prelude::*, render::render_resource::ShaderType};

use super::swatch::TerrainSwatchUniform;

/// Per-frequency band. **`config.x`** = FBM seed; **`band_scale`**: **`x`** = frequency, **`y`** = amplitude, **`z`** = blend weight vs other bands.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TerrainBandUniform {
	pub config: Vec4,
	pub band_scale: Vec4,
	pub swatches: [TerrainSwatchUniform; 8],
}

impl TerrainBandUniform {
	pub fn new(
		seed: f32,
		frequency: f32,
		amplitude: f32,
		blend_weight: f32,
		swatches: [TerrainSwatchUniform; 8],
	) -> Self {
		Self {
			config: Vec4::new(seed, 0.0, 0.0, 0.0),
			band_scale: Vec4::new(frequency, amplitude, blend_weight, 0.0),
			swatches,
		}
	}

	pub fn from_macro_scale(seed: f32, frequency: f32, amplitude: f32, blend_weight: f32) -> Self {
		Self::new(
			seed,
			frequency,
			amplitude,
			blend_weight,
			TerrainSwatchUniform::palette_macro_scale(),
		)
	}

	pub fn from_meso_high_contrast(
		seed: f32,
		frequency: f32,
		amplitude: f32,
		blend_weight: f32,
	) -> Self {
		Self::new(
			seed,
			frequency,
			amplitude,
			blend_weight,
			TerrainSwatchUniform::palette_meso_high_contrast(),
		)
	}

	pub fn from_finer_high_contrast(
		seed: f32,
		frequency: f32,
		amplitude: f32,
		blend_weight: f32,
	) -> Self {
		Self::new(
			seed,
			frequency,
			amplitude,
			blend_weight,
			TerrainSwatchUniform::palette_finer_high_contrast(),
		)
	}

	pub fn from_detail_fun(seed: f32, frequency: f32, amplitude: f32, blend_weight: f32) -> Self {
		Self::new(
			seed,
			frequency,
			amplitude,
			blend_weight,
			TerrainSwatchUniform::palette_detail_fun(),
		)
	}

	pub fn with_seed(mut self, seed: f32) -> Self {
		self.config.x = seed;
		self
	}

	pub fn with_frequency(mut self, frequency: f32) -> Self {
		self.band_scale.x = frequency;
		self
	}

	pub fn with_amplitude(mut self, amplitude: f32) -> Self {
		self.band_scale.y = amplitude;
		self
	}

	/// Weight of this band when combining with the other three bands (shader uses `max(..., 0)`).
	pub fn with_blend_weight(mut self, blend_weight: f32) -> Self {
		self.band_scale.z = blend_weight;
		self
	}

	pub fn with_swatches(mut self, swatches: [TerrainSwatchUniform; 8]) -> Self {
		self.swatches = swatches;
		self
	}
}
