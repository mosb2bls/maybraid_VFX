//! Rough-stonework joint leaf helpers.

use bevy::prelude::Transform;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::LodSceneLevel;

use crate::assets::joints::rough_stonework::{JOINT_HIGH, JOINT_MID};
use crate::assets::AssetPath;
use crate::lod_host_helper::LodHostHelper;
use crate::partitions::probe::{PartitionLodBand, PartitionLodProbe};
use crate::placed::Placement;

/// Tighter lone-joint leaf banding. Under a polyline parent, joints follow the parent level.
pub const JOINT_HIGH_FACTOR: f32 = 3.0;
pub const JOINT_MEDIUM_FACTOR: f32 = 12.0;

/// Joint mesh choice under a parent level; optional lone-leaf host.
pub struct JointLod;

impl JointLod {
	pub fn included_at(level: LodSceneLevel) -> bool {
		matches!(level, LodSceneLevel::High | LodSceneLevel::Medium)
	}

	pub fn asset_for_level(level: LodSceneLevel) -> Option<AssetPath> {
		match level {
			LodSceneLevel::High => Some(JOINT_HIGH),
			LodSceneLevel::Medium => Some(JOINT_MID),
			_ => None,
		}
	}

	pub fn band_from_distance_factor(factor: f32) -> PartitionLodBand {
		if factor <= JOINT_HIGH_FACTOR {
			PartitionLodBand::High
		} else if factor <= JOINT_MEDIUM_FACTOR {
			PartitionLodBand::Medium
		} else {
			PartitionLodBand::Low
		}
	}

	pub fn posed_tier(transform: Transform, level: LodSceneLevel) -> impl Scene + 'static {
		LodHostHelper::posed_asset_tier(Self::asset_for_level(level), transform)
	}

	/// Level from placement + viewer (tighter than the shared partition banding).
	pub fn level_for_placement(placement: &Placement, viewer: &Transform) -> LodSceneLevel {
		let probe = PartitionLodProbe::from_placement(placement);
		let factor = viewer.translation.distance(probe.center) / probe.extent.max(1e-4);
		match Self::band_from_distance_factor(factor) {
			PartitionLodBand::High => LodSceneLevel::High,
			PartitionLodBand::Medium => LodSceneLevel::Medium,
			PartitionLodBand::Low | PartitionLodBand::UltraLow => LodSceneLevel::Low,
		}
	}

	/// Level from a lone-joint [`LodRef`] (AABB probe; unit-kit previews).
	pub fn level_for_lod_ref(lod_ref: &LodRef) -> LodSceneLevel {
		let probe = PartitionLodProbe::from_aabb(lod_ref.bounds);
		let factor =
			lod_ref.current_transform.translation.distance(probe.center) / probe.extent.max(1e-4);
		match Self::band_from_distance_factor(factor) {
			PartitionLodBand::High => LodSceneLevel::High,
			PartitionLodBand::Medium => LodSceneLevel::Medium,
			PartitionLodBand::Low | PartitionLodBand::UltraLow => LodSceneLevel::Low,
		}
	}
}
