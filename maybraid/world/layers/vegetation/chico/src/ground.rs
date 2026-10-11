//! Chunk identity for padded ground cells Chico overlays.

use bevy::math::Vec3;
use durham::{cascade_chunk_for_cell, TerrainMeshBuilder, TERRAIN_CELL_SIZE};
use lod_cascade::Chunk;
use richmond::TerrainWithPads;
use terrain_chunk_ref::TerrainChunkRef;

/// Fine overlay cells match Durham terrain width.
pub fn fine_overlay_size() -> f32 {
	TERRAIN_CELL_SIZE
}

/// Overlay chunk identity for one padded surface cell.
pub fn overlay_chunk_ref(surface: &TerrainWithPads) -> TerrainChunkRef<TerrainMeshBuilder> {
	let (origin, extent) = cascade_chunk(surface);
	let chunk = Chunk::from_min_max(origin, origin + extent, None);
	TerrainChunkRef::new(surface.mesh_builder(), chunk, surface.res_2)
}

fn cascade_chunk(surface: &TerrainWithPads) -> (Vec3, Vec3) {
	let cascade = cascade_chunk_for_cell(surface.cell, surface.res_2);
	let extent = cascade.extent.unwrap_or(Vec3::splat(cascade.size));
	(cascade.origin, extent)
}

#[cfg(test)]
mod tests {
	use std::sync::Arc;

	use bevy::math::bounding::Aabb3d;
	use bevy::math::Vec3;
	use bevy::prelude::World;
	use durham::{
		BaseTerrainNoise, Terrain, TerrainCellLayout, TerrainConfig, TerrainMeshBuilder,
	};
	use lod::gen::Id;
	use lod::hcsg::HcsgStorage;
	use lod_cascade::Chunk;
	use terrain_chunk_ref::TerrainChunkRef;

	use super::{fine_overlay_size, overlay_chunk_ref};

	#[test]
	fn chunk_ref_matches_cascade_chunk_for_cell() -> anyhow::Result<()> {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(7));
		world.init_resource::<HcsgStorage>();
		let terrain = Terrain::base_cell_for_test(&layout, 0, 0, base);
		let cell = terrain.cell;
		world
			.resource::<HcsgStorage>()
			.publish(Id::from_cell(cell), Arc::new(terrain), cell);
		let store = world.resource::<HcsgStorage>();
		let probe = Aabb3d::from_min_max(Vec3::new(1.0, -10.0, 1.0), Vec3::new(2.0, 10.0, 2.0));
		let id = store
			.try_overlapping::<durham::Terrain>(probe)
			.map_err(|busy| anyhow::anyhow!("HcsgStorage busy: {busy:?}"))?
			.into_iter()
			.next()
			.ok_or_else(|| anyhow::anyhow!("stored cell"))?;
		let terrain = store
			.try_entry::<durham::Terrain>(id)
			.map_err(|busy| anyhow::anyhow!("HcsgStorage busy: {busy:?}"))?
			.map(|entry| entry.value)
			.ok_or_else(|| anyhow::anyhow!("terrain"))?;
		let padded = richmond::TerrainWithPads::compose(terrain.as_ref(), []);
		let built = overlay_chunk_ref(&padded);
		let cascade = durham::cascade_chunk_for_cell(padded.cell, padded.res_2);
		let extent = match cascade.extent {
			Some(extent) => extent,
			None => Vec3::splat(cascade.size),
		};
		let chunk = Chunk::from_min_max(cascade.origin, cascade.origin + extent, None);
		let manual =
			TerrainChunkRef::<TerrainMeshBuilder>::new(padded.mesh_builder(), chunk, padded.res_2);
		assert_eq!(built.key(), manual.key());
		Ok(())
	}

	#[test]
	fn bump_out_cells_match_terrain_rings() {
		assert!((crate::BUMP_OUT_CELL_XZ - fine_overlay_size()).abs() < 1e-3);
		assert!((crate::MEDIUM_BUMP_OUT_CELL_XZ - 2.0 * fine_overlay_size()).abs() < 1e-3);
	}
}
