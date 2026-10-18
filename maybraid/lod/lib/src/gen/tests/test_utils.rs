use crate::lod_ref::LodRef;
use crate::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::scene::{ResolveContext, ResolvedScene, Scene, SceneFunction};

pub fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

pub fn stub_scene() -> impl Scene + 'static {
	SceneFunction(empty_scene)
}

pub fn cell(x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(x, 0.0, 0.0), Vec3::new(x + 1.0, 1.0, 1.0))
}

pub fn span(min_x: f32, max_x: f32) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(min_x, 0.0, 0.0), Vec3::new(max_x, 1.0, 1.0))
}

#[derive(Debug, Clone, PartialEq)]
pub struct Terrain {
	pub cell: Aabb3d,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Vegetation {
	pub cell: Aabb3d,
}

macro_rules! impl_lod_scene {
	($ty:ty) => {
		impl LodScene for $ty {
			fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
				LodSceneStatus::Unchanged
			}

			fn scene_with_level(
				&self,
				_lod_ref: &LodRef,
				_level: LodSceneLevel,
			) -> impl Scene + 'static {
				stub_scene()
			}
		}
	};
}

impl_lod_scene!(Terrain);
impl_lod_scene!(Vegetation);
