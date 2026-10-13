//! Streamed vs pinned region helpers presentation and padding share.

use bevy::math::bounding::Aabb3d;
use terrain_layer_model::TerrainExtent;

/// Visual region for padding. Streamed extents use the presentation ring; a
/// pinned patch uses the scheme region or the extent.
pub fn urbanization_visual_region<M: Send + Sync + 'static>(
	extent: &TerrainExtent<M>,
	layer_region: Option<Aabb3d>,
) -> Option<Aabb3d> {
	if extent.is_streamed() {
		Some(extent.presentation_region())
	} else {
		Some(layer_region.unwrap_or_else(|| extent.presentation_region()))
	}
}

/// Host region: the scheme's write, or the extent on a pinned patch.
pub fn urbanization_host_region<M: Send + Sync + 'static>(
	extent: &TerrainExtent<M>,
	layer_region: Option<Aabb3d>,
) -> Option<Aabb3d> {
	if extent.is_streamed() {
		layer_region
	} else {
		Some(layer_region.unwrap_or_else(|| extent.presentation_region()))
	}
}
