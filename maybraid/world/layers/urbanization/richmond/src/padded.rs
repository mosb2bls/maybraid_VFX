//! Terrain origin cell with development-pad elevation ops applied.

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use durham::terrain::ElevationModulation;
use durham::{
	cascade_chunk_for_cell, stream_banded_level, stream_banded_scene, ComposedTerrain,
	StreamBandedLod, Terrain, TerrainCellRing, TerrainColliderMeshSource, TerrainMeshBuilder,
	TerrainSdf,
};
use lod::gen::{Id, OriginalId};
use lod::hcsg::{self, GenerationContext};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneLevel, LodSceneStatus};
use render_item::mesh::handle::Cached;
use render_item::sdf::cpu_shot::{CpuShotBuilder, WallFaces};
use std::marker::PhantomData;
use std::sync::Arc;
use terrain_shaders::TerrainShader;

use crate::compose::PadComposable;
use crate::developments::RichmondDevelopment;
use crate::ground::RichmondGround;
use crate::pad::PadComplex;
/// Durham [`Terrain`] plus overlapping development pads.
#[derive(Debug, Clone, Component)]
pub struct TerrainWithPads {
	pub cell: Aabb3d,
	pub sdf: Arc<ComposedTerrain>,
	pub material: Handle<TerrainShader>,
	pub res_2: u8,
	pub wall_faces: WallFaces,
	pub pad_count: usize,
	/// Moving stream band copied from the source Durham cell.
	pub stream_ring: Option<TerrainCellRing>,
}

impl TerrainWithPads {
	/// Without pad nodes the terrain is wrapped as is: its surface and walls.
	pub fn compose<'a>(terrain: &Terrain, pads: impl IntoIterator<Item = &'a PadComplex>) -> Self {
		let mut pad_count = 0;
		let mut nodes = Vec::new();
		for pad in pads {
			pad_count += 1;
			nodes.extend(pad.pads.iter().cloned());
		}
		let merged = PadComplex::from_nodes(nodes);
		if merged.is_empty() {
			return Self {
				cell: terrain.cell,
				sdf: Arc::clone(&terrain.sdf),
				material: terrain.material.clone(),
				res_2: terrain.res_2,
				wall_faces: terrain.wall_faces,
				pad_count: 0,
				stream_ring: terrain.stream_ring,
			};
		}
		let mut sdf: TerrainSdf = terrain.sdf.terrain().clone();
		sdf.add_elevation_modulation(Box::new(merged) as Box<dyn ElevationModulation>);
		Self {
			cell: terrain.cell,
			sdf: Arc::new(ComposedTerrain::from_terrain(sdf)),
			material: terrain.material.clone(),
			res_2: terrain.res_2,
			// Building-skirt pads can still meet origin-cell faces on a large
			// footprint; interior skirts close the CpuShot crack.
			wall_faces: WallFaces::ALL,
			pad_count,
			stream_ring: terrain.stream_ring,
		}
	}

	pub fn mesh_builder(&self) -> TerrainMeshBuilder {
		CpuShotBuilder::new(Arc::clone(&self.sdf)).with_wall_faces(self.wall_faces)
	}

	/// World pose for CpuShot verts, which are local to the cascade origin.
	pub fn chunk_pose(&self) -> Transform {
		Transform::from_translation(cascade_chunk_for_cell(self.cell, self.res_2).origin)
	}

	/// Posed fill entity. [`Mesh3d`] (and the trimesh, when [`Self::seeds_collision`])
	/// land on this same entity after Cached fulfill.
	pub fn spawn_fill(
		&self,
		commands: &mut Commands,
		visibility: Visibility,
		collide: bool,
	) -> Entity {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let entity = commands
			.spawn((
				self.chunk_pose(),
				chunk,
				Cached::new(self.mesh_builder()),
				MeshMaterial3d(self.material.clone()),
				visibility,
			))
			.id();
		if collide {
			commands.entity(entity).insert(TerrainColliderMeshSource);
		}
		entity
	}

	/// Visual fill plus the trimesh source.
	pub fn scene(&self) -> impl Scene + 'static {
		(self.mesh_scene(), bsn! { TerrainColliderMeshSource })
	}

	pub fn seeds_collision(&self) -> bool {
		self.stream_ring.map(|ring| ring.seeds_collision()).unwrap_or(true)
	}

	fn center(&self) -> Vec3 {
		(Vec3::from(self.cell.min) + Vec3::from(self.cell.max)) * 0.5
	}

	/// Visual fill. Pose is the cascade origin; [`Mesh3d`] fulfills onto this root.
	pub fn mesh_scene(&self) -> impl Scene + 'static {
		let chunk = cascade_chunk_for_cell(self.cell, self.res_2);
		let transform = self.chunk_pose();
		let builder = self.mesh_builder();
		let material = self.material.clone();
		bsn! {
			template_value(transform)
			template_value(chunk)
			template(move |_ctx| Ok(Cached::new(builder.clone())))
			MeshMaterial3d::<TerrainShader>({material.clone()})
		}
	}
}

