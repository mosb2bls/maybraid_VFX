//! Water mesh material used when building water instances.

use bevy::prelude::*;
use terrain_shaders::RefractionWater;

/// Material used when building water instances.
///
/// Mesh `res_2` and origin-cell bounds come from the sibling [`crate::terrain::Terrain`]
/// cell / [`crate::terrain::cell::TerrainCellLayout`] — not from this resource — so
/// water and terrain always share one cascade lattice.
#[derive(Resource, Clone)]
pub struct WaterMeshAssets {
	pub material: Handle<RefractionWater>,
}

lod::seeded_root!(WaterMeshAssets);
