//! Layer contract for generated world surfaces.
//!
//! [`OnTerrain<T>`] names the solid-ground surface of terrain model `T`.

mod contract;
mod on_terrain;

pub use contract::{
	terrain_streaming, TerrainContract, TerrainExtent, TerrainExtentKind, TerrainLayerSystems,
	TerrainStreaming,
};
pub use on_terrain::OnTerrain;