impl StreamBandedLod for TerrainWithPads {
	fn stream_ring(&self) -> Option<TerrainCellRing> {
		self.stream_ring
	}

	fn stream_center(&self) -> Vec3 {
		self.center()
	}
}

impl LodScene for TerrainWithPads {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		stream_banded_level(self, lod_ref.current_transform)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		let previous = stream_banded_level(self, lod_ref.previous_transform);
		let current = stream_banded_level(self, lod_ref.current_transform);
		if previous == current {
			LodSceneStatus::Unchanged
		} else {
			LodSceneStatus::Changed(current)
		}
	}

	fn scene_with_level(&self, _lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		if self.seeds_collision() {
			stream_banded_scene(self, level, || self.scene())
		} else {
			stream_banded_scene(self, level, || self.mesh_scene())
		}
	}
}

/// A cell of ground `G` with the pads of the developments over it composed in.
pub struct PaddedTerrain<G> {
	pub surface: TerrainWithPads,
	_ground: PhantomData<fn() -> G>,
}

impl<G: RichmondGround> LodScene for PaddedTerrain<G> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.surface.scene_lod_level(lod_ref)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		self.surface.scene_lod_status(lod_ref)
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		self.surface.scene_with_level(lod_ref, level)
	}
}

impl<G> PaddedTerrain<G> {
	pub fn new(surface: TerrainWithPads) -> Self {
		Self { surface, _ground: PhantomData }
	}
}

/// Total over the ground: every cell of `G` is presented padded, wrapped as
/// is where no pad reaches it, so the raw cell is never presented beneath.
impl<G: RichmondGround> hcsg::GenerationScheme for PaddedTerrain<G> {
	lod::hcsg_index_scale!(crate::storage::COLUMN_INDEX_SCALE);

	fn original_ids_for(cx: &mut GenerationContext, region: Aabb3d) -> Vec<OriginalId> {
		cx.original_ids_for::<G::Cell>(region)
	}

	/// The cell with the pads of every development over it.
	fn build_with_id(cx: &mut GenerationContext, id: Id) -> Option<(Self, Aabb3d)> {
		let (cell, bounds) = cx.get_or_generate_stored::<G::Cell>(id)?;
		let developments = RichmondDevelopment::<G>::filled_in(cx, bounds);
		let pads = RichmondDevelopment::merge_pads(bounds, developments.iter().map(Arc::as_ref));
		Some((Self::new(cell.as_ref().compose_pads(&pads)), bounds))
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use durham::{stream_banded_draws, stream_banded_level, TerrainSdf, TERRAIN_CELL_SIZE};
	use lod::LodSceneLevel;

	fn far_ring() -> TerrainCellRing {
		TerrainCellRing {
			cell_size: 2.0 * TERRAIN_CELL_SIZE,
			res_2: 4,
			anchor_step: 4.0 * TERRAIN_CELL_SIZE,
			high_inner_radius: 8.0 * TERRAIN_CELL_SIZE,
			high_outer_radius: 16.0 * TERRAIN_CELL_SIZE,
			cull_margin: 8.0 * TERRAIN_CELL_SIZE,
		}
	}

	fn pad_at(center: Vec3, ring: TerrainCellRing) -> TerrainWithPads {
		let half = ring.cell_size * 0.5;
		let bounds = Aabb3d::from_min_max(
			Vec3::new(center.x - half, -1.0, center.z - half),
			Vec3::new(center.x + half, 1.0, center.z + half),
		);
		TerrainWithPads {
			cell: bounds,
			sdf: Arc::new(ComposedTerrain::from_terrain(TerrainSdf::new(1, 20.0))),
			material: Handle::default(),
			res_2: ring.res_2,
			wall_faces: WallFaces::NONE,
			pad_count: 0,
			stream_ring: Some(ring),
		}
	}

	#[test]
	fn far_pad_underfoot_is_empty_high() {
		let pad = pad_at(Vec3::ZERO, far_ring());
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&pad, &viewer);
		assert_eq!(level, LodSceneLevel::High);
		assert!(!stream_banded_draws(&pad, level));
	}

