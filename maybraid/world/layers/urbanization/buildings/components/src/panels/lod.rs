//! Panel mesh-resolution LOD (distance / extent banding).
//!
//! Unlike roofs, panel High / Medium hold for tens of meters so walking a storey
//! does not thrash kit resolution. UltraLow remains a distinct flat low-res kit.

use bevy::prelude::{Component, Query, Transform, With};
use bevy::scene::prelude::Scene;
use bevy_math::bounding::Aabb3d;
use bevy_math::Vec3;
use lod::lod_ref::LodRef;
use lod::lod_scene_host::LodSceneHost;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use scene_ref::SceneRef;

use crate::assets::panels::flat;
use crate::assets::AssetPath;
use crate::empty_scene;
use crate::lod_band::{
	center_extent_from_aabb, characteristic_extent_abs, placement_center, warm_mesh_lod_culls,
	DistanceLodBand,
};
use crate::placed::Placement;

/// `distance / max_extent` out to this → High (also see [`PANEL_HIGH_METERS`]).
pub const PANEL_HIGH_FACTOR: f32 = 20.0;
/// Out to this → Medium.
pub const PANEL_MEDIUM_FACTOR: f32 = 80.0;
/// Out to this → Low; else UltraLow.
pub const PANEL_LOW_FACTOR: f32 = 250.0;

/// Keep High while the viewer is this close, even on small tiles.
pub const PANEL_HIGH_METERS: f32 = 20.0;
/// Keep Medium out to this range so High/Mid does not thrash.
pub const PANEL_MEDIUM_METERS: f32 = 80.0;
/// Keep Low out to this range.
pub const PANEL_LOW_METERS: f32 = 250.0;

/// Shared UltraLow rectangle kit for every [`crate::panels::PanelStyle`].
pub const PANEL_ULTRA_LOW_RECTANGLE: AssetPath = flat::RECTANGLE_LOW;
/// Shared UltraLow right-triangle kit for every [`crate::panels::PanelStyle`].
pub const PANEL_ULTRA_LOW_RIGHT_TRIANGLE: AssetPath = flat::RIGHT_TRIANGLE_LOW;

/// Viewer distance band for panel mesh resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PanelLodBand {
	UltraLow,
	Low,
	Medium,
	High,
}

impl PanelLodBand {
	pub fn from_distance_factor(factor: f32) -> Self {
		match DistanceLodBand::from_factors(
			factor,
			PANEL_HIGH_FACTOR,
			PANEL_MEDIUM_FACTOR,
			PANEL_LOW_FACTOR,
		) {
			DistanceLodBand::High => Self::High,
			DistanceLodBand::Medium => Self::Medium,
			DistanceLodBand::Low => Self::Low,
			DistanceLodBand::UltraLow => Self::UltraLow,
		}
	}

	/// UltraLow is a real host root (flat low-res), not collapsed onto Low.
	pub fn to_lod_scene_level(self) -> LodSceneLevel {
		match self {
			Self::High => LodSceneLevel::High,
			Self::Medium => LodSceneLevel::Medium,
			Self::Low => LodSceneLevel::Low,
			Self::UltraLow => LodSceneLevel::UltraLow,
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

/// Fine-phase probe for panel mesh hosts (center + characteristic extent).
#[derive(Debug, Clone, Copy, Component, Default)]
pub struct PanelLodProbe {
	pub center: Vec3,
	pub extent: f32,
}

impl PanelLodProbe {
	pub fn from_placement(placement: &Placement) -> Self {
		Self { center: placement_center(placement), extent: characteristic_extent_abs(placement) }
	}

	pub fn from_aabb(aabb: &Aabb3d) -> Self {
		let (center, extent) = center_extent_from_aabb(aabb);
		Self { center, extent }
	}

	pub fn band_for(&self, viewer: &Transform) -> PanelLodBand {
		let dist = viewer.translation.distance(self.center);
		let factor = dist / self.extent.max(1e-4);
		if dist <= PANEL_HIGH_METERS || factor <= PANEL_HIGH_FACTOR {
			PanelLodBand::High
		} else if dist <= PANEL_MEDIUM_METERS || factor <= PANEL_MEDIUM_FACTOR {
			PanelLodBand::Medium
		} else if dist <= PANEL_LOW_METERS || factor <= PANEL_LOW_FACTOR {
			PanelLodBand::Low
		} else {
			PanelLodBand::UltraLow
		}
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

impl LodScene for PanelLodProbe {
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
pub(crate) fn panel_kit_scene_ref(
	high: SceneRef,
	mid: SceneRef,
	low: SceneRef,
	ultra_low: SceneRef,
	level: LodSceneLevel,
) -> SceneRef {
	match level {
		LodSceneLevel::High => high,
		LodSceneLevel::Medium => mid,
		LodSceneLevel::Low => low,
		LodSceneLevel::UltraLow => ultra_low,
		LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => mid,
	}
}

/// Posed panel kit content for one [`LodSceneLevel`] (no host scaffolding).
pub fn panel_scene_ref_for_level(
	high: SceneRef,
	mid: SceneRef,
	low: SceneRef,
	ultra_low: SceneRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	panel_kit_scene_ref(high, mid, low, ultra_low, level).scene()
}

/// Update panel host levels from the [`lod::LodViewer`] pose.
pub fn update_panel_host_levels(
	viewer: Query<&lod::LodNodePose, With<lod::LodViewer>>,
	mut hosts: Query<(&PanelLodProbe, &mut LodSceneLevel), With<LodSceneHost>>,
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
	fn ultra_low_is_distinct_from_low() -> anyhow::Result<()> {
		assert_eq!(
			PanelLodBand::from_distance_factor(PANEL_LOW_FACTOR).to_lod_scene_level(),
			LodSceneLevel::Low
		);
		assert_eq!(
			PanelLodBand::from_distance_factor(PANEL_LOW_FACTOR + 1.0).to_lod_scene_level(),
			LodSceneLevel::UltraLow
		);
		assert_eq!(PANEL_ULTRA_LOW_RECTANGLE, flat::RECTANGLE_LOW);
		assert_eq!(PANEL_ULTRA_LOW_RIGHT_TRIANGLE, flat::RIGHT_TRIANGLE_LOW);
		Ok(())
	}

	#[test]
	fn high_holds_to_twenty_meters_even_on_small_tiles() -> anyhow::Result<()> {
		let probe = PanelLodProbe { center: Vec3::ZERO, extent: 0.4 };
		assert_eq!(
			probe.band_for(&Transform::from_xyz(0.0, 0.0, PANEL_HIGH_METERS)),
			PanelLodBand::High
		);
		assert_eq!(
			probe.band_for(&Transform::from_xyz(0.0, 0.0, PANEL_HIGH_METERS + 1.0)),
			PanelLodBand::Medium
		);
		assert_eq!(
			probe.band_for(&Transform::from_xyz(0.0, 0.0, PANEL_MEDIUM_METERS)),
			PanelLodBand::Medium
		);
		Ok(())
	}
}
