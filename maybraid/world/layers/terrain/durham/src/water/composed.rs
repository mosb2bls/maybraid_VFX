//! Water-fill composition layer — parallel to [`crate::terrain::sdf::ComposedTerrain`].
//!
//! [`Terrain`](crate::terrain::Terrain) composes elevation mods into a heightfield SDF
//! and meshes it on the origin-cell cascade lattice
//! ([`cascade_chunk_for_cell`](crate::terrain::render::cascade_chunk_for_cell)).
//! This module is the matching **water** composition: stamp-owned [`WaterFill`]s
//! unioned against that finished heightfield into one meshable [`ComposedWater`].
//!
//! **Same sample space as terrain.** Water cells are the same origin cells as
//! [`TerrainCellLayout`](crate::terrain::cell::TerrainCellLayout), use the same
//! `res_2`, and mesh with the same cascade chunk bounds (including full cell Y).
//! Hydro fills are carve × half-space below \(W\) (see [`WaterFill`] /
//! [`HydroComplex::water_distance`](terrain_watersheds::HydroComplex::water_distance)).

use crate::terrain::sdf::TerrainSdf;
use crate::water::water_distance as fill_water_distance;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::mesh::{IdentifiedMesh, MeshId};
use render_item::NormalizeChunk;
use sdf::{Sdf, Sign, SignBoundary, SignUniformIntervals};
use terrain_watersheds::WaterFill;

/// Composed wet volume for one terrain origin cell: union of stamp fills against
/// the cell's finished heightfield.
///
/// Analogous to [`ComposedTerrain`](crate::terrain::sdf::ComposedTerrain): stamps
/// author fills; this type is the meshable product the water model presents on
/// the shared cascade chunk.
#[derive(Clone, Debug)]
pub struct ComposedWater {
	/// Finished heightfield (same instance the sibling [`Terrain`](crate::terrain::Terrain) cell composed).
	pub terrain: TerrainSdf,
	/// Stamp-owned fills whose support intersects this cell's collection pass.
	pub fills: Vec<WaterFill>,
}

impl ComposedWater {
	/// Compose stamp fills against a finished heightfield (water analogue of
	/// [`Terrain::compose_sdf`](crate::terrain::Terrain::compose_sdf)).
	pub fn compose(terrain: TerrainSdf, fills: Vec<WaterFill>) -> Self {
		Self { terrain, fills }
	}

	/// Backward-compatible alias of [`Self::compose`].
	pub fn new(terrain: TerrainSdf, fills: Vec<WaterFill>) -> Self {
		Self::compose(terrain, fills)
	}
}

impl Sdf for ComposedWater {
	fn distance(&self, p: Vec3) -> f32 {
		let h = self.terrain.height_at_with_all_modulations(p.x, p.z);
		fill_water_distance(&self.fills, p, h)
	}

	fn sign_uniform_on_y(&self, x: f32, z: f32) -> SignUniformIntervals {
		let h = self.terrain.height_at_with_all_modulations(x, z);
		let p_xz = Vec2::new(x, z);

		// Half-space below W: union of wet columns → (-∞, W_max].
		let mut wet_top = f32::NEG_INFINITY;
		let mut any_wet = false;
		for fill in &self.fills {
			if let Some((_lo, hi)) = fill.wet_y_span_at(p_xz.x, p_xz.y, h) {
				any_wet = true;
				wet_top = wet_top.max(hi);
			}
		}

		let mut intervals = SignUniformIntervals::default();
		if any_wet && wet_top.is_finite() {
			intervals
				.insert_boundary(SignBoundary { min: f32::NEG_INFINITY, sign: Sign::Negative });
			intervals.insert_boundary(SignBoundary { min: wet_top, sign: Sign::Positive });
		} else {
			intervals
				.insert_boundary(SignBoundary { min: f32::NEG_INFINITY, sign: Sign::Positive });
		}
		intervals
	}
}

impl NormalizeChunk for ComposedWater {
	fn normalize_chunk(&self, cascade_chunk: &CascadeChunk) -> CascadeChunk {
		cascade_chunk.clone()
	}
}

impl IdentifiedMesh for ComposedWater {
	fn id(&self) -> MeshId {
		MeshId::new(format!("{:?}", self))
	}
}
