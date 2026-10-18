//! Roof IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Quat, Transform};
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use crate::assets::panels::shepherds_thatch::{RECTANGLE_HIGH, RECTANGLE_LOW, RECTANGLE_MID};
use crate::assets::roofs::shepherds_thatch::{
	RIGHT_TRIANGLE_HIGH, RIGHT_TRIANGLE_LOW, RIGHT_TRIANGLE_MID,
};
use crate::kit_merge::{scenes_from_kit_parts, KitPart};
use crate::lod_band::{placement_bounds, warm_mesh_lod_culls};
use crate::parent_confines::ParentConfines;
use crate::partitions::host::mesh_scene_ref;
use crate::partitions::mesh_set::PartitionMeshSet;
use crate::placed::Placement;
use crate::roofs::geometry::RoofGeometry;
use crate::roofs::lod::{roof_kit_scene_ref, RoofLodProbe};
use crate::roofs::style::RoofStyle;
use crate::roofs::tessellate::RoofKit;
use crate::scene_children::{pose, scene_children};

/// Authoring IR for a roof / cap feature.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct RoofNode {
	pub style: RoofStyle,
	pub geometry: RoofGeometry,
	pub placement: Placement,
}

impl RoofNode {
	pub fn new(style: RoofStyle, geometry: RoofGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
	}

	pub fn shepherds_thatch(geometry: RoofGeometry, placement: Placement) -> Self {
		Self::new(RoofStyle::ShepherdsThatch, geometry, placement)
	}

	fn probe(&self) -> RoofLodProbe {
		RoofLodProbe::from_placement(&self.placement)
	}

	pub(crate) fn kit_parts(&self, level: LodSceneLevel) -> Vec<KitPart> {
		let parent = pose(self.placement);
		let pitch = Transform::from_rotation(Quat::from_rotation_x(self.geometry.pitch_radians()));
		self.geometry
			.kit_pieces_for_style(self.style)
			.into_iter()
			.filter_map(|piece| {
				let transform = parent * pitch * pose(piece.placement);
				let scene = match (self.style, piece.geom) {
					(RoofStyle::ShepherdsThatch, RoofKit::RightTriangle { mirror }) => {
						Some(mesh_scene_ref(
							PartitionMeshSet::new(
								RIGHT_TRIANGLE_HIGH,
								RIGHT_TRIANGLE_MID,
								RIGHT_TRIANGLE_LOW,
							),
							level,
							mirror,
						))
					}
					(RoofStyle::ShepherdsThatch, RoofKit::Rectangle) => Some(roof_kit_scene_ref(
						RECTANGLE_HIGH.scene_ref(),
						RECTANGLE_MID.scene_ref(),
						RECTANGLE_LOW.scene_ref(),
						level,
					)),
					(RoofStyle::ShepherdsThatch, RoofKit::DomeArc(_)) => None,
				}?;
				Some(KitPart {
					scene,
					transform,
					material: None,
					confines: ParentConfines::External,
				})
			})
			.collect()
	}

	fn content_for_level(&self, level: LodSceneLevel) -> impl Scene + 'static {
		scene_children(scenes_from_kit_parts(self.kit_parts(level)))
	}
}

impl LodScene for RoofNode {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.probe().level_for(lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.probe().status_for_lod_ref(lod_ref)
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		warm_mesh_lod_culls(current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.content_for_level(level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
