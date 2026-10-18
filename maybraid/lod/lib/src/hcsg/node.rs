//! [`HcsgNode<T>`]: the Bevy host component for one published value.

use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Component, Scene};

use crate::gen::{Id, Version};
use crate::lod_ref::LodRef;
use crate::scene::{
	LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk, SemanticLodScene, VisualLodScene,
	VisualSceneChunk,
};

use super::storage::HcsgValue;

/// One published value on a presentation host.
///
/// Hosts sit at the identity transform, so a value's scenes are posed in
/// world space and [`Self::bounds`] (its stored bounds) are the host's scene
/// bounds. `T` needs no `Component`, `Clone` or `Default`.
#[derive(Component)]
pub struct HcsgNode<T: HcsgValue> {
	pub id: Id,
	pub version: Version,
	pub bounds: Aabb3d,
	pub value: Arc<T>,
}

impl<T: HcsgValue> Clone for HcsgNode<T> {
	fn clone(&self) -> Self {
		Self {
			id: self.id,
			version: self.version,
			bounds: self.bounds,
			value: Arc::clone(&self.value),
		}
	}
}

impl<T: HcsgValue + SemanticLodScene> SemanticLodScene for HcsgNode<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.value.scene_lod_level(lod_ref)
	}

	fn scene_lod_level_from_levels(&self, lod_refs: &[LodRef]) -> LodSceneLevel {
		self.value.scene_lod_level_from_levels(lod_refs)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.value.scene_lod_status(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		self.value.scene_lod_culls(lod_ref, current)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.value.scene_with_level(lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		self.value.scene_chunks_with_level(lod_ref, level)
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		self.value.scene_with_lod(lod_ref)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.bounds
	}
}

impl<T: HcsgValue + VisualLodScene> VisualLodScene for HcsgNode<T> {
	fn visual_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> VisualSceneChunk {
		self.value.visual_chunks_with_level(lod_ref, level)
	}
}
