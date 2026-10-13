//! Full `terrain_noise` uniform: regional blend driver + four bands.

use bevy::{prelude::*, render::render_resource::ShaderType};

use super::band::TerrainBandUniform;

/// Equal contribution from each frequency band when combining (shader uses this as `band_scale.z`).
pub const EVEN_BAND_BLEND_WEIGHT: f32 = 0.25;

/// **`regional_blend`**: **`x`** / **`y`** reserved for future regional warp driving (**WGSL** unused today); **`zw`** unused.
///
/// **`global_seed`**: **`x`** = material-wide seed (reserved for future use in WGSL paths); **`yzw`** reserved.
#[derive(Clone, Copy, Debug, ShaderType)]
pub struct TerrainNoiseUniform {
	pub regional_blend: Vec4,
	pub global_seed: Vec4,
	pub bands: [TerrainBandUniform; 4],
}

impl TerrainNoiseUniform {
	/// Noise seed used for the broadest regional warp phase (**`global_seed.x`** / WGSL **`master_seed`**).
	#[inline]
	pub fn seed(&self) -> f32 {
		self.global_seed.x
	}

	/// Sets **`global_seed.x`**; leaves per-band **`config.x`** unchanged.
	#[inline]
	pub fn set_global_seed(&mut self, seed: f32) {
		self.global_seed.x = seed;
	}

	/// Sets **`global_seed.x`**; leaves per-band **`config.x`** unchanged.
	#[inline]
	pub fn with_global_seed(mut self, seed: f32) -> Self {
		self.global_seed.x = seed;
		self
	}

	/// Sets **`global_seed.x`** and copies **`seed`** into every band’s FBM seed (**`config.x`**).
	pub fn with_seed_uniform_across_bands(mut self, seed: f32) -> Self {
		self.global_seed.x = seed;
		for band in &mut self.bands {
			band.config.x = seed;
		}
		self
	}

	/// Updates only band **`index`** **`config.x`**, leaving **`global_seed`** unchanged.
	pub fn with_band_seed(mut self, index: usize, seed: f32) -> Self {
		if index < 4 {
			self.bands[index].config.x = seed;
		}
		self
	}

	pub fn with_regional_blend_frequency(mut self, frequency: f32) -> Self {
		self.regional_blend.x = frequency;
		self
	}

	pub fn with_regional_blend_amplitude(mut self, amplitude: f32) -> Self {
		self.regional_blend.y = amplitude;
		self
	}

	pub fn with_band(mut self, index: usize, band: TerrainBandUniform) -> Self {
		if index < 4 {
			self.bands[index] = band;
		}
		self
	}
}

impl Default for TerrainNoiseUniform {
	fn default() -> Self {
		Self {
			regional_blend: Vec4::new(0.00015, 0.5, 0.0, 0.0),
			global_seed: Vec4::new(42.0, 0.0, 0.0, 0.0),
			bands: [
				TerrainBandUniform::from_macro_scale(120_079.0, 0.00001, 0.5, 0.25),
				TerrainBandUniform::from_meso_high_contrast(42.0, 0.0001, 0.5, 0.45),
				TerrainBandUniform::from_finer_high_contrast(42.0, 0.01, 0.5, 0.15),
				TerrainBandUniform::from_detail_fun(42.0, 0.1, 0.5, 0.15),
			],
		}
	}
}