	#[test]
	fn far_pad_in_ring_is_drawn_medium() {
		let ring = far_ring();
		let pad = pad_at(Vec3::new(ring.high_inner_radius + 80.0, 0.0, 0.0), ring);
		let viewer = Transform::IDENTITY;
		let level = stream_banded_level(&pad, &viewer);
		assert_eq!(level, LodSceneLevel::Medium);
		assert!(stream_banded_draws(&pad, level));
	}

	#[test]
	fn unbanded_pad_always_draws_high() {
		let mut pad = pad_at(Vec3::ZERO, far_ring());
		pad.stream_ring = None;
		let viewer = Transform::IDENTITY;
		assert_eq!(stream_banded_level(&pad, &viewer), LodSceneLevel::High);
		assert!(stream_banded_draws(&pad, LodSceneLevel::High));
	}

	#[test]
	fn chunk_pose_is_cascade_origin() {
		let pad = pad_at(Vec3::new(160.0, 0.0, -320.0), far_ring());
		let chunk = cascade_chunk_for_cell(pad.cell, pad.res_2);
		assert_eq!(pad.chunk_pose(), Transform::from_translation(chunk.origin));
		assert_ne!(pad.chunk_pose(), Transform::IDENTITY);
	}

	mod scheme_perf_tests {
		use super::*;
		use crate::config::{DevelopmentConfig, DevelopmentSites};
		use crate::developments::RichmondDevelopment;
		use crate::ground::GroundCell;
		use durham::terrain::host::TerrainCoverage;
		use durham::{
			fine_patch_cell_layout, mesh_assets, CellTiling, Durham, Terrain, TerrainCellLayout,
			TerrainConfig, TerrainStampConfigs, WatershedConfigs,
		};
		use lod::gen::{Id, OriginalId};
		use lod::hcsg::{self, universal_bounds, GenerationContext, HcsgStorage};
		use std::sync::Arc;
		use std::time::Instant;
		use terrain_layer_model::OnTerrain;
		use urbanization_cells::{UrbanizationKind, UrbanizationSelection};

		type Ground = OnTerrain<Durham>;

		fn seed_patch(
			storage: &HcsgStorage,
			seed: u32,
			origin: bevy::math::IVec2,
			development: DevelopmentConfig,
			selection: UrbanizationSelection,
		) -> TerrainCellLayout {
			let layout = fine_patch_cell_layout(1, origin);
			storage.seed(layout.clone(), universal_bounds());
			storage.seed(TerrainStampConfigs::from_world_seed(seed), universal_bounds());
			storage.seed(WatershedConfigs::default().with_seed(seed), universal_bounds());
			storage.seed(
				mesh_assets(
					TerrainConfig::new(seed),
					Handle::default(),
					TerrainCoverage::FinePatch,
					1,
				),
				universal_bounds(),
			);
			storage.seed(development, universal_bounds());
			storage.seed(selection, universal_bounds());
			layout
		}

		fn build_padded_legacy(cx: &mut GenerationContext, id: Id) -> Option<TerrainWithPads> {
			let cell = cx.get_or_generate::<Terrain>(id)?;
			let bounds = GroundCell::bounds(cell.as_ref());
			let developments = cx.get_or_generate_in::<RichmondDevelopment<Ground>>(bounds);
			let pads = RichmondDevelopment::merge_pads(bounds, developments.iter().map(Arc::as_ref));
			Some(cell.as_ref().compose_pads(&pads))
		}

		fn padded_snapshot(padded: &TerrainWithPads) -> (usize, f32, f32, f32) {
			let center = (Vec3::from(padded.cell.min) + Vec3::from(padded.cell.max)) * 0.5;
			let height = |x, z| padded.sdf.terrain().height_at_with_all_modulations(x, z);
			(padded.pad_count, height(center.x, center.z), height(center.x + 4.0, center.z), height(center.x, center.z + 4.0))
		}

