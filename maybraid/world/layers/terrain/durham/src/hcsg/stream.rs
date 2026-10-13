//! The playable world's terrain streams on the shared runtime.
//!
//! Each stream is one ring of [`TerrainCellRing`] cells around the viewer.
//! [`StreamRing<R>`] sends stream `R`'s cells as boxes (the near disk, or four
//! strips around a far ring's hole), and [`Streamed<R, T>`] is a value `T` on
//! one of those cells, so each stream discovers only its own cell size.
//! Banding within a ring is `T`'s own [`SemanticLodScene`].

use std::marker::PhantomData;
use std::sync::Arc;

use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use lod::gen::{Id, OriginalId};
use lod::hcsg::{
	self, GenerationContext, HcsgClass, HcsgValue, PresentationPlugin, ViewerHcsgBounds,
};
use lod::lod_ref::LodRef;
use lod::scene::{LodSceneCulls, LodSceneLevel, LodSceneStatus, SceneChunk, SemanticLodScene};

use crate::terrain::host::{WORLD_BACKGROUND_RING, WORLD_FAR_RING, WORLD_NEAR_RING};
use crate::terrain::{TerrainCellLayout, TerrainCellRing};

/// One terrain stream: the ring its cells tile.
pub trait TerrainStream: Send + Sync + 'static {
	const RING: TerrainCellRing;
	const CLASS: HcsgClass;
}

/// The playable near disk: 160 m cells that own collision.
pub struct NearStream;

/// The playable far ring: 320 m cells around the near disk.
pub struct FarStream;

/// The playable background ring: 640 m cells around the far ring.
pub struct BackgroundStream;

impl TerrainStream for NearStream {
	const RING: TerrainCellRing = WORLD_NEAR_RING;
	const CLASS: HcsgClass = HcsgClass::Near;
}

impl TerrainStream for FarStream {
	const RING: TerrainCellRing = WORLD_FAR_RING;
	const CLASS: HcsgClass = HcsgClass::Far;
}

impl TerrainStream for BackgroundStream {
	const RING: TerrainCellRing = WORLD_BACKGROUND_RING;
	const CLASS: HcsgClass = HcsgClass::Background;
}

/// `T` on one of stream `R`'s cells.
pub struct Streamed<R, T> {
	pub value: Arc<T>,
	_stream: PhantomData<fn() -> R>,
}

impl<R, T> Streamed<R, T> {
	pub fn new(value: Arc<T>) -> Self {
		Self { value, _stream: PhantomData }
	}
}

/// The playable world's three streams together.
pub struct PlayableStreams;

impl<R: TerrainStream, T: hcsg::GenerationScheme> hcsg::GenerationScheme for Streamed<R, T> {
	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		let Some(layout) = cx.get_or_generate::<TerrainCellLayout>(Id::Universal) else {
			return Vec::new();
		};
		R::RING.cell_ids(region, layout.vertical_half_extent)
	}

	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let value = cx.get_or_generate::<T>(id)?;
		Some((Self::new(value), id.origin_cell_bounds()?))
	}
}

impl<R: TerrainStream, T: SemanticLodScene + HcsgValue> SemanticLodScene for Streamed<R, T> {
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

	fn scene_bounds(&self) -> Aabb3d {
		self.value.scene_bounds()
	}
}

/// Stream `R`'s cells around the [`LodViewer`].
pub struct StreamRing<R>(PhantomData<fn() -> R>);

impl<R: TerrainStream> ViewerHcsgBounds for StreamRing<R> {
	const CLASS: HcsgClass = R::CLASS;

	fn regions_around(viewer: Vec3) -> Vec<Aabb3d> {
		R::RING.regions_around(viewer)
	}
}

/// Presents `T` on stream `R`'s cells within channel `C`'s regions.
pub struct StreamPresentationPlugin<C, R, T>(PhantomData<fn() -> (C, R, T)>);

impl<C, R, T> Default for StreamPresentationPlugin<C, R, T> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<C, R, T> Plugin for StreamPresentationPlugin<C, R, T>
where
	C: Send + Sync + 'static,
	R: TerrainStream,
	T: hcsg::GenerationScheme + SemanticLodScene,
{
	fn build(&self, app: &mut App) {
		app.add_plugins(PresentationPlugin::<C, Streamed<R, T>>::default());
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::playable_world_cell_layout;
	use std::collections::HashSet;

	/// The cells a stream's boxes discover around `anchor`.
	fn discovered<R: TerrainStream>(anchor: Vec3) -> HashSet<OriginalId> {
		let layout = playable_world_cell_layout();
		R::RING
			.regions_around(anchor)
			.into_iter()
			.flat_map(|region| R::RING.cell_ids(region, layout.vertical_half_extent))
			.collect()
	}

	/// The cells the ring retains around `anchor`, by their centers.
	fn retained<R: TerrainStream>(anchor: Vec3) -> HashSet<OriginalId> {
		let layout = playable_world_cell_layout();
		let ring = R::RING;
		let reach = ring.high_outer_radius + 2.0 * ring.cell_size;
		let a = ring.aligned_anchor(anchor);
		let around = Aabb3d::from_min_max(
			a - Vec3::new(reach, 10.0, reach),
			a + Vec3::new(reach, 10.0, reach),
		);
		ring.cell_ids(around, layout.vertical_half_extent)
			.into_iter()
			.filter(|OriginalId(id)| {
				let cell = id.origin_cell_bounds().unwrap_or(around);
				ring.retains_cell_center(Vec3::from((cell.min + cell.max) * 0.5), anchor)
			})
			.collect()
	}

	#[test]
	fn each_stream_sends_exactly_the_cells_it_retains() {
		for anchor in [Vec3::ZERO, Vec3::new(1234.0, 0.0, -987.0), Vec3::new(-5000.0, 0.0, 320.0)] {
			assert_eq!(discovered::<NearStream>(anchor), retained::<NearStream>(anchor));
			assert_eq!(discovered::<FarStream>(anchor), retained::<FarStream>(anchor));
			assert_eq!(
				discovered::<BackgroundStream>(anchor),
				retained::<BackgroundStream>(anchor)
			);
		}
	}

	#[test]
	fn far_rings_send_four_strips_and_near_one_box() {
		assert_eq!(NearStream::RING.regions_around(Vec3::ZERO).len(), 1);
		assert_eq!(FarStream::RING.regions_around(Vec3::ZERO).len(), 4);
		assert_eq!(BackgroundStream::RING.regions_around(Vec3::ZERO).len(), 4);
	}

	#[test]
	fn layout_origin_ids_are_the_near_lattice_wherever_they_are_asked() {
		let layout = playable_world_cell_layout();
		let far_away =
			Aabb3d::from_min_max(Vec3::new(9000.0, -1.0, 9000.0), Vec3::new(9500.0, 1.0, 9200.0));
		assert_eq!(
			layout.origin_ids(far_away),
			NearStream::RING.cell_ids(far_away, layout.vertical_half_extent)
		);
	}
}
