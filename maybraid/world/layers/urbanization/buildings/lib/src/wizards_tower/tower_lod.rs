//! Shared Wizard's Tower LOD banding via footprint capsule → [`LodSceneLevel`].
//!
//! Verticality must not inflate the Low (cylinder) cutoff: use **capsule surface
//! distance in world meters** for that boundary. High stays a multiple of
//! footprint radius so wide towers open detail farther out. That can clash with
//! scale-dependent [`ParentConfines`] reveal — if internals would still be
//! eligible at that range, showing them is fine.

use bevy::prelude::{GlobalTransform, Transform};
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use building_components::distance_to_segment;
use lod::lod_ref::LodRef;
use lod::scene::LodSceneLevel;

/// High (exterior + internals) when capsule surface distance ≤ this × footprint radius.
pub const HIGH_FOOTPRINT_MULTIPLIER: f32 = 5.0;
/// Medium (exterior) while capsule surface distance ≤ this many world meters; beyond → Low cylinder.
pub const LOW_RES_CUTOFF_METERS: f32 = 400.0;

/// Footprint-derived LOD helpers for the tower host.
pub(crate) trait TowerLodFootprint {
	fn lod_aabb(&self) -> &Aabb3d;

	fn footprint_radius(&self) -> f32 {
		let aabb = self.lod_aabb();
		let extent = aabb.max - aabb.min;
		0.5 * extent.x.min(extent.z)
	}

	fn tower_height(&self) -> f32 {
		let aabb = self.lod_aabb();
		(aabb.max.y - aabb.min.y).max(1e-4)
	}

	/// Vertical capsule through the full tower AABB (medial axis + footprint radius).
	fn lod_capsule(&self) -> (Vec3, Vec3, f32) {
		let aabb = self.lod_aabb();
		let c = (aabb.min + aabb.max) * 0.5;
		let radius = self.footprint_radius().max(1e-4);
		(Vec3::new(c.x, aabb.min.y, c.z), Vec3::new(c.x, aabb.max.y, c.z), radius)
	}

	/// Meters outside the tower capsule (0 when inside / on the hull).
	fn capsule_surface_distance(&self, viewer: &Transform) -> f32 {
		let (a, b, radius) = self.lod_capsule();
		(distance_to_segment(viewer.translation, a, b) - radius).max(0.0)
	}

	#[allow(dead_code)]
	fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		self.band_progress_for(viewer).0
	}

	/// Named band and 0..=1 progress through it (capsule surface meters).
	///
	/// Low is open-ended → progress stays `0` so offset-band GC keeps Medium warm.
	fn band_progress_for(&self, viewer: &Transform) -> (LodSceneLevel, f32) {
		let dist = self.capsule_surface_distance(viewer);
		let high_cut = self.footprint_radius() * HIGH_FOOTPRINT_MULTIPLIER;
		if dist <= high_cut {
			let progress = if high_cut > 1e-4 { (dist / high_cut).clamp(0.0, 1.0) } else { 1.0 };
			(LodSceneLevel::High, progress)
		} else if dist <= LOW_RES_CUTOFF_METERS {
			let span = (LOW_RES_CUTOFF_METERS - high_cut).max(1e-4);
			(LodSceneLevel::Medium, ((dist - high_cut) / span).clamp(0.0, 1.0))
		} else {
			// Low + UltraLow both use the cylinder silhouette; no authored far edge.
			(LodSceneLevel::Low, 0.0)
		}
	}

	#[allow(dead_code)]
	fn level_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	#[allow(dead_code)]
	fn band_progress_for_lod_ref(&self, lod_ref: &LodRef) -> (LodSceneLevel, f32) {
		self.band_progress_for(lod_ref.current_transform)
	}

	fn level_for_world(&self, viewer_world: Vec3, host: &Transform) -> LodSceneLevel {
		self.band_progress_for_world(viewer_world, host).0
	}

	fn band_progress_for_world(
		&self,
		viewer_world: Vec3,
		host: &Transform,
	) -> (LodSceneLevel, f32) {
		let viewer_local =
			GlobalTransform::from(*host).affine().inverse().transform_point3(viewer_world);
		self.band_progress_for(&Transform::from_translation(viewer_local))
	}

	fn level_for_world_lod_ref(&self, lod_ref: &LodRef, host: &Transform) -> LodSceneLevel {
		self.level_for_world(lod_ref.current_transform.translation, host)
	}

	fn band_progress_for_world_lod_ref(
		&self,
		lod_ref: &LodRef,
		host: &Transform,
	) -> (LodSceneLevel, f32) {
		self.band_progress_for_world(lod_ref.current_transform.translation, host)
	}
}
