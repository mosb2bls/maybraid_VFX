//! Per-layer Bucket Throw ([RFC-183 §3.5.2.3]).

use bevy_math::Vec3;
use procedural_common::{BucketThrow, NoiseConfig, NoiseParams};

use crate::{ForestGroveKind, ForestLayering, SelectedLayers, WeightedGrove};

const TUFTS_LANE: Vec3 = Vec3::new(41.0, 0.0, 0.0);
const UNDERSTORY_LANE: Vec3 = Vec3::new(43.0, 0.0, 0.0);
const LOWER_LANE: Vec3 = Vec3::new(47.0, 0.0, 0.0);
const UPPER_LANE: Vec3 = Vec3::new(53.0, 0.0, 0.0);

/// Throw one layer distribution at the forest-cell center.
pub fn throw_layer(
	buckets: &[WeightedGrove],
	noise: NoiseParams,
	position: Vec3,
	lane: Vec3,
) -> Option<ForestGroveKind> {
	if buckets.is_empty() {
		return None;
	}
	let throw = BucketThrow::from_weights(buckets.iter().map(|b| b.weight), 0.0);
	let n = NoiseConfig::new(noise);
	let sample =
		n.sample_3d(position + lane + crate::hopscotch::SAMPLE_ORIGIN) * throw.total_weight();
	let index = throw.select(sample)?;
	buckets.get(index).and_then(|b| b.kind)
}

/// Select one grove (or `None`) for each layer of `layering`.
pub fn select_layers(
	layering: &ForestLayering,
	noise: NoiseParams,
	position: Vec3,
) -> SelectedLayers {
	SelectedLayers {
		layering: layering.kind,
		tufts: throw_layer(&layering.tufts, noise, position, TUFTS_LANE),
		understory: throw_layer(&layering.understory, noise, position, UNDERSTORY_LANE),
		lower_canopy: throw_layer(&layering.lower_canopy, noise, position, LOWER_LANE),
		upper_canopy: throw_layer(&layering.upper_canopy, noise, position, UPPER_LANE),
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::layerings;
	use anyhow::Result;

	#[test]
	fn ag_town_layer_throw_is_deterministic() -> Result<()> {
		let layering = layerings::ag_town();
		let noise = NoiseParams::from_scalar(2.0, 0.01, 1.0, 1);
		let at = Vec3::new(100.0, 0.0, 40.0);
		assert_eq!(select_layers(&layering, noise, at), select_layers(&layering, noise, at));
		Ok(())
	}
}
