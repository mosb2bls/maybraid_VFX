//! Shared distance / extent banding for vegetation LOD probes.

use bevy_math::Vec3;
use lod::scene::{cull_bands_with_adjacent_depth, LodSceneCulls, LodSceneLevel, LodSceneStatus};

use crate::placed::Placement;

/// Despawn policy for warm High/Medium/Low mesh hosts (aggressive adjacent cull).
pub fn warm_mesh_lod_culls(level: LodSceneLevel) -> LodSceneCulls {
	cull_bands_with_adjacent_depth(level, 1.0, 0.0).with_customs()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistanceLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl DistanceLodBand {
	pub fn from_factors(factor: f32, high: f32, medium: f32, low: f32) -> Self {
		if factor <= high {
			Self::High
		} else if factor <= medium {
			Self::Medium
		} else if factor <= low {
			Self::Low
		} else {
			Self::UltraLow
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

pub fn characteristic_extent_abs(placement: &Placement) -> f32 {
	placement
		.scale
		.x
		.abs()
		.max(placement.scale.y.abs())
		.max(placement.scale.z.abs())
		.max(1e-4)
}

/// Mid-height of a placed stick kit (base + half length).
pub fn placement_center(placement: &Placement) -> Vec3 {
	placement.translation
		+ placement.rotation() * Vec3::new(0.0, placement.scale.y.abs() * 0.5, 0.0)
}
