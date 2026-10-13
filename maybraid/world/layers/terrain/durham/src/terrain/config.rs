//! Shared terrain authoring config (seed / height scale).

use bevy::prelude::*;

/// Configuration for terrain composition and base noise.
#[derive(Resource, Clone, Debug)]
pub struct TerrainConfig {
	pub seed: u32,
	pub height_scale: f32,
}

impl TerrainConfig {
	/// Naturescapes-scale defaults (`seed=42`, `height_scale=500`).
	pub fn new(seed: u32) -> Self {
		Self { seed, height_scale: 500.0 }
	}
}
