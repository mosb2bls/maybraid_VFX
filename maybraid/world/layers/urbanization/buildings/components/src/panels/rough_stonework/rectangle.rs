//! Rough-stonework unit rectangle panel kit (LOD triad + flat UltraLow).

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use crate::assets::panels::rough_stonework::{RECTANGLE_HIGH, RECTANGLE_LOW, RECTANGLE_MID};
use crate::panels::lod::{panel_scene_ref_for_level, PanelLodProbe, PANEL_ULTRA_LOW_RECTANGLE};

/// Unit rectangle \(X, Z \in [0, 1]\), \(Y \in [-0.2, 0.2]\).
#[derive(Debug, Clone, Copy, PartialEq, Default, Component)]
pub struct RoughStonePanelRectangle;

impl LodScene for RoughStonePanelRectangle {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		PanelLodProbe::from_aabb(lod_ref.bounds).level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		PanelLodProbe::from_aabb(lod_ref.bounds).status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		crate::lod_band::warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		panel_scene_ref_for_level(
			RECTANGLE_HIGH.scene_ref(),
			RECTANGLE_MID.scene_ref(),
			RECTANGLE_LOW.scene_ref(),
			PANEL_ULTRA_LOW_RECTANGLE.scene_ref(),
			level,
		)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		Aabb3d::from_min_max(Vec3::new(0.0, -0.2, 0.0), Vec3::new(1.0, 0.2, 1.0))
	}
}
