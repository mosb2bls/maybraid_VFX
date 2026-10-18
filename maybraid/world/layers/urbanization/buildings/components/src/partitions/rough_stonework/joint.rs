//! Circular joint between partition segments (\(X,Z \in [-0.5, 0.5]\), \(Y \in [0, 1]\)).
//!
//! High + mid GLBs only; low / ultra-low LOD omit this filler.

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{Component, Transform};
use lod::SceneChunk;

use crate::partitions::geometry::JointLod;
use crate::partitions::probe::PartitionLodProbe;

/// Circular / post joint between upright linear partition segments.
#[derive(Debug, Clone, Copy, PartialEq, Default, Component)]
pub struct RoughStoneworkJoint;

impl lod::scene::LodScene for RoughStoneworkJoint {
	fn scene_lod_level(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::scene::LodSceneLevel {
		JointLod::level_for_lod_ref(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &lod::lod_ref::LodRef) -> lod::scene::LodSceneStatus {
		let probe = PartitionLodProbe::from_aabb(lod_ref.bounds);
		let prev_factor =
			lod_ref.previous_transform.translation.distance(probe.center) / probe.extent.max(1e-4);
		let curr_factor =
			lod_ref.current_transform.translation.distance(probe.center) / probe.extent.max(1e-4);
		let prev = JointLod::band_from_distance_factor(prev_factor);
		let curr = JointLod::band_from_distance_factor(curr_factor);
		if prev == curr {
			lod::scene::LodSceneStatus::Unchanged
		} else {
			lod::scene::LodSceneStatus::Changed(JointLod::level_for_lod_ref(lod_ref))
		}
	}

	fn scene_lod_culls(
		&self,
		_lod_ref: &lod::lod_ref::LodRef,
		current: lod::scene::LodSceneLevel,
	) -> lod::scene::LodSceneCulls {
		crate::lod_band::warm_mesh_lod_culls(current)
	}

	fn scene_with_level(
		&self,
		_lod_ref: &lod::lod_ref::LodRef,
		level: lod::scene::LodSceneLevel,
	) -> impl bevy::scene::Scene + 'static {
		JointLod::posed_tier(Transform::IDENTITY, level)
	}

	fn scene_chunks_with_level(
		&self,
		lod_ref: &lod::lod_ref::LodRef,
		level: lod::scene::LodSceneLevel,
	) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(-0.5, 0.0, -0.5), Vec3::new(0.5, 1.0, 0.5))
	}
}
