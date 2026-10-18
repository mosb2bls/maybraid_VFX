//! Terrain shaders: reusable Bevy materials and embedded WGSL for Maybraid terrain work.

mod refraction_water;
mod terrain_shader;

pub use refraction_water::{RefractionWater, RefractionWaterPlugin};
pub use terrain_shader::{
	TerrainBandUniform, TerrainGrassUniform, TerrainNoiseUniform, TerrainShader,
	TerrainShaderPlugin, TerrainSwatchUniform, EVEN_BAND_BLEND_WEIGHT, EVEN_SWATCH_FOLD_WEIGHT,
};
