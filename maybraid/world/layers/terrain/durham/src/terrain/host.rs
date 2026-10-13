//! Playable stream layout, presentation assets, and coverage retargeting.

use bevy::ecs::system::SystemParam;
use bevy::math::{IVec2, UVec2};
use bevy::prelude::*;
use terrain_shaders::TerrainShader;

use crate::terrain::cell::{TerrainCellLayout, TerrainCellRing, TERRAIN_CELL_SIZE};
use crate::terrain::config::TerrainConfig;
use crate::terrain::mesh::{TerrainMeshAssets, TerrainMeshLodBand};

/// Composed Durham SDF / CpuShot terrain model.
pub struct Durham;

/// Fine-only patch vs playable world extents (fine grid + macro rings).
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TerrainCoverage {
	#[default]
	FinePatch,
	PlayableWorld,
}

/// When true, a pinned fine patch keeps its origin instead of recentering on the viewer.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TerrainLayoutPinned(pub bool);

/// Base noise used for camera height before (and alongside) generation.
#[derive(Resource)]
pub struct WorldBaseTerrain(pub crate::terrain::base_noise::BaseTerrainNoise);

/// When true, fill should clear and rebuild (playground radius / seed commands).
#[derive(Resource, Default)]
pub struct TerrainPresentationDirty(pub bool);

/// When true, fill meshes have generated and present is still owed.
#[derive(Resource, Default)]
pub struct TerrainPresentPending(pub bool);

/// Near-stream High half-extent (8 × 160 m = 1.28 km).
pub const WORLD_FINE_HALF_EXTENT_CELLS: i32 = 8;
/// 2× macro ring past the fine grid (FinePatch / nested-footprint layouts).
pub const WORLD_OUTER_2X_ROWS: i32 = 2;
/// 4× macro ring past the 2× ring.
pub const WORLD_OUTER_4X_ROWS: i32 = 1;

const WORLD_TERRAIN_NEAR_RADIUS_M: f32 = 8.0 * TERRAIN_CELL_SIZE;
const WORLD_TERRAIN_FAR_RADIUS_M: f32 = 16.0 * TERRAIN_CELL_SIZE;
const WORLD_TERRAIN_BACKGROUND_RADIUS_M: f32 = 24.0 * TERRAIN_CELL_SIZE;
const WORLD_TERRAIN_CULL_MARGIN_M: f32 = 0.0;
const WORLD_TERRAIN_PRESENT_STEP_M: f32 = 4.0 * TERRAIN_CELL_SIZE;
/// Far hole inset. At least one present step so a jump retires only far cells
/// that already sit under the previous near disk.
const WORLD_TERRAIN_FAR_HOLE_INSET_M: f32 = WORLD_TERRAIN_PRESENT_STEP_M;
const WORLD_TERRAIN_BACKGROUND_HOLE_INSET_M: f32 = WORLD_TERRAIN_PRESENT_STEP_M;

fn playground_lod_bands(half_extent: i32) -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: half_extent.max(1), res_2: 5 }]
}

fn world_lod_bands() -> Vec<TerrainMeshLodBand> {
	vec![TerrainMeshLodBand { max_radius_cells: WORLD_FINE_HALF_EXTENT_CELLS, res_2: 5 }]
}

fn cell_layout(half_extent: i32) -> TerrainCellLayout {
	let r = half_extent.max(1);
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-r, -r);
	let n = (2 * r) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings.clear();
	layout.stream_rings.clear();
	layout
}

/// Playable near stream: 160 m cells that own collision.
pub const WORLD_NEAR_RING: TerrainCellRing = TerrainCellRing {
	cell_size: TERRAIN_CELL_SIZE,
	res_2: 5,
	anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
	high_inner_radius: 0.0,
	high_outer_radius: WORLD_TERRAIN_NEAR_RADIUS_M,
	cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
};

/// Playable far stream: 320 m cells around the near disk.
pub const WORLD_FAR_RING: TerrainCellRing = TerrainCellRing {
	cell_size: 2.0 * TERRAIN_CELL_SIZE,
	res_2: 4,
	anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
	high_inner_radius: WORLD_TERRAIN_NEAR_RADIUS_M - WORLD_TERRAIN_FAR_HOLE_INSET_M,
	high_outer_radius: WORLD_TERRAIN_FAR_RADIUS_M,
	cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
};

