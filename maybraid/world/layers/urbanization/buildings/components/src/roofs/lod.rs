//! Roof mesh-resolution LOD (distance / extent banding).
//!
//! Distinct from partition linear factors — roofs stay on tighter High / Medium
//! thresholds so pitched kits drop resolution sooner for the same extent.

use bevy::prelude::{Component, Query, Transform, With};
use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use scene_ref::SceneRef;

use crate::empty_scene;
use crate::lod_band::{
	center_extent_from_aabb, characteristic_extent_abs, placement_center, warm_mesh_lod_culls,
	DistanceLodBand,
};
use crate::placed::Placement;

/// `distance / max_extent` out to this → High.
pub const ROOF_HIGH_FACTOR: f32 = 2.0;
/// Out to this → Medium.
pub const ROOF_MEDIUM_FACTOR: f32 = 3.0;
/// Out to this → Low; else UltraLow.
pub const ROOF_LOW_FACTOR: f32 = 8.0;

/// Viewer distance band for roof mesh resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoofLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl RoofLodBand {
	pub fn from_distance_factor(factor: f32) -> Self {
		match DistanceLodBand::from_factors(
			factor,
			ROOF_HIGH_FACTOR,
			ROOF_MEDIUM_FACTOR,
			ROOF_LOW_FACTOR,
		) {
			DistanceLodBand::High => Self::High,
			DistanceLodBand::Medium => Self::Medium,
			DistanceLodBand::Low => Self::Low,
			DistanceLodBand::UltraLow => Self::UltraLow,
		}
	}

	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self {
			Self::High => LodSceneLevel::High,
			Self::Medium => LodSceneLevel::Medium,
			Self::UltraLow | Self::Low => LodSceneLevel::Low,
		}
	}

	pub fn status_vs(self, prev: Self) -> LodSceneStatus {
		let prev_l = prev.to_lod_scene_level();
		let curr_l = self.to_lod_scene_level();
		if prev_l == curr_l {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(curr_l)
		}
	}
}

/// Fine-phase probe for roof mesh hosts (center + characteristic extent).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct RoofLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl RoofLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self { center: placement_center(placement), extent: characteristic_extent_abs(placement) }
	}

	pub fn from_aabb(aabb: &Aabb3d) -> Self {
		let (center, extent) = center_extent_from_aabb(aabb);
		Self { center, extent }
	}

	pub fn band_for(&self, viewer: &Transform) -> RoofLodBand {
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		RoofLodBand::from_distance_factor(factor)
	}

	pub fn level_for(&self, viewer: &Transform) -> LodSceneLevel {
		self.band_for(viewer).to_lod_scene_level()
	}

	pub fn status_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.band_for(lod_ref.current_transform)
			.status_vs(self.band_for(lod_ref.previous_transform))
	}

	pub fn culls_for_lod_ref(&self, lod_ref: &LodRef) -> LodSceneCulls {
		warm_mesh_lod_culls(self.level_for(lod_ref.current_transform))
	}
}

/// Probe writes [`LodSceneLevel`]; mesh hosts fulfill via chunk sync on domain nodes.
impl LodScene for RoofLodProbe {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.status_for_lod_ref(lod_ref)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		bevy::scene::SceneFunction(empty_scene)
	}
}

/// Kit [`SceneRef`] for one [`LodSceneLevel`] (no host scaffolding).
pub(crate) fn roof_kit_scene_ref(
	high: SceneRef,
	mid: SceneRef,
	low: SceneRef,
	level: LodSceneLevel,
) -> SceneRef {
	match level {
		LodSceneLevel::High => high,
		LodSceneLevel::Medium => mid,
		LodSceneLevel::Low | LodSceneLevel::UltraLow => low,
		LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => mid,
	}
}

/// Posed roof kit content for one [`LodSceneLevel`] (no host scaffolding).
pub fn roof_scene_ref_for_level(
	high: SceneRef,
	mid: SceneRef,
	low: SceneRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	roof_kit_scene_ref(high, mid, low, level).scene()
}

/// Update roof host levels from the [`lod::LodViewer`] pose.
pub fn update_roof_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&RoofLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	for (probe, mut level) in &mut hosts {
		let desired = probe.level_for(&pose.current);
		if *level != desired {
			*level = desired;
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn roof_factors_are_tighter_than_partition_linear() -> anyhow::Result<()> {
		use crate::partitions::geometry::LINEAR_HIGH_FACTOR;

		assert_eq!(RoofLodBand::from_distance_factor(ROOF_HIGH_FACTOR), RoofLodBand::High);
		assert_eq!(RoofLodBand::from_distance_factor(ROOF_MEDIUM_FACTOR), RoofLodBand::Medium);
		assert!(ROOF_HIGH_FACTOR < LINEAR_HIGH_FACTOR);
		assert!(ROOF_MEDIUM_FACTOR < LINEAR_HIGH_FACTOR);
		// Wall High cutoff is already past roof Medium → Low for roofs.
		assert_eq!(RoofLodBand::from_distance_factor(LINEAR_HIGH_FACTOR), RoofLodBand::Low);
		assert_eq!(RoofLodBand::from_distance_factor(ROOF_LOW_FACTOR + 1.0), RoofLodBand::UltraLow);
		Ok(())
	}
}
