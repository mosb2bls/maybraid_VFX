//! [`LodScene`] for [`FoliageNode`](super::FoliageNode).
//!
//! Band + cull come from [`FoliageLodProbe`]. Scene content is
//! [`super::present`] — one chunk per node, whether the collection merged or
//! instanced its kits.

use bevy::math::bounding::Aabb3d;
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::lod_ref::LodRef;
use lod::scene::{
	cull_offset_bands_from_factor, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus,
};
use lod::SceneChunk;

use crate::foliage::collection::{
	CHEAP_BALL_COLLECTION_HIGH_METERS, CHEAP_BALL_COLLECTION_LOW_METERS,
	CHEAP_BALL_COLLECTION_MEDIUM_METERS, FROND_COLLECTION_HIGH_METERS, FROND_COLLECTION_LOW_METERS,
	FROND_COLLECTION_MEDIUM_METERS,
};
use crate::foliage::geometry::FoliageGeometry;

use super::FoliageNode;

impl LodScene for FoliageNode {
	fn host_contents(&self, _lod_ref: &LodRef) -> impl Scene + 'static {
		let host = self.clone();
		let probe = self.probe();
		bsn! {
			template_value(host)
			template_value(probe)
		}
	}

	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		if self.geometry.is_kit_collection() {
			let probe = self.probe();
			let distance = lod_ref.current_transform.translation.distance(probe.center);
			let (high_m, mid_m, low_m) = match &self.geometry {
				FoliageGeometry::CheapBallCollection(_) => (
					CHEAP_BALL_COLLECTION_HIGH_METERS,
					CHEAP_BALL_COLLECTION_MEDIUM_METERS,
					CHEAP_BALL_COLLECTION_LOW_METERS,
				),
				_ => (
					FROND_COLLECTION_HIGH_METERS,
					FROND_COLLECTION_MEDIUM_METERS,
					FROND_COLLECTION_LOW_METERS,
				),
			};
			// Keep all warm roots while still inside the High cull band (~500 m).
			// `cull_offset_bands_from_factor` alone still lists Low/UltraLow in High,
			// which would defeat the `LodSceneCulls::None` short-circuit in cull enqueue.
			if distance <= high_m {
				return LodSceneCulls::None;
			}
			return cull_offset_bands_from_factor(distance, high_m, mid_m, low_m);
		}

		let probe = self.probe();
		let factor = probe.band_metric(lod_ref.current_transform.translation);
		if factor <= probe.high_factor {
			return LodSceneCulls::None;
		}
		cull_offset_bands_from_factor(
			factor,
			probe.high_factor,
			probe.medium_factor,
			probe.low_factor,
		)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		// One chunk per node so fulfill admits the host as a unit. Instanced
		// members stay siblings inside that chunk; they are not per-leaf hosts.
		SceneChunk::primitive(self.content_for_level(level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		let (center, extent) = match &self.geometry {
			FoliageGeometry::FrondCollection(_) | FoliageGeometry::CheapBallCollection(_) => {
				let probe = self.probe();
				(probe.center, probe.extent.max(1.0))
			}
			_ => (
				crate::lod_band::placement_center(&self.placement),
				crate::lod_band::characteristic_extent_abs(&self.placement).max(1.0),
			),
		};
		let half = bevy::math::Vec3::splat(extent);
		Aabb3d::from_min_max(center - half, center + half)
	}
}
