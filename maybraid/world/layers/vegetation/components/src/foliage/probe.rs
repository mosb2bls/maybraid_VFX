//! Distance / extent probe for foliage LOD hosts.

use bevy::prelude::{Component, Query, Transform, With};
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodSceneLevel, LodSceneStatus};

use crate::foliage::collection::{
	CheapBallCollection, FrondCollection, CHEAP_BALL_COLLECTION_HIGH_METERS,
	CHEAP_BALL_COLLECTION_LOW_METERS, CHEAP_BALL_COLLECTION_MEDIUM_METERS,
	FROND_COLLECTION_HIGH_FACTOR, FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_MEDIUM_FACTOR,
};
use crate::lod_band::{characteristic_extent_abs, DistanceLodBand};
use crate::placed::Placement;

pub const FOLIAGE_HIGH_FACTOR: f32 = 5.0;
pub const FOLIAGE_MEDIUM_FACTOR: f32 = 20.0;
pub const FOLIAGE_LOW_FACTOR: f32 = 500.0;

/// Fine-phase probe for foliage / frond-collection hosts (center + characteristic extent).
///
/// Ball / single-kit foliage uses [`FOLIAGE_*`](FOLIAGE_HIGH_FACTOR) **extent-relative**
/// factors and collapses UltraLow onto Low. Frond collections use
/// [`FROND_COLLECTION_*_FACTOR`](crate::foliage::collection::FROND_COLLECTION_HIGH_FACTOR)
/// (also extent-relative: `distance / radius`) with a real UltraLow tier
/// (`preserve_ultra_low`). Warm-root cull for collections uses absolute meters separately
/// in [`crate::FoliageNode::scene_lod_culls`].
#[derive(Debug, Clone, Copy, Component)]
pub struct FoliageLodProbe {
	pub center: Vec3,
	pub extent: f32,
	pub high_factor: f32,
	pub medium_factor: f32,
	pub low_factor: f32,
	/// When true, band UltraLow maps to [`LodSceneLevel::UltraLow`] (frond collections).
	pub preserve_ultra_low: bool,
}

impl Default for FoliageLodProbe {
	fn default() -> Self {
		Self {
			center: Vec3::ZERO,
			extent: 1.0,
			high_factor: FOLIAGE_HIGH_FACTOR,
			medium_factor: FOLIAGE_MEDIUM_FACTOR,
			low_factor: FOLIAGE_LOW_FACTOR,
			preserve_ultra_low: false,
		}
	}
}

impl FoliageLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self {
			center: placement.translation,
			extent: characteristic_extent_abs(placement),
			..Self::default()
		}
	}

	/// Probe for a frond collection: authored center/radius + radius-relative spawn bands.
	///
	/// Band constants: [`FROND_COLLECTION_HIGH_FACTOR`], [`FROND_COLLECTION_MEDIUM_FACTOR`],
	/// [`FROND_COLLECTION_LOW_FACTOR`] in [`crate::foliage::collection`].
	pub fn for_frond_collection(collection: &FrondCollection) -> Self {
		let (center, extent) = collection.center_and_extent();
		Self::for_kit_collection(center, extent)
	}

	/// Shared spawn bands for frond collections (`distance / radius`).
	pub fn for_kit_collection(center: Vec3, radius: f32) -> Self {
		Self {
			center,
			extent: radius.max(1e-4),
			high_factor: FROND_COLLECTION_HIGH_FACTOR,
			medium_factor: FROND_COLLECTION_MEDIUM_FACTOR,
			low_factor: FROND_COLLECTION_LOW_FACTOR,
			preserve_ultra_low: true,
		}
	}

	/// Cheap-ball collection probe: High/Medium/Low edges are **absolute meters**
	/// ([`CHEAP_BALL_COLLECTION_HIGH_METERS`], …) encoded as `meters / radius`.
	///
	/// A tight baked canopy AABB (~2–4 m) must not use frond `5×/10×/50×` spawn
	/// factors — that drops the host to UltraLow (one ball) a few dozen meters out
	/// and looks like a flicker-then-gone merge.
	pub fn for_cheap_ball_collection(collection: &CheapBallCollection) -> Self {
		let (center, radius) = collection.center_and_extent();
		Self::for_cheap_ball_probe(center, radius)
	}

	pub fn for_cheap_ball_probe(center: Vec3, radius: f32) -> Self {
		let extent = radius.max(1e-4);
		Self {
			center,
			extent,
			high_factor: (CHEAP_BALL_COLLECTION_HIGH_METERS / extent).max(1.0),
			medium_factor: (CHEAP_BALL_COLLECTION_MEDIUM_METERS / extent).max(1.0),
			low_factor: (CHEAP_BALL_COLLECTION_LOW_METERS / extent).max(1.0),
			preserve_ultra_low: true,
		}
	}

	fn band_to_level(self, band: DistanceLodBand) -> LodSceneLevel {
		match band {
			DistanceLodBand::High => LodSceneLevel::High,
			DistanceLodBand::Medium => LodSceneLevel::Medium,
			DistanceLodBand::Low => LodSceneLevel::Low,
			DistanceLodBand::UltraLow if self.preserve_ultra_low => LodSceneLevel::UltraLow,
			DistanceLodBand::UltraLow => LodSceneLevel::Low,
		}
	}

	/// Distance / extent — same metric as spawn/presentation level bands.
	pub fn band_metric(self, viewer_translation: Vec3) -> f32 {
		let distance = viewer_translation.distance(self.center);
		distance / self.extent.max(1e-4)
	}

	pub fn level_for(self, viewer: &Transform) -> LodSceneLevel {
		self.band_to_level(DistanceLodBand::from_factors(
			self.band_metric(viewer.translation),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		))
	}

	pub fn status_for_lod_ref(self, lod_ref: &LodRef) -> LodSceneStatus {
		let prev = self.band_to_level(DistanceLodBand::from_factors(
			self.band_metric(lod_ref.previous_transform.translation),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		let curr = self.band_to_level(DistanceLodBand::from_factors(
			self.band_metric(lod_ref.current_transform.translation),
			self.high_factor,
			self.medium_factor,
			self.low_factor,
		));
		if prev == curr {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr)
		}
	}
}

