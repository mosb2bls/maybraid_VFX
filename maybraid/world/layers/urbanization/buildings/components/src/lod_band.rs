//! Shared distance / extent banding for domain LOD probes.
//!
//! Partitions and roofs each own factor constants and probe components; they share
//! this mapping from `distance / extent` onto High / Medium / Low / UltraLow.

use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::scene::{cull_bands_with_adjacent_depth, LodSceneCulls, LodSceneLevel, LodSceneStatus};

use crate::placed::Placement;

/// Despawn policy for warm High/Medium/Low mesh hosts.
///
/// Aggressive adjacent cull (`depth = 0`): drop the nearer adjacent band as soon
/// as the current band is entered. Acceptable here because warm mesh roots are
/// cheap SceneRefs; **do not copy this for heavy composite hosts** — prefer
/// [`lod::cull_non_adjacent_bands`] or [`lod::cull_offset_bands`]. Also refuses
/// Distance/Resolution customs.
pub fn warm_mesh_lod_culls(level: LodSceneLevel) -> LodSceneCulls {
	cull_bands_with_adjacent_depth(level, 1.0, 0.0).with_customs()
}

/// Like [`warm_mesh_lod_culls`], but only cull the nearer adjacent once
/// `progress_into_band` ≥ `depth` (see [`lod::cull_bands_with_adjacent_depth`]).
pub fn warm_mesh_lod_culls_at_depth(
	level: LodSceneLevel,
	progress_into_band: f32,
	depth: f32,
) -> LodSceneCulls {
	cull_bands_with_adjacent_depth(level, progress_into_band, depth).with_customs()
}

/// Four-way band from a distance factor (used by partition and roof probes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DistanceLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl DistanceLodBand {
	/// `distance / extent` → band using domain-specific thresholds.
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

	/// UltraLow and Low share the low mesh until a dedicated ultra-low exists.
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

/// AABB center and characteristic extent (max axis).
pub fn center_extent_from_aabb(aabb: &Aabb3d) -> (Vec3, f32) {
	let center = Vec3::from((aabb.min + aabb.max) * 0.5);
	let size = aabb.max - aabb.min;
	(center, size.x.max(size.y).max(size.z).max(1e-4))
}

/// Max absolute placement scale axis.
pub fn characteristic_extent_abs(placement: &Placement) -> f32 {
	placement
		.scale
		.x
		.abs()
		.max(placement.scale.y.abs())
		.max(placement.scale.z.abs())
		.max(1e-4)
}

/// Mid-height of a placed kit (translation + half Y scale).
pub fn placement_center(placement: &Placement) -> Vec3 {
	placement.translation + Vec3::new(0.0, placement.scale.y.abs() * 0.5, 0.0)
}

/// Local AABB around a placed kit ([`placement_center`] ± characteristic extent).
///
/// Coarse on purpose: this feeds [`lod::LodScene::scene_bounds`] for host indexing,
/// not culling geometry.
pub fn placement_bounds(placement: &Placement) -> Aabb3d {
	let center = placement_center(placement);
	let half = Vec3::splat(characteristic_extent_abs(placement).max(1.0));
	Aabb3d::from_min_max(center - half, center + half)
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn from_factors_thresholds() -> anyhow::Result<()> {
		assert_eq!(DistanceLodBand::from_factors(2.5, 2.5, 10.0, 500.0), DistanceLodBand::High);
		assert_eq!(DistanceLodBand::from_factors(10.0, 2.5, 10.0, 500.0), DistanceLodBand::Medium);
		assert_eq!(DistanceLodBand::from_factors(500.0, 2.5, 10.0, 500.0), DistanceLodBand::Low);
		assert_eq!(
			DistanceLodBand::from_factors(501.0, 2.5, 10.0, 500.0),
			DistanceLodBand::UltraLow
		);
		Ok(())
	}

	#[test]
	fn warm_mesh_culls_high_when_not_high() -> anyhow::Result<()> {
		let high = warm_mesh_lod_culls(LodSceneLevel::High);
		assert!(!high.should_cull(LodSceneLevel::High));
		assert!(!high.should_cull(LodSceneLevel::Medium));
		assert!(high.should_cull(LodSceneLevel::Low));
		assert!(high.should_cull(LodSceneLevel::Distance(lod::QuantizedDistance(1))));

		assert!(warm_mesh_lod_culls(LodSceneLevel::Medium).should_cull(LodSceneLevel::High));
		assert!(warm_mesh_lod_culls(LodSceneLevel::Low).should_cull(LodSceneLevel::High));
		assert!(warm_mesh_lod_culls(LodSceneLevel::Low).should_cull(LodSceneLevel::Medium));

		let early_mid = warm_mesh_lod_culls_at_depth(LodSceneLevel::Medium, 0.2, 0.5);
		assert!(!early_mid.should_cull(LodSceneLevel::High));
		let deep_mid = warm_mesh_lod_culls_at_depth(LodSceneLevel::Medium, 0.6, 0.5);
		assert!(deep_mid.should_cull(LodSceneLevel::High));
		Ok(())
	}
}
