//! Merge multiple [`SceneRef`] worlds into one mesh [`WorldAsset`].
//!
//! Per-part transforms are baked into `POSITION`. Kit-local positions are copied
//! into `COLOR.xyz` first so object-space materials can still see a unit kit after
//! the merge. `COLOR.w` is the part's uniform bake scale so sway can recover
//! per-kit amplitude (`instance_scale * color.w`).

mod mesh;
mod transform_key;

use bevy::asset::{AssetId, AssetServer, Handle};
use bevy::color::Color;
use bevy::ecs::reflect::AppTypeRegistry;
use bevy::mesh::Mesh;
use bevy::platform::collections::HashMap;
use bevy::prelude::{Assets, Component, Resource, StandardMaterial, Transform};
use bevy::scene::prelude::{bsn, template_value};
use bevy::scene::Scene;
use bevy::world_serialization::WorldAsset;

use crate::handles::SceneRefHandles;
use crate::scene_ref::SceneRef;
use crate::world_asset::world_asset_from_mesh;

pub use transform_key::TransformKey;

use mesh::{
	bake_source_instances, collect_source_mesh_instances, merge_meshes, SourceMeshInstance,
};

/// One input scene and its local transform in the merged multi-scene.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MultiScenePart {
	pub scene: SceneRef,
	pub transform: TransformKey,
}

impl MultiScenePart {
	pub fn new(scene: SceneRef, transform: Transform) -> Self {
		Self { scene, transform: TransformKey::new(transform) }
	}

	pub fn identity(scene: SceneRef) -> Self {
		Self { scene, transform: TransformKey::IDENTITY }
	}
}

/// Ordered list of scene parts to merge into a single-mesh [`WorldAsset`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MultiSceneMerge {
	pub parts: Vec<MultiScenePart>,
}

/// BSN / ECS root that resolves to [`WorldAssetRoot`] via [`MultiSceneMergeHandles`].
#[derive(Component, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct MultiSceneMergeRoot(pub MultiSceneMerge);

impl MultiSceneMerge {
	pub fn new(parts: impl IntoIterator<Item = MultiScenePart>) -> Self {
		Self { parts: parts.into_iter().collect() }
	}

	pub fn part(mut self, scene: SceneRef, transform: Transform) -> Self {
		self.parts.push(MultiScenePart::new(scene, transform));
		self
	}

	/// BSN scene root (`MultiSceneMergeRoot`; fulfilled to [`WorldAssetRoot`]).
	pub fn scene(self) -> impl Scene + 'static {
		bsn! {
			template_value(MultiSceneMergeRoot(self))
		}
	}

	/// Same as [`Self::scene`], with a local pose on the merge root entity.
	///
	/// Part transforms stay collection-/unit-local; `transform` places the whole merge.
	pub fn scene_at(self, transform: Transform) -> impl Scene + 'static {
		bsn! {
			template_value(MultiSceneMergeRoot(self))
			template_value(transform)
			bevy::prelude::Visibility::Inherited
		}
	}
}

/// Memoized merged [`Handle<WorldAsset>`]s keyed by [`MultiSceneMerge`].
#[derive(Resource, Default)]
pub struct MultiSceneMergeHandles {
	cache: HashMap<MultiSceneMerge, Handle<WorldAsset>>,
	/// Shared placeholder material so merged meshes enter the PBR pipeline.
	default_material: Option<Handle<StandardMaterial>>,
	/// `Mesh3d` instances inside a resolved part world (kit GLBs are reused
	/// across hundreds of collection parts). Walked once per source asset.
	source_layouts: HashMap<AssetId<WorldAsset>, Vec<SourceMeshInstance>>,
}

impl MultiSceneMergeHandles {
	/// Resolve `merge` to a cached handle when every part is ready.
	///
	/// Returns [`None`] until all part [`SceneRef`]s resolve and their mesh
	/// dependencies are loaded, or when `miss_budget` is `0` and this key is
	/// not cached yet (retry next frame). Cache hits do not consume
	/// `miss_budget`. Pipelines through [`SceneRefHandles`] so mirrors rebuild
	/// before merge.
	fn default_material(
		&mut self,
		materials: &mut Assets<StandardMaterial>,
	) -> Handle<StandardMaterial> {
		if let Some(handle) = &self.default_material {
			return handle.clone();
		}
		let handle = materials.add(StandardMaterial {
			base_color: Color::srgb(0.25, 0.55, 0.28),
			perceptual_roughness: 0.85,
			..Default::default()
		});
		self.default_material = Some(handle.clone());
		handle
	}

	fn source_layout(
		&mut self,
		handle: &Handle<WorldAsset>,
		world_assets: &Assets<WorldAsset>,
		meshes: &Assets<Mesh>,
	) -> Option<Vec<SourceMeshInstance>> {
		let id = handle.id();
		if let Some(layout) = self.source_layouts.get(&id) {
			return Some(layout.clone());
		}
		let source = world_assets.get(handle)?;
		let layout = collect_source_mesh_instances(source, meshes)?;
		self.source_layouts.insert(id, layout.clone());
		Some(layout)
	}

	pub fn try_resolve(
		&mut self,
		merge: &MultiSceneMerge,
		scene_handles: &mut SceneRefHandles,
		asset_server: &AssetServer,
		world_assets: &mut Assets<WorldAsset>,
		meshes: &mut Assets<Mesh>,
		materials: &mut Assets<StandardMaterial>,
		type_registry: &AppTypeRegistry,
		miss_budget: &mut u32,
	) -> Option<Handle<WorldAsset>> {
		if let Some(handle) = self.cache.get(merge) {
			return Some(handle.clone());
		}
		if *miss_budget == 0 {
			return None;
		}

		let mut part_handles = Vec::with_capacity(merge.parts.len());
		for part in &merge.parts {
			let handle = scene_handles.try_resolve(
				&part.scene,
				asset_server,
				world_assets,
				meshes,
				type_registry,
			)?;
			if !asset_server.is_loaded_with_dependencies(&handle) {
				return None;
			}
			part_handles.push((handle, part.transform.0));
		}

		let mut prepared = Vec::new();
		for (handle, part_transform) in &part_handles {
			let layout = self.source_layout(handle, world_assets, meshes)?;
			// Missing Mesh3d bytes → retry next frame (do not cache an empty mesh).
			bake_source_instances(&layout, *part_transform, meshes, &mut prepared)?;
		}

		let merged_mesh = merge_meshes(prepared)?;
		let mesh_handle = meshes.add(merged_mesh);
		let material = self.default_material(materials);
		let world_asset = world_asset_from_mesh(mesh_handle, material);
		let handle = world_assets.add(world_asset);
		self.cache.insert(merge.clone(), handle.clone());
		*miss_budget = miss_budget.saturating_sub(1);
		Some(handle)
	}

	/// Kick off source loads for every part (mirrored rebuild / merge still need
	/// [`Self::try_resolve`] once dependencies are ready).
	pub fn preload(
		&mut self,
		merge: &MultiSceneMerge,
		scene_handles: &mut SceneRefHandles,
		asset_server: &AssetServer,
	) {
		scene_handles.preload(merge.parts.iter().map(|p| &p.scene), asset_server);
	}

	pub fn len(&self) -> usize {
		self.cache.len()
	}

	pub fn is_empty(&self) -> bool {
		self.cache.is_empty()
	}
}