/// Playable background stream: 640 m cells around the far ring.
pub const WORLD_BACKGROUND_RING: TerrainCellRing = TerrainCellRing {
	cell_size: 4.0 * TERRAIN_CELL_SIZE,
	res_2: 3,
	anchor_step: WORLD_TERRAIN_PRESENT_STEP_M,
	high_inner_radius: WORLD_TERRAIN_FAR_RADIUS_M - WORLD_TERRAIN_BACKGROUND_HOLE_INSET_M,
	high_outer_radius: WORLD_TERRAIN_BACKGROUND_RADIUS_M,
	cull_margin: WORLD_TERRAIN_CULL_MARGIN_M,
};

fn world_cell_layout() -> TerrainCellLayout {
	let mut layout = TerrainCellLayout::default();
	layout.origin = IVec2::new(-WORLD_FINE_HALF_EXTENT_CELLS, -WORLD_FINE_HALF_EXTENT_CELLS);
	let n = (2 * WORLD_FINE_HALF_EXTENT_CELLS) as u32;
	layout.extents = UVec2::new(n, n);
	layout.outer_rings.clear();
	layout.stream_rings = vec![WORLD_NEAR_RING, WORLD_FAR_RING, WORLD_BACKGROUND_RING];
	layout
}

/// Playable-world rings (near / far / background).
pub fn playable_world_cell_layout() -> TerrainCellLayout {
	world_cell_layout()
}

/// Fine-only grid whose minimum corner is `origin`.
pub fn fine_patch_cell_layout(half_extent: i32, origin: IVec2) -> TerrainCellLayout {
	let mut layout = cell_layout(half_extent);
	layout.origin = origin;
	layout
}

fn lod_bands_for(coverage: TerrainCoverage, terrain_radius: i32) -> Vec<TerrainMeshLodBand> {
	match coverage {
		TerrainCoverage::FinePatch => playground_lod_bands(terrain_radius),
		TerrainCoverage::PlayableWorld => world_lod_bands(),
	}
}

/// Point presentation assets at a coverage after a live session retarget.
pub fn retarget_mesh_assets(
	assets: &mut TerrainMeshAssets,
	coverage: TerrainCoverage,
	terrain_radius: i32,
) {
	let (macro_seam_half_extents, macro_cell_min_size, macro_res_2) = match coverage {
		TerrainCoverage::FinePatch => (Vec::new(), None, None),
		TerrainCoverage::PlayableWorld => (
			vec![WORLD_TERRAIN_NEAR_RADIUS_M, WORLD_TERRAIN_FAR_RADIUS_M],
			Some(2.0 * TERRAIN_CELL_SIZE),
			Some(3),
		),
	};
	assets.lod_bands = lod_bands_for(coverage, terrain_radius);
	assets.fine_grid_max_radius = Some(terrain_radius);
	assets.macro_seam_half_extents = macro_seam_half_extents;
	assets.macro_cell_min_size = macro_cell_min_size;
	assets.macro_res_2 = macro_res_2;
}

/// Layout, coverage, pin, dirty, pending, and presentation assets for a retarget.
#[derive(SystemParam)]
pub struct TerrainRetarget<'w> {
	layout: ResMut<'w, TerrainCellLayout>,
	coverage: ResMut<'w, TerrainCoverage>,
	pinned: ResMut<'w, TerrainLayoutPinned>,
	dirty: ResMut<'w, TerrainPresentationDirty>,
	pending: ResMut<'w, TerrainPresentPending>,
	assets: Option<ResMut<'w, TerrainMeshAssets>>,
}

impl TerrainRetarget<'_> {
	pub fn coverage(&self) -> TerrainCoverage {
		*self.coverage
	}

	pub fn layout(&self) -> &TerrainCellLayout {
		&self.layout
	}

	pub fn apply(
		&mut self,
		layout: TerrainCellLayout,
		coverage: TerrainCoverage,
		terrain_radius: i32,
		pin: bool,
	) {
		*self.layout = layout;
		*self.coverage = coverage;
		self.pinned.0 = pin;
		self.dirty.0 = true;
		self.pending.0 = true;
		if let Some(assets) = self.assets.as_mut() {
			retarget_mesh_assets(assets, coverage, terrain_radius);
		}
	}
}

/// Terrain presentation assets for `coverage`, drawn with `material`.
pub fn mesh_assets(
	config: TerrainConfig,
	material: Handle<TerrainShader>,
	coverage: TerrainCoverage,
	terrain_radius: i32,
) -> TerrainMeshAssets {
	let mut assets = TerrainMeshAssets {
		config,
		material,
		lod_bands: Vec::new(),
		outer_add_walls: true,
		fine_grid_max_radius: None,
		macro_seam_half_extents: Vec::new(),
		macro_cell_min_size: None,
		macro_res_2: None,
	};
	retarget_mesh_assets(&mut assets, coverage, terrain_radius);
	assets
}

