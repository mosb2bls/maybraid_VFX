//! Present-layer [`LodScene`] for a grown forest grove tile ([#652](https://github.com/ramate-io/maybraid/issues/652)).
//!
//! `ChicoForest` stays select-only. Playground forest present registers this one
//! host type. Typed `/show orchard` keeps the concrete grove `LodScene`.

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use lod::{lod_host_scene_pending, LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk};

use crate::{ForestGroveTile, ForestLayer, LayerDropOut};

/// One presented forest grove: tile geometry plus the stacking layer.
#[derive(Clone, Component)]
pub struct ChicoGroveHost {
	pub tile: ForestGroveTile,
	pub layer: ForestLayer,
}

impl ChicoGroveHost {
	pub fn new(tile: ForestGroveTile, layer: ForestLayer) -> Self {
		Self { tile, layer }
	}

	pub fn drop_out(&self) -> LayerDropOut {
		LayerDropOut::for_stacked(self.layer, self.tile.is_tuft())
	}

	/// This tile as a nested host, pending at the level `lod_ref` (the
	/// viewer) sees it.
	pub fn scene(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let host = self.clone();
		(
			lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds()),
			bsn! {
				template_value(Transform::IDENTITY)
				Visibility::default()
				template(move |_ctx| Ok(host.clone()))
			},
		)
	}

	fn empty_scene() -> impl Scene + 'static {
		bsn! {
			Visibility::Inherited
		}
	}

	fn empty_chunks() -> SceneChunk {
		SceneChunk::primitive(Self::empty_scene())
	}

	fn tuft_high_chunks(&self, lod_ref: &LodRef, min_height_m: f32) -> Option<SceneChunk> {
		use ForestGroveTile::*;
		let chunks = match &self.tile {
			BraidGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			CommonTufts(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			MonsterGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			TallGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			TropicalTufts(g) => g.high_chunks_dropping_shorter_than(lod_ref, min_height_m),
			WildGrass(g) => g.tuft_body().high_medium_chunks_dropping_shorter_than(
				lod_ref,
				LodSceneLevel::High,
				min_height_m,
			),
			_ => return None,
		};
		Some(chunks)
	}
}

impl LodScene for ChicoGroveHost {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.tile.scene_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.tile.scene_lod_status(lod_ref)
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		self.tile.scene_lod_culls(lod_ref, current)
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, _level: LodSceneLevel) -> impl Scene + 'static {
		Self::empty_scene()
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		let drop = self.drop_out();
		if drop.omits(level) {
			return Self::empty_chunks();
		}
		if drop.min_height_m > 0.0 && level == LodSceneLevel::High {
			if let Some(chunks) = self.tuft_high_chunks(lod_ref, drop.min_height_m) {
				return chunks;
			}
		}
		self.tile.scene_chunks_with_level(lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.tile.scene_bounds()
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::grove::forest_world_sample;
	use crate::{ChicoGrove, ForestGroveKind, ForestGroveRecipe};
	use bevy::prelude::{Entity, Transform, Vec3};
	use lod::LodScene;
	use vegetation_groves::GroveExtent;

	fn lod_at(translation: Vec3) -> (Transform, Aabb3d) {
		(Transform::from_translation(translation), Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE))
	}

	fn with_lod<R>(translation: Vec3, f: impl FnOnce(&LodRef<'_>) -> R) -> R {
		let (xf, bounds) = lod_at(translation);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &xf,
			current_transform: &xf,
			bounds: &bounds,
		};
		f(&lod_ref)
	}

	#[test]
	fn tufts_drop_out_omits_medium() {
		assert!(ForestLayer::Tufts.drop_out().omits(LodSceneLevel::Medium));
		assert!(ForestLayer::Tufts.drop_out().omits(LodSceneLevel::Low));
		assert!(ForestLayer::Tufts.drop_out().omits(LodSceneLevel::UltraLow));
		assert!(!ForestLayer::Tufts.drop_out().omits(LodSceneLevel::High));
		assert!(!ForestLayer::UpperCanopy.drop_out().omits(LodSceneLevel::Medium));
	}

	#[test]
	fn tufts_host_medium_is_empty_high_is_not() {
		let extent = GroveExtent::new(Vec3::ZERO, Vec3::new(100.0, 1.0, 100.0));
		let grove = ChicoGrove::selected(
			extent,
			ForestLayer::Tufts,
			vec![ForestGroveRecipe::uniform(ForestGroveKind::CommonTufts, extent)],
		);
		let tile = grove.grow(&forest_world_sample()).remove(0);
		let host = ChicoGroveHost::new(tile, ForestLayer::Tufts);
		with_lod(Vec3::new(0.0, 20.0, 40.0), |lod_ref| {
			assert_eq!(
				host.scene_chunks_with_level(lod_ref, LodSceneLevel::Medium).total_primitives(),
				1
			);
			assert!(host.scene_chunks_with_level(lod_ref, LodSceneLevel::High).total_weight() > 1);
		});
	}
}