pub fn update_foliage_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&FoliageLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	let viewer = pose.current;
	for (probe, mut level) in &mut hosts {
		let next = probe.level_for(&viewer);
		if *level != next {
			*level = next;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::foliage::collection::{
		FrondCollection, FrondMember, FrondRun, FROND_COLLECTION_HIGH_FACTOR,
		FROND_COLLECTION_LOW_FACTOR, FROND_COLLECTION_MEDIUM_FACTOR,
	};
	use crate::placed::Placement;

	#[test]
	fn frond_collection_probe_uses_radius_relative_spawn_bands() {
		let collection = FrondCollection::new([FrondRun::new([FrondMember::segment(
			Placement::frond_segment(Vec3::ZERO, Vec3::Y, 1.0, 0.02).expect("placement"),
		)])])
		.with_probe(Vec3::ZERO, 2.0);
		let probe = FoliageLodProbe::for_frond_collection(&collection);
		assert!((probe.extent - 2.0).abs() < 1e-5);
		assert!((probe.high_factor - FROND_COLLECTION_HIGH_FACTOR).abs() < 1e-5);
		assert!((probe.medium_factor - FROND_COLLECTION_MEDIUM_FACTOR).abs() < 1e-5);
		assert!((probe.low_factor - FROND_COLLECTION_LOW_FACTOR).abs() < 1e-5);
		assert!(probe.preserve_ultra_low);

		// radius 2 → High while distance ≤ 10 (5×), Medium ≤ 20, Low ≤ 50.
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(9.0, 0.0, 0.0))),
			LodSceneLevel::High
		);
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(15.0, 0.0, 0.0))),
			LodSceneLevel::Medium
		);
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(40.0, 0.0, 0.0))),
			LodSceneLevel::Low
		);
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(110.0, 0.0, 0.0))),
			LodSceneLevel::UltraLow
		);
		// Absolute meters no longer drive spawn: 100 m is still UltraLow for radius 2.
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(1_000.0, 0.0, 0.0))),
			LodSceneLevel::UltraLow
		);
	}

	#[test]
	fn cheap_ball_collection_stays_high_across_a_grove() {
		use crate::placed::Placement;

		let collection = CheapBallCollection::new([
			Placement::foliage_uniform(Vec3::new(0.0, 4.0, 0.0), 0.3),
			Placement::foliage_uniform(Vec3::new(2.0, 5.0, 1.0), 0.3),
		])
		.bake_bounds_from_placements();
		let probe = FoliageLodProbe::for_cheap_ball_collection(&collection);
		assert!(probe.extent < 8.0, "baked canopy should stay tree-scale, got {}", probe.extent);
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(80.0, 2.0, 0.0))),
			LodSceneLevel::High
		);
		assert_eq!(
			probe.level_for(&Transform::from_translation(Vec3::new(600.0, 2.0, 0.0))),
			LodSceneLevel::Medium
		);
	}
}
