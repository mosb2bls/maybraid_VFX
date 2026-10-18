//! Distance / extent probe shared by partition LOD hosts.

use bevy::prelude::{Component, Query, Transform, With};
use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};

use crate::empty_scene;
use crate::lod_band::{
	center_extent_from_aabb, characteristic_extent_abs, placement_center, warm_mesh_lod_culls,
	DistanceLodBand,
};
use crate::partitions::geometry::{LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR};
use crate::placed::Placement;

/// Viewer distance band for partition mesh resolution (linear / polyline parent).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartitionLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl PartitionLodBand {
	pub fn from_distance_factor(factor: f32) -> Self {
		match DistanceLodBand::from_factors(
			factor,
			LINEAR_HIGH_FACTOR,
			LINEAR_MEDIUM_FACTOR,
			LINEAR_LOW_FACTOR,
		) {
			DistanceLodBand::High => Self::High,
			DistanceLodBand::Medium => Self::Medium,
			DistanceLodBand::Low => Self::Low,
			DistanceLodBand::UltraLow => Self::UltraLow,
		}
	}

	pub fn mesh_tier(self) -> crate::partitions::mesh_set::PartitionMeshTier {
		use crate::partitions::mesh_set::PartitionMeshTier;
		match self {
			Self::UltraLow | Self::Low => PartitionMeshTier::Low,
			Self::Medium => PartitionMeshTier::Mid,
			Self::High => PartitionMeshTier::High,
		}
	}

	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self.mesh_tier() {
			crate::partitions::mesh_set::PartitionMeshTier::High => LodSceneLevel::High,
			crate::partitions::mesh_set::PartitionMeshTier::Mid => LodSceneLevel::Medium,
			crate::partitions::mesh_set::PartitionMeshTier::Low => LodSceneLevel::Low,
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

/// Fine-phase probe for partition mesh hosts (center + characteristic extent).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct PartitionLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl PartitionLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self { center: placement_center(placement), extent: characteristic_extent_abs(placement) }
	}

	pub fn from_aabb(aabb: &Aabb3d) -> Self {
		let (center, extent) = center_extent_from_aabb(aabb);
		Self { center, extent }
	}

	pub fn characteristic_extent(placement: &Placement) -> f32 {
		characteristic_extent_abs(placement)
	}

	pub fn placement_center(placement: &Placement) -> Vec3 {
		placement_center(placement)
	}

	pub fn band_for(&self, viewer: &Transform) -> PartitionLodBand {
		let factor = viewer.translation.distance(self.center) / self.extent.max(1e-4);
		PartitionLodBand::from_distance_factor(factor)
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
impl LodScene for PartitionLodProbe {
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

impl Placement {
	pub fn partition_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		PartitionLodProbe::from_placement(self).level_for(lod_ref.current_transform)
	}

	pub fn partition_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		PartitionLodProbe::from_placement(self).status_for_lod_ref(lod_ref)
	}
}

/// Update partition host levels from the [`lod::LodViewer`] pose.
pub fn update_partition_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&PartitionLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
) {
	let Ok(pose) = viewer.single() else {
		return;
	};
	let t0 = std::time::Instant::now();
	let mut n = 0u32;
	let mut changed = 0u32;
	for (probe, mut level) in &mut hosts {
		n += 1;
		let desired = probe.level_for(&pose.current);
		if *level != desired {
			*level = desired;
			changed += 1;
		}
	}
	let elapsed_ms = t0.elapsed().as_secs_f64() * 1000.0;
	if changed > 0 || elapsed_ms >= 0.5 {
		bevy::log::info!(
			"[lod.partition] update_partition_host_levels: hosts={n} changed={changed} in {elapsed_ms:.2}ms"
		);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn distance_factor_maps_to_bands() -> anyhow::Result<()> {
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_HIGH_FACTOR),
			PartitionLodBand::High
		);
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_MEDIUM_FACTOR),
			PartitionLodBand::Medium
		);
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_LOW_FACTOR),
			PartitionLodBand::Low
		);
		assert_eq!(
			PartitionLodBand::from_distance_factor(LINEAR_LOW_FACTOR + 1.0),
			PartitionLodBand::UltraLow
		);
		Ok(())
	}
}
