//! Shared stream-band LOD: High is the inner hole, Medium is the drawn ring.
//!
//! Near (`high_inner_radius == 0`) has no hole, so High is the mesh. Far and
//! background use High as an inner-disk hole and Medium as the visible
//! annulus. Playable rings inset that hole so Medium overlaps the next-finer
//! High rim (snap + half-tile). Presenters filter wanted ids with
//! [`stream_banded_draws`]; they must not encode “don’t draw” as an empty scene.

use crate::terrain::cell::TerrainCellRing;
use bevy::math::Vec3;
use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use lod::LodSceneLevel;

/// A terrain (or overlay) cell that can follow a moving stream ring.
pub trait StreamBandedLod {
	fn stream_ring(&self) -> Option<TerrainCellRing>;
	fn stream_center(&self) -> Vec3;
}

/// Band for `item` relative to `viewer`. Unbanded cells are High.
pub fn stream_banded_level(item: &impl StreamBandedLod, viewer: &Transform) -> LodSceneLevel {
	match item.stream_ring() {
		Some(ring) => ring.level_for(item.stream_center(), viewer.translation),
		None => LodSceneLevel::High,
	}
}

/// Whether this band draws a mesh at `level`. Unbanded cells always draw.
pub fn stream_banded_draws(item: &impl StreamBandedLod, level: LodSceneLevel) -> bool {
	match item.stream_ring() {
		Some(ring) => ring.draws_level(level),
		None => true,
	}
}

/// Mesh when the band draws at `level`, otherwise an empty scene.
///
/// Prefer [`stream_banded_draws`] on the presenter's wanted set. This helper
/// remains for leftover FinePatch `LodScene` impls that still branch here.
pub fn stream_banded_scene<S: Scene + 'static>(
	item: &impl StreamBandedLod,
	level: LodSceneLevel,
	mesh: impl FnOnce() -> S,
) -> Box<dyn Scene> {
	if stream_banded_draws(item, level) {
		Box::new(mesh())
	} else {
		Box::new(())
	}
}
