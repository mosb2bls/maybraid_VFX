//! Terrain mesh LOD bands and universal generation assets.

use crate::terrain::cell::{expand_aabb_xz, TerrainCellLayout};
use crate::terrain::config::TerrainConfig;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use render_item::sdf::cpu_shot::WallFaces;
use terrain_shaders::TerrainShader;

/// One concentric mesh-LOD band on the fine (base-sized) cell grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerrainMeshLodBand {
	/// Inclusive Chebyshev cell-index radius for this band.
	pub max_radius_cells: i32,
	/// Cascade `res_2` (`2^res_2` samples along each axis).
	pub res_2: u8,
}

/// Config / material / mesh resolution used when building terrain instances.
///
/// Materialized once under [`Id::Universal`] via [`lod::gen::GenerationScheme`].
///
/// Fine-grid LOD: first [`TerrainMeshLodBand`] with `radius ≤ max_radius_cells`
/// wins ([`Self::lod_bands`] must be sorted ascending by radius). Radii past the
/// last band reuse that band's `res_2`.
///
/// When [`Self::outer_add_walls`] is set, CpuShot skirts are emitted on faces
/// shared with a neighbor whose LOD `res_2` differs, and on nested macro
/// seams listed in [`Self::macro_seam_half_extents`].
///
/// Cells whose XZ edge is at least [`Self::macro_cell_min_size`] skip the fine
/// bands and use [`Self::macro_res_2`] (macro outer-ring tiles).
#[derive(Resource, Clone)]
pub struct TerrainMeshAssets {
	pub config: TerrainConfig,
	pub material: Handle<TerrainShader>,
	/// Concentric fine-grid LOD bands (ascending `max_radius_cells`).
	pub lod_bands: Vec<TerrainMeshLodBand>,
	/// When true, enable per-face CpuShot walls on LOD / fine–macro boundaries.
	pub outer_add_walls: bool,
	/// Inclusive Chebyshev radius of the fine grid (for fine→macro wall faces).
	pub fine_grid_max_radius: Option<i32>,
	/// World half-extents of nested footprints that macro faces may abut
	/// (fine edge, 2×→4× edge, …), for macro→inner wall faces.
	pub macro_seam_half_extents: Vec<f32>,
	/// XZ edge length at/above which a cell is treated as a macro outer tile.
	pub macro_cell_min_size: Option<f32>,
	/// Mesh resolution for macro outer tiles. Defaults to 3 when unset.
	pub macro_res_2: Option<u8>,
}

impl TerrainMeshAssets {
	fn res_2_for_radius(&self, radius: i32) -> u8 {
		for band in &self.lod_bands {
			if radius <= band.max_radius_cells {
				return band.res_2;
			}
		}
		self.lod_bands.last().map(|b| b.res_2).unwrap_or(0)
	}

	fn wall_toward_neighbor(&self, my_r: i32, my_res: u8, n_r: i32) -> bool {
		if self.res_2_for_radius(n_r) != my_res {
			return true;
		}
		if let Some(fine_max) = self.fine_grid_max_radius {
			let my_in = my_r <= fine_max;
			let n_in = n_r <= fine_max;
			if my_in != n_in {
				return true;
			}
		}
		false
	}

	fn wall_faces_for_fine_cell(&self, ix: i32, iz: i32, layout: &TerrainCellLayout) -> WallFaces {
		if !self.outer_add_walls || self.lod_bands.is_empty() {
			return WallFaces::NONE;
		}
		let my_r = layout.fine_cell_radius(ix, iz);
		let mine = self.res_2_for_radius(my_r);
		WallFaces {
			neg_x: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix - 1, iz)),
			pos_x: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix + 1, iz)),
			neg_z: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix, iz - 1)),
			pos_z: self.wall_toward_neighbor(my_r, mine, layout.fine_cell_radius(ix, iz + 1)),
		}
	}

	fn macro_inner_footprints(layout: &TerrainCellLayout) -> Vec<Aabb3d> {
		let mut inners = vec![layout.fine_request_region()];
		let mut covered = layout.fine_request_region();
		for (i, outer) in layout.outer_rings.iter().enumerate() {
			if outer.rows <= 0 {
				continue;
			}
			covered = expand_aabb_xz(covered, outer.rows as f32 * outer.cell_size.max(1e-3));
			if i + 1 < layout.outer_rings.len() {
				inners.push(covered);
			}
		}
		inners
	}

	fn wall_faces_for_macro_cell(&self, bounds: Aabb3d, layout: &TerrainCellLayout) -> WallFaces {
		if !self.outer_add_walls {
			return WallFaces::NONE;
		}
		let inners = Self::macro_inner_footprints(layout);
		if inners.is_empty() {
			return WallFaces::ALL;
		}
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let eps = 1.0;
		let mut faces = WallFaces::NONE;
		for inner in inners {
			let inner_min = Vec3::from(inner.min);
			let inner_max = Vec3::from(inner.max);
			faces.neg_x |= (min.x - inner_max.x).abs() < eps || (min.x - inner_min.x).abs() < eps;
			faces.pos_x |= (max.x - inner_min.x).abs() < eps || (max.x - inner_max.x).abs() < eps;
			faces.neg_z |= (min.z - inner_max.z).abs() < eps || (min.z - inner_min.z).abs() < eps;
			faces.pos_z |= (max.z - inner_min.z).abs() < eps || (max.z - inner_max.z).abs() < eps;
		}
		faces
	}

	fn wall_faces_for_stream_cell(
		&self,
		layout: &TerrainCellLayout,
		ring: crate::terrain::cell::TerrainCellRing,
	) -> WallFaces {
		if !self.outer_add_walls {
			return WallFaces::NONE;
		}
		// Inner holes and Near/Far rims sit under the next-finer stream (draw
		// overlap). Only the outermost Background skirt faces empty space, and
		// any of its cells can be on the edge wherever the stream is anchored.
		if layout.is_outermost_stream_ring(ring) {
			WallFaces::ALL
		} else {
			WallFaces::NONE
		}
	}

	/// `(res_2, wall_faces)` for a terrain origin cell AABB.
	///
	/// Fine-grid LOD radius is Chebyshev distance from the current layout window
	/// center, so newly admitted cells use the sliding stream's bands.
	pub fn mesh_params_for_cell(
		&self,
		bounds: Aabb3d,
		layout: &TerrainCellLayout,
	) -> (u8, WallFaces) {
		let min = Vec3::from(bounds.min);
		let max = Vec3::from(bounds.max);
		let cell_size = (max.x - min.x).max(1e-3);
		if let Some(ring) = layout.stream_ring_for_cell_size(cell_size) {
			return (ring.res_2, self.wall_faces_for_stream_cell(layout, ring));
		}
		if let Some(macro_min) = self.macro_cell_min_size {
			if cell_size + 1e-3 >= macro_min {
				return (
					self.macro_res_2.unwrap_or(3),
					self.wall_faces_for_macro_cell(bounds, layout),
				);
			}
		}
		let ix = (min.x / cell_size).floor() as i32;
		let iz = (min.z / cell_size).floor() as i32;
		let radius = layout.fine_cell_radius(ix, iz);
		(self.res_2_for_radius(radius), self.wall_faces_for_fine_cell(ix, iz, layout))
	}
}

lod::seeded_root!(TerrainMeshAssets);
