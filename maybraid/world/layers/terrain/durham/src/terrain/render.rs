//! [`RenderItem`] that meshes terrain cells via SDF sampling.

use crate::terrain::cell::{expand_aabb_xz_y, TERRAIN_MESH_PAD_VOXELS, TERRAIN_MESH_PAD_Y_SLOPE};
use crate::terrain::sdf::ComposedTerrain;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::*;
use chunk::cascade::CascadeChunk;
use render_item::mesh::cache::handle::map::HandleMap;
use render_item::mesh::cache::mesh::disk::DiskMeshCache;
use render_item::mesh::handle::MeshHandle;
use render_item::{mesh::MeshDispatch, RenderItem};

/// Dispatches SDF mesh generation for a composed Durham terrain.
#[derive(Component, Debug, Clone)]
pub struct TerrainRenderItem<M: Material> {
	pub sdf: ComposedTerrain,
	pub material: MeshMaterial3d<M>,
	pub handle_map: HandleMap<ComposedTerrain>,
	pub mesh_cache: Option<DiskMeshCache<ComposedTerrain>>,
}

impl<M: Material> TerrainRenderItem<M> {
	pub fn new(sdf: ComposedTerrain, material: MeshMaterial3d<M>) -> Self {
		Self { sdf, material, handle_map: HandleMap::new(), mesh_cache: None }
	}

	pub fn with_handle_map(mut self, handle_map: HandleMap<ComposedTerrain>) -> Self {
		self.handle_map = handle_map;
		self
	}

	pub fn with_mesh_cache(mut self, mesh_cache: Option<DiskMeshCache<ComposedTerrain>>) -> Self {
		self.mesh_cache = mesh_cache;
		self
	}
}

impl<M: Material> RenderItem for TerrainRenderItem<M> {
	fn spawn_render_items(
		&self,
		commands: &mut Commands,
		cascade_chunk: &CascadeChunk,
		_transform: Transform,
	) -> Vec<Entity> {
		log::debug!("Spawning terrain render items for cascade chunk: {:?}", cascade_chunk);

		let transform = Transform::from_translation(cascade_chunk.origin);
		let mesh_handle = MeshHandle::new(self.sdf.clone())
			.with_handle_cache(self.handle_map.clone())
			.with_mesh_cache(self.mesh_cache.clone());
		let entity = commands
			.spawn((
				cascade_chunk.clone(),
				MeshDispatch::new(mesh_handle),
				transform,
				Visibility::default(),
				self.material.clone(),
			))
			.id();
		vec![entity]
	}
}

/// Build a cascade chunk covering a terrain cell AABB for SDF meshing.
///
/// Pads XZ by [`TERRAIN_MESH_PAD_VOXELS`] sample pitches and Y by that times
/// [`TERRAIN_MESH_PAD_Y_SLOPE`] so steep ridges still overlap across cell faces.
pub fn cascade_chunk_for_cell(bounds: Aabb3d, res_2: u8) -> CascadeChunk {
	let min0 = Vec3::from(bounds.min);
	let max0 = Vec3::from(bounds.max);
	let base = max0 - min0;
	let res = 2_f32.powi(res_2 as i32).max(1.0);
	let xz_pitch = base.x.min(base.z) / res;
	let pad_xz = xz_pitch * TERRAIN_MESH_PAD_VOXELS;
	let pad_y = pad_xz * TERRAIN_MESH_PAD_Y_SLOPE;
	let padded = expand_aabb_xz_y(bounds, pad_xz, pad_y);
	cascade_chunk_from_aabb(padded, res_2)
}

fn cascade_chunk_from_aabb(bounds: Aabb3d, res_2: u8) -> CascadeChunk {
	let min = Vec3::from(bounds.min);
	let max = Vec3::from(bounds.max);
	let extent = max - min;
	CascadeChunk {
		world: 0,
		origin: min,
		size: extent.max_element(),
		extent: Some(extent),
		res_2,
		omit: None,
	}
}