#[cfg(test)]
mod tests {
	use super::*;
	use crate::terrain::cell::CellTiling;
	use bevy::math::bounding::{Aabb3d, IntersectsVolume};
	use lod::gen::OriginalId;

	fn playable_rings() -> [TerrainCellRing; 3] {
		[WORLD_NEAR_RING, WORLD_FAR_RING, WORLD_BACKGROUND_RING]
	}

	fn rings_overlap_by_anchor_step(inner: TerrainCellRing, outer: TerrainCellRing) -> bool {
		inner.anchor_step == outer.anchor_step
			&& inner.high_outer_radius - outer.high_inner_radius + 1e-3 >= inner.anchor_step
	}

	struct HostedCell {
		center: Vec3,
		bounds: Aabb3d,
	}

	fn hosted_cells(ring: TerrainCellRing, anchor: Vec3) -> Vec<HostedCell> {
		let layout = world_cell_layout();
		ring.regions_around(anchor)
			.into_iter()
			.flat_map(|region| ring.cell_ids(region, layout.vertical_half_extent))
			.filter_map(|OriginalId(id)| {
				let bounds = id.origin_cell_bounds()?;
				let center = Vec3::from((bounds.min + bounds.max) * 0.5);
				Some(HostedCell { center, bounds })
			})
			.collect()
	}

	fn xz_contains(bounds: Aabb3d, point: Vec3) -> bool {
		point.x >= bounds.min.x
			&& point.x <= bounds.max.x
			&& point.z >= bounds.min.z
			&& point.z <= bounds.max.z
	}

	fn overlaps_any(regions: &[Aabb3d], bounds: Aabb3d) -> bool {
		regions.iter().any(|region| region.intersects(&bounds))
	}

	/// Previously covered near / far cell centers that still fall in the new
	/// near or far request must already sit on a kept near host or a kept far
	/// host. Newly generated cells on either ring are not assumed to be ready.
	fn still_wanted_centers_stay_covered(
		near: TerrainCellRing,
		far: TerrainCellRing,
		before: Vec3,
		after: Vec3,
	) -> Result<(), Vec3> {
		let near_before = hosted_cells(near, before);
		let far_before = hosted_cells(far, before);
		let near_after = near.regions_around(after);
		let far_after = far.regions_around(after);
		let kept_near: Vec<Aabb3d> = near_before
			.iter()
			.filter(|cell| overlaps_any(&near_after, cell.bounds))
			.map(|cell| cell.bounds)
			.collect();
		let kept_far: Vec<Aabb3d> = far_before
			.iter()
			.filter(|cell| overlaps_any(&far_after, cell.bounds))
			.map(|cell| cell.bounds)
			.collect();
		for cell in near_before.iter().chain(&far_before) {
			let wanted = near.retains_cell_center(cell.center, after)
				|| overlaps_any(&far_after, cell.bounds);
			if !wanted {
				continue;
			}
			let on_kept_near =
				kept_near.iter().copied().any(|bounds| xz_contains(bounds, cell.center));
			let on_kept_far =
				kept_far.iter().copied().any(|bounds| xz_contains(bounds, cell.center));
			if !on_kept_near && !on_kept_far {
				return Err(cell.center);
			}
		}
		Ok(())
	}

	#[test]
	fn world_near_high_stays_at_eight_cells() {
		assert_eq!(WORLD_FINE_HALF_EXTENT_CELLS, 8);
		assert_eq!(world_lod_bands(), vec![TerrainMeshLodBand { max_radius_cells: 8, res_2: 5 }]);
	}

	#[test]
	fn world_stream_rings_are_three_high_only_scales() {
		let layout = world_cell_layout();
		assert_eq!(layout.stream_rings.len(), 3);
		assert_eq!(layout.stream_rings[0].cell_size, TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[1].cell_size, 2.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[2].cell_size, 4.0 * TERRAIN_CELL_SIZE);
		assert_eq!(layout.stream_rings[0].res_2, 5);
		assert_eq!(layout.stream_rings[1].res_2, 4);
		assert_eq!(layout.stream_rings[2].res_2, 3);
		assert!(layout.stream_rings[0].seeds_collision());
		assert!(!layout.stream_rings[1].seeds_collision());
		assert!(!layout.stream_rings[2].seeds_collision());
		let ids = layout.cell_ids(layout.request_region());
		assert!(!ids.is_empty());
	}