		fn assert_patch_matches(
			seed: u32,
			origin: bevy::math::IVec2,
			development: DevelopmentConfig,
			selection: UrbanizationSelection,
		) -> anyhow::Result<()> {
			let layout = fine_patch_cell_layout(1, origin);
			let ids: Vec<Id> = layout
				.cell_ids(layout.request_region())
				.into_iter()
				.map(|OriginalId(id)| id)
				.collect();

			let legacy_storage = HcsgStorage::default();
			seed_patch(&legacy_storage, seed, origin, development.clone(), selection.clone());
			let mut legacy_cx = GenerationContext::new(&legacy_storage);

			let optimized_storage = HcsgStorage::default();
			seed_patch(&optimized_storage, seed, origin, development, selection);
			let mut optimized_cx = GenerationContext::new(&optimized_storage);

			for id in ids {
				let legacy = build_padded_legacy(&mut legacy_cx, id)
					.ok_or_else(|| anyhow::anyhow!("legacy padded missing for {id:?}"))?;
				let optimized = <PaddedTerrain<Ground> as hcsg::GenerationScheme>::build_with_id(
					&mut optimized_cx,
					id,
				)
					.ok_or_else(|| anyhow::anyhow!("optimized padded missing for {id:?}"))?
					.0
					.surface;
				assert_eq!(
					padded_snapshot(&legacy),
					padded_snapshot(&optimized),
					"seed {seed} origin {origin:?} cell {id:?}"
				);
			}
			Ok(())
		}

		#[test]
		fn padded_terrain_matches_skipping_empty_developments() -> anyhow::Result<()> {
			let lattice = DevelopmentConfig {
				sites: DevelopmentSites::Lattice,
				seed: 42,
				..DevelopmentConfig::default()
			};
			let urban = DevelopmentConfig {
				sites: DevelopmentSites::Urbanization,
				..DevelopmentConfig::default()
			};
			let selection = UrbanizationSelection {
				kind: Some(UrbanizationKind::Frontier),
				..UrbanizationSelection::default()
			};
			for seed in [11u32, 42, 99, 7_001] {
				assert_patch_matches(seed, bevy::math::IVec2::new(-1, -1), lattice.clone(), selection.clone())?;
				assert_patch_matches(seed, bevy::math::IVec2::new(0, 1), lattice.clone(), selection.clone())?;
				assert_patch_matches(seed, bevy::math::IVec2::new(-1, -1), urban.clone(), selection.clone())?;
				assert_patch_matches(seed, bevy::math::IVec2::new(0, 1), urban.clone(), selection.clone())?;
			}
			Ok(())
		}

		fn cold_patch_build_time(
			seed: u32,
			origin: bevy::math::IVec2,
			legacy: bool,
		) -> std::time::Duration {
			let layout = fine_patch_cell_layout(1, origin);
			let ids: Vec<Id> = layout
				.cell_ids(layout.request_region())
				.into_iter()
				.map(|OriginalId(id)| id)
				.collect();
			let development = DevelopmentConfig {
				sites: DevelopmentSites::Lattice,
				seed,
				..DevelopmentConfig::default()
			};
			let selection = UrbanizationSelection::default();
			let storage = HcsgStorage::default();
			seed_patch(&storage, seed, origin, development, selection);
			let mut cx = GenerationContext::new(&storage);
			let start = Instant::now();
			for id in ids {
				if legacy {
					build_padded_legacy(&mut cx, id).expect("legacy padded");
				} else {
					<PaddedTerrain<Ground> as hcsg::GenerationScheme>::build_with_id(&mut cx, id)
						.expect("optimized padded");
				}
			}
			start.elapsed()
		}

		/// `cargo test -p richmond padded_terrain_scheme_microbench --release -- --ignored --nocapture`
		#[test]
		#[ignore = "microbench"]
		fn padded_terrain_scheme_microbench() {
			let seed = 42u32;
			let origin = bevy::math::IVec2::new(-1, -1);
			let legacy = cold_patch_build_time(seed, origin, true);
			let optimized = cold_patch_build_time(seed, origin, false);
			eprintln!(
				"padded_terrain_scheme_microbench (2×2 fine patch, lattice sites, seed {seed}): \
				 legacy {:?} optimized {:?} ({:.2}x)",
				legacy,
				optimized,
				legacy.as_secs_f64() / optimized.as_secs_f64().max(1e-9)
			);
		}
	}
}
