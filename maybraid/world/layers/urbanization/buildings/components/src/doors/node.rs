//! Door IR node: style + geometry + placement — fine-phase [`LodScene`] host.

use bevy::math::bounding::Aabb3d;
use bevy::prelude::Component;
use bevy::scene::prelude::Scene;
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;

use crate::doors::geometry::DoorGeometry;
use crate::doors::style::DoorStyle;
use crate::doors::tessellate::DoorKit;
use crate::doors::WoodDoorLeaf;
use crate::lod_band::placement_bounds;
use crate::partitions::node::partition_tile_scene;
use crate::placed::Placement;
use crate::scene_children::{pose, scene_children, with_pose};

/// Authoring IR for a door feature.
#[derive(Debug, Clone, PartialEq, Component, Default)]
pub struct DoorNode {
	pub style: DoorStyle,
	pub geometry: DoorGeometry,
	pub placement: Placement,
}

impl DoorNode {
	pub fn new(style: DoorStyle, geometry: DoorGeometry, placement: Placement) -> Self {
		Self { style, geometry, placement }
	}

	pub fn wood(geometry: DoorGeometry, placement: Placement) -> Self {
		Self::new(DoorStyle::Wood, geometry, placement)
	}
}

impl LodScene for DoorNode {
	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = self
			.geometry
			.placed_kits(self.placement)
			.into_iter()
			.map(|piece| {
				let transform = pose(piece.placement);
				let child: Box<dyn Scene> = match piece.geom {
					DoorKit::Leaf => Box::new(WoodDoorLeaf.scene_with_level(lod_ref, level)),
					DoorKit::FramePiece(kit) => partition_tile_scene(kit, level),
				};
				Box::new(with_pose(transform, child)) as Box<dyn Scene>
			})
			.collect();
		scene_children(children)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		SceneChunk::primitive(self.scene_with_level(lod_ref, level))
	}

	fn scene_bounds(&self) -> Aabb3d {
		placement_bounds(&self.placement)
	}
}