	#[test]
	fn world_far_interior_is_empty_high() {
		let layout = world_cell_layout();
		let far = layout.stream_rings[1];
		assert!(!far.draws_high());
		assert!(far.draws_level(lod::LodSceneLevel::Medium));
		assert_eq!(far.level_for(Vec3::ZERO, Vec3::ZERO), lod::LodSceneLevel::High);
		assert_eq!(
			far.level_for(Vec3::X * 3.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::High
		);
		assert_eq!(
			far.level_for(Vec3::X * 5.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		assert_eq!(
			far.level_for(Vec3::X * 7.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		let background = layout.stream_rings[2];
		assert_eq!(background.level_for(Vec3::ZERO, Vec3::ZERO), lod::LodSceneLevel::High);
		assert_eq!(
			background.level_for(Vec3::X * 13.0 * TERRAIN_CELL_SIZE, Vec3::ZERO),
			lod::LodSceneLevel::Medium
		);
		assert!(background.draws_level(lod::LodSceneLevel::Medium));
	}

	#[test]
	fn world_layout_recenters_with_the_viewer() {
		let mut layout = world_cell_layout();
		let size = TERRAIN_CELL_SIZE;
		assert!(layout.recenter_on_xz(Vec3::X * 20.0 * size));
		assert_eq!(layout.origin, IVec2::new(12, -WORLD_FINE_HALF_EXTENT_CELLS));
		let ids = layout.cell_ids(layout.request_region());
		assert!(!ids.is_empty());
	}

	#[test]
	fn retarget_clears_playable_macro_bands_on_a_fine_patch() {
		let mut assets = TerrainMeshAssets {
			config: TerrainConfig::new(42),
			material: Handle::default(),
			lod_bands: world_lod_bands(),
			outer_add_walls: true,
			fine_grid_max_radius: Some(WORLD_FINE_HALF_EXTENT_CELLS),
			macro_seam_half_extents: vec![WORLD_TERRAIN_NEAR_RADIUS_M],
			macro_cell_min_size: Some(2.0 * TERRAIN_CELL_SIZE),
			macro_res_2: Some(3),
		};
		retarget_mesh_assets(&mut assets, TerrainCoverage::FinePatch, 2);
		assert_eq!(assets.lod_bands, playground_lod_bands(2));
		assert_eq!(assets.fine_grid_max_radius, Some(2));
		assert!(assets.macro_seam_half_extents.is_empty());
		assert!(assets.macro_cell_min_size.is_none());
		assert!(assets.macro_res_2.is_none());
	}

	#[test]
	fn playable_rings_overlap_by_at_least_one_anchor_step() {
		let rings = playable_rings();
		for pair in rings.windows(2) {
			assert_eq!(pair[0].anchor_step, pair[1].anchor_step);
			assert!(
				rings_overlap_by_anchor_step(pair[0], pair[1]),
				"inner outer {} minus hole {} is below step {}",
				pair[0].high_outer_radius,
				pair[1].high_inner_radius,
				pair[0].anchor_step
			);
		}
	}

	#[test]
	fn two_cell_far_hole_inset_breaks_the_overlap_invariant() {
		let mut far = WORLD_FAR_RING;
		far.high_inner_radius = WORLD_TERRAIN_NEAR_RADIUS_M - 2.0 * TERRAIN_CELL_SIZE;
		assert!(
			!rings_overlap_by_anchor_step(WORLD_NEAR_RING, far),
			"a two-cell far hole must fail the overlap invariant"
		);
	}

	#[test]
	fn one_step_anchor_jump_covers_every_still_wanted_cell_center() {
		let step = WORLD_TERRAIN_PRESENT_STEP_M;
		let jumps = [
			Vec3::X * step,
			Vec3::NEG_X * step,
			Vec3::Z * step,
			Vec3::NEG_Z * step,
			Vec3::new(step, 0.0, step),
			Vec3::new(step, 0.0, -step),
			Vec3::new(-step, 0.0, step),
			Vec3::new(-step, 0.0, -step),
		];
		for jump in jumps {
			still_wanted_centers_stay_covered(WORLD_NEAR_RING, WORLD_FAR_RING, Vec3::ZERO, jump)
				.unwrap_or_else(|center| {
					panic!("uncovered cell center {center} after jump {jump}")
				});
		}
	}

	#[test]
	fn two_cell_far_hole_inset_winks_on_a_one_step_jump() {
		let mut far = WORLD_FAR_RING;
		far.high_inner_radius = WORLD_TERRAIN_NEAR_RADIUS_M - 2.0 * TERRAIN_CELL_SIZE;
		assert!(
			still_wanted_centers_stay_covered(
				WORLD_NEAR_RING,
				far,
				Vec3::ZERO,
				Vec3::X * WORLD_TERRAIN_PRESENT_STEP_M
			)
			.is_err(),
			"a two-cell far hole must wink on a one-step jump"
		);
	}
}
