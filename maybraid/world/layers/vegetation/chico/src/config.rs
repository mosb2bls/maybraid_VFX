//! Stream knobs for Chico forest selection.

use crate::extent::DEFAULT_FOREST_GROVE_TILE_XZ;
use crate::generation::{GROVE_GENERATE_RADIUS_M, GROVE_PRESENT_RADIUS_M};
use crate::kind::LayeringKind;
use crate::{select_cell, ForestExtent, SelectedLayers};
use bevy::prelude::*;
use procedural_common::NoiseParams;

/// Default present ring multiplier (`1` → 1 km present / 3 km generate).
pub const DEFAULT_FOREST_STREAM_RADIUS: u32 = 1;

/// Hopscotch default so neighboring 1600 m cells stay related.
pub const DEFAULT_FOREST_NOISE: &str = "1337,0.0005,1,1";

/// Present / generate metric radii for a stream-radius multiplier.
pub fn stream_radii_m(stream_radius: u32) -> (f32, f32) {
	if stream_radius == 0 {
		return (DEFAULT_FOREST_GROVE_TILE_XZ, DEFAULT_FOREST_GROVE_TILE_XZ * 2.0);
	}
	let present = GROVE_PRESENT_RADIUS_M * stream_radius as f32;
	(present, present + (GROVE_GENERATE_RADIUS_M - GROVE_PRESENT_RADIUS_M))
}

/// Live forest-stream knobs (noise / ring / pinned layering).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ForestStreamSpec {
	pub noise: NoiseParams,
	pub stream_radius: u32,
	pub layering: Option<LayeringKind>,
}

impl Default for ForestStreamSpec {
	fn default() -> Self {
		Self {
			noise: NoiseParams {
				seed: 1337,
				frequency: 0.0005,
				amplitude: 1.0,
				octaves: 1,
				..default()
			},
			stream_radius: DEFAULT_FOREST_STREAM_RADIUS,
			layering: None,
		}
	}
}

impl ForestStreamSpec {
	pub fn key(self) -> String {
		let layering_key = self.layering.map(LayeringKind::as_kebab).unwrap_or("hopscotch");
		format!("forest:{layering_key}|{:?}|r={}", self.noise, self.stream_radius)
	}
}

/// How a session selects each forest cell's layers: Hopscotch on `noise`, or
/// a pinned layering's typical groves.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct ForestSelection {
	pub noise: NoiseParams,
	pub layering: Option<LayeringKind>,
}

impl Default for ForestSelection {
	fn default() -> Self {
		ForestStreamSpec::default().into()
	}
}

impl From<ForestStreamSpec> for ForestSelection {
	fn from(spec: ForestStreamSpec) -> Self {
		Self { noise: spec.noise, layering: spec.layering }
	}
}

impl ForestSelection {
	pub fn layers_for(self, extent: ForestExtent) -> SelectedLayers {
		match self.layering {
			Some(kind) => kind.layering().typical_layers(),
			None => select_cell(extent, self.noise),
		}
	}
}

/// Forest stream spec for [`ChicoRoots`].
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct ChicoConfig {
	pub forest: Option<ForestStreamSpec>,
}

impl Default for ChicoConfig {
	fn default() -> Self {
		Self { forest: None }
	}
}

impl ChicoConfig {
	/// Forest on at radius 1 (1 km present / 3 km generate).
	pub fn world_defaults() -> Self {
		Self { forest: Some(ForestStreamSpec { stream_radius: 1, ..ForestStreamSpec::default() }) }
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use procedural_common::noise_params_from_scalar_str;

	#[test]
	fn world_defaults_keep_grove_fill_at_one_kilometre() -> anyhow::Result<()> {
		let spec = ChicoConfig::world_defaults()
			.forest
			.ok_or_else(|| anyhow::anyhow!("forest on"))?;
		assert_eq!(spec.stream_radius, 1);
		assert_eq!(stream_radii_m(1), (1_000.0, 3_000.0));
		Ok(())
	}

	#[test]
	fn default_forest_noise_parses() -> anyhow::Result<()> {
		let noise = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		assert_eq!(noise.seed, 1337);
		assert!((noise.frequency - 0.0005).abs() < 1e-8);
		Ok(())
	}

	#[test]
	fn default_spec_matches_noise_string() -> anyhow::Result<()> {
		let parsed = noise_params_from_scalar_str(DEFAULT_FOREST_NOISE)
			.map_err(|e| anyhow::anyhow!("{e}"))?;
		let spec = ForestStreamSpec::default();
		assert_eq!(spec.noise.seed, parsed.seed);
		assert!((spec.noise.frequency - parsed.frequency).abs() < 1e-8);
		assert_eq!(spec.stream_radius, DEFAULT_FOREST_STREAM_RADIUS);
		Ok(())
	}
}
