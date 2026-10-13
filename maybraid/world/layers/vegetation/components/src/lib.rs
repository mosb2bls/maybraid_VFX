//! Reusable Chico vegetation scene components.
//!
//! Per domain: geometry + [`Placement`] → node (`LodScene`).

pub mod assets;
pub mod foliage;
pub mod layer;
pub mod lod_band;
pub mod lod_host;
pub mod materials;
pub mod placed;
pub mod placed_vegetation;
pub mod procedural;
pub mod scene_children;
pub mod sticks;
pub mod structural_lod;

pub use assets::AssetPath;
pub use foliage::{
	update_foliage_host_levels, CheapBallCollection, CollectionPresent, FoliageGeometry,
	FoliageLodProbe, FoliageNode, FrondCollection, FrondKit, FrondMember, FrondRun,
	CHEAP_BALL_COLLECTION_HIGH_METERS, CHEAP_BALL_COLLECTION_LOW_METERS,
	CHEAP_BALL_COLLECTION_MEDIUM_METERS, COLLECTION_HIGH_METERS, COLLECTION_LOW_METERS,
	COLLECTION_MEDIUM_METERS, FOLIAGE_HIGH_FACTOR, FOLIAGE_LOW_FACTOR, FOLIAGE_MEDIUM_FACTOR,
	FROND_COLLECTION_HIGH_FACTOR, FROND_COLLECTION_HIGH_METERS, FROND_COLLECTION_LOW_FACTOR,
	FROND_COLLECTION_LOW_METERS, FROND_COLLECTION_MEDIUM_FACTOR, FROND_COLLECTION_MEDIUM_METERS,
};
pub use layer::{Layer, Layers};
pub use lod_host::{
	posed_foliage_multi_scene_merge, posed_frond_multi_scene_merge, posed_material_asset_tier,
};
pub use materials::{
	frond_material_ref, leaf_material_ref, stick_material_ref, FROND_MATERIAL, LEAF_MATERIAL,
	STICK_MATERIAL,
};
pub use placed::Placement;
pub use placed_vegetation::{PlacedVegetation, VegetationInstance};
pub use procedural::{
	VegetationProceduralAssets, VegetationProceduralPlugin, FROND_KIT_HALF_X, STICK_KIT_HALF,
};
pub use scene_children::{pose, posed_mesh, posed_mesh_material_ref, scene_children, with_pose};
pub use sticks::{
	update_stick_host_levels, StickCollection, StickGeometry, StickLodProbe, StickMember,
	StickNode, STICK_COLLECTION_HIGH_METERS, STICK_COLLECTION_LOW_METERS,
	STICK_COLLECTION_MEDIUM_METERS, STICK_HIGH_FACTOR, STICK_LOW_FACTOR, STICK_MEDIUM_FACTOR,
};
pub use structural_lod::{
	StructuralLod, STRUCTURAL_HIGH_FACTOR, STRUCTURAL_LOW_FACTOR, STRUCTURAL_MEDIUM_FACTOR,
};

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::prelude::{Commands, CommandsSceneExt, Component, Entity, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::{cull_offset_bands_from_factor, lod_host_scene_pending, SceneChunk};

/// Domain IR exposed by a tree (or vegetation part) for structural composition.
pub trait VegetationComponents {
	fn stick_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StickNode> {
		Layers::new()
	}

	fn foliage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FoliageNode> {
		Layers::new()
	}

	/// When set, drives structural [`LodScene`] banding / bounds for host wrappers.
	fn structural_lod(&self) -> Option<StructuralLod> {
		None
	}
}

impl<T: VegetationComponents + ?Sized> VegetationComponents for &T {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		(**self).stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		(**self).foliage_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		(**self).structural_lod()
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> VegetationComponents for std::sync::Arc<T> {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		(**self).stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		(**self).foliage_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		(**self).structural_lod()
	}
}

/// Newtype: present a [`VegetationComponents`] value as an [`LodScene`] host component.
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ComponentsOnly<T: Send + Sync + 'static>(pub T);

impl<T: Send + Sync + 'static> ComponentsOnly<T> {
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T: Send + Sync + 'static> From<T> for ComponentsOnly<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T: Send + Sync + 'static> std::ops::Deref for ComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> VegetationComponents for ComponentsOnly<T> {
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		self.0.stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.0.foliage_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		self.0.structural_lod()
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> LodScene for ComponentsOnly<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.0
			.structural_lod()
			.map(|p| p.level_for(lod_ref.current_transform))
			.unwrap_or(LodSceneLevel::High)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match self.0.structural_lod() {
			Some(band) => band.status_for_lod_ref(lod_ref),
			None => LodSceneStatus::Unchanged,
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		match self.0.structural_lod() {
			Some(band) => {
				let factor = lod_ref.current_transform.translation.distance(band.center)
					/ band.tree_radius.max(1e-4);
				cull_offset_bands_from_factor(
					factor,
					band.high_factor,
					band.medium_factor,
					band.low_factor,
				)
			}
			None => LodSceneCulls::None,
		}
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		component_only_scene(&self.0, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		vegetation_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.0
			.structural_lod()
			.map(|p| p.footprint_aabb())
			.unwrap_or_else(|| vegetation_bounds(&self.0))
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		// Pending structural host: chunks nest fine-phase stick/foliage hosts.
		lod_host_scene_pending(level, self.scene_bounds())
	}
}

/// Same as [`ComponentsOnly`], but kit nodes spawn as posed content (no nested
/// [`FoliageNode`] / [`StickNode`] LOD hosts).
///
/// One Avian volume per plant. Unmerged kits keep instance [`Transform`]s;
/// merged collections pack kit-local into vertex color so leaf breakup and
/// frond sway still work after [`scene_ref::MultiSceneMerge`].
#[derive(Debug, Clone, PartialEq, Component)]
pub struct FlattenedComponentsOnly<T: Send + Sync + 'static>(pub T);

impl<T: Send + Sync + 'static> FlattenedComponentsOnly<T> {
	pub fn into_inner(self) -> T {
		self.0
	}
}

impl<T: Send + Sync + 'static> From<T> for FlattenedComponentsOnly<T> {
	fn from(value: T) -> Self {
		Self(value)
	}
}

impl<T: Send + Sync + 'static> std::ops::Deref for FlattenedComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> VegetationComponents
	for FlattenedComponentsOnly<T>
{
	fn stick_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StickNode> {
		self.0.stick_nodes_for_level(level)
	}

	fn foliage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FoliageNode> {
		self.0.foliage_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<StructuralLod> {
		self.0.structural_lod()
	}
}

impl<T: VegetationComponents + Send + Sync + 'static> LodScene for FlattenedComponentsOnly<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.0
			.structural_lod()
			.map(|p| p.level_for(lod_ref.current_transform))
			.unwrap_or(LodSceneLevel::High)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match self.0.structural_lod() {
			Some(band) => band.status_for_lod_ref(lod_ref),
			None => LodSceneStatus::Unchanged,
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		match self.0.structural_lod() {
			Some(band) => {
				let factor = lod_ref.current_transform.translation.distance(band.center)
					/ band.tree_radius.max(1e-4);
				cull_offset_bands_from_factor(
					factor,
					band.high_factor,
					band.medium_factor,
					band.low_factor,
				)
			}
			None => LodSceneCulls::None,
		}
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		flattened_component_scene(&self.0, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		flattened_vegetation_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.0
			.structural_lod()
			.map(|p| p.footprint_aabb())
			.unwrap_or_else(|| vegetation_bounds(&self.0))
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		lod_host_scene_pending(self.scene_lod_level(lod_ref), self.scene_bounds())
	}
}

/// Weighted chunks for one structural level: each stick/foliage node is a nested LOD host.
pub fn vegetation_scene_chunks(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let mut chunks = Vec::new();
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	if chunks.is_empty() {
		SceneChunk::primitive(scene_children(Vec::new()))
	} else {
		SceneChunk::chunks(chunks)
	}
}

/// Drain weight for one posed kit ([`scene_ref::SceneRef`] + later WorldAsset admit).
///
/// Weight 1 treated a GLB instance like an empty transform; vast-orchard hitches
/// were 512 kits per frame at the spawn cap.
pub const FLATTENED_KIT_CHUNK_WEIGHT: u32 = 4;

/// Low / UltraLow canopy proxies as posed kit content (no nested [`LodScene`] hosts).
///
/// Cheap balls fold through [`FoliageNode::merge_canopy_proxies`] so a grove tile
/// is one (or a few) [`FLATTENED_KIT_CHUNK_WEIGHT`] kits, not one host per plant.
/// Sticks, if any, are flattened the same way and not hosted.
///
/// Stick UltraLow is an empty kit. Proxy trunks still need the Low trunk GLB, so
/// sticks present at Low when the grove band is UltraLow.
pub fn flattened_canopy_proxy_chunks(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let stick_level = match level {
		LodSceneLevel::UltraLow | LodSceneLevel::Distance(_) | LodSceneLevel::Resolution(_) => {
			LodSceneLevel::Low
		}
		other => other,
	};
	let sticks = vegetation.stick_nodes_for_level(stick_level).flatten();
	let foliage =
		FoliageNode::merge_canopy_proxies(vegetation.foliage_nodes_for_level(level).flatten());
	let n = sticks.len() + foliage.len();
	if n == 0 {
		return SceneChunk::primitive(scene_children(Vec::new()));
	}

	let kit_w = FLATTENED_KIT_CHUNK_WEIGHT;
	let mut chunks = Vec::with_capacity(n);
	for node in sticks {
		chunks.push(SceneChunk::weighted(kit_w, node.scene_with_level(lod_ref, stick_level)));
	}
	for node in foliage {
		chunks.push(SceneChunk::weighted(kit_w, node.scene_with_level(lod_ref, level)));
	}
	if chunks.len() == 1 {
		chunks
			.pop()
			.unwrap_or_else(|| SceneChunk::primitive(scene_children(Vec::new())))
	} else {
		SceneChunk::chunks(chunks)
	}
}

/// Append every domain node from `vegetation` at `level` as nested [`LodScene`] hosts.
///
/// Each child is embedded via [`LodScene::host`] (pending host + typed component).
pub fn append_component_scenes(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
}

/// Scene whose children are nested stick/foliage [`LodScene`] hosts at `level`.
pub fn component_only_scene(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(vegetation, lod_ref, level, &mut children);
	scene_children(children)
}

/// Append posed kit *content* (GLB / merge scenes), not nested [`LodScene`] hosts.
pub fn append_flattened_component_scenes(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in vegetation.stick_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_level(lod_ref, level)));
	}
	for node in vegetation.foliage_nodes_for_level(level).flatten() {
		children.push(Box::new(node.scene_with_level(lod_ref, level)));
	}
}

/// All kit content for `level` as siblings under one parent (no fine-phase hosts).
pub fn flattened_component_scene(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_flattened_component_scenes(vegetation, lod_ref, level, &mut children);
	scene_children(children)
}

/// Weighted chunks: one posed kit per stick/foliage node (no nested LOD hosts).
///
/// Kits are produced lazily so begin does not box every `scene_with_level` up front.
/// Each kit costs [`FLATTENED_KIT_CHUNK_WEIGHT`] so drain does not admit a full
/// SceneRef / instance wave in one frame.
pub fn flattened_vegetation_scene_chunks(
	vegetation: &impl VegetationComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let sticks = vegetation.stick_nodes_for_level(level).flatten();
	let foliage = vegetation.foliage_nodes_for_level(level).flatten();
	let n = sticks.len() + foliage.len();
	if n == 0 {
		return SceneChunk::primitive(scene_children(Vec::new()));
	}

	let prev = *lod_ref.previous_transform;
	let curr = *lod_ref.current_transform;
	let bounds = *lod_ref.bounds;
	let entity = lod_ref.entity;
	let n_sticks = sticks.len();
	let mut index = 0usize;

	let kit_w = FLATTENED_KIT_CHUNK_WEIGHT;
	SceneChunk::lazy(n as u32 * kit_w, n, move || {
		let kit_lod =
			LodRef { entity, previous_transform: &prev, current_transform: &curr, bounds: &bounds };
		if index < n_sticks {
			let node = &sticks[index];
			index += 1;
			return Some(SceneChunk::weighted(kit_w, node.scene_with_level(&kit_lod, level)));
		}
		let fi = index - n_sticks;
		if fi < foliage.len() {
			let node = &foliage[fi];
			index += 1;
			return Some(SceneChunk::weighted(kit_w, node.scene_with_level(&kit_lod, level)));
		}
		None
	})
}

/// Nest a [`ComponentsOnly`] host (pending level roots + typed component).
///
/// Uses [`template`] so `T` need not implement [`Default`] (unlike [`LodScene::host`]).
pub fn components_only_host<T>(vegetation: T, lod_ref: &LodRef) -> impl Scene + 'static
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let host = ComponentsOnly(vegetation);
	let level = host.scene_lod_level(lod_ref);
	let bounds = host.scene_bounds();
	let host_for_template = host.clone();
	(
		lod_host_scene_pending(level, bounds),
		bsn! {
			template(move |_ctx| Ok(host_for_template.clone()))
		},
	)
}

/// Nest a [`FlattenedComponentsOnly`] host (pending level roots + typed component).
pub fn flattened_components_only_host<T>(vegetation: T, lod_ref: &LodRef) -> impl Scene + 'static
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let host = FlattenedComponentsOnly(vegetation);
	let level = host.scene_lod_level(lod_ref);
	let bounds = host.scene_bounds();
	let anchor = (bounds.min + bounds.max) * 0.5;
	let radius = ((bounds.max.x - bounds.min.x).max(bounds.max.z - bounds.min.z) * 0.5).max(0.05);
	let instance = VegetationInstance::new(anchor.into(), radius);
	let host_for_template = host.clone();
	(
		lod_host_scene_pending(level, bounds),
		bsn! {
			template_value(instance)
			template(move |_ctx| Ok(host_for_template.clone()))
		},
	)
}

/// Spawn a [`ComponentsOnly`] vegetation host; chunk fulfill streams the first level.
pub fn spawn_vegetation_components<T>(
	commands: &mut Commands,
	vegetation: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(vegetation.clone());
	let level = host.scene_lod_level(&lod_ref);
	let pending = lod_host_scene_pending(level, bounds);
	let entity = commands
		.spawn_scene((
			pending,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.id();
	commands.entity(entity).insert(host);
	vec![entity]
}

/// Spawn a [`FlattenedComponentsOnly`]`<`[`PlacedVegetation`]`<`[`std::sync::Arc`]`<T>>>` host.
///
/// Isolated `/show` / `/render` plants use this family so they share grove plant hosts.
pub fn spawn_flattened_placed_vegetation<T>(
	commands: &mut Commands,
	vegetation: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: VegetationComponents + Clone + Send + Sync + 'static,
{
	let host = FlattenedComponentsOnly(PlacedVegetation::identity(std::sync::Arc::new(
		vegetation.clone(),
	)));
	spawn_lod_scene_host(commands, &host, transform, bounds)
}

/// Spawn a typed [`LodScene`] host (grove roots that nest flattened plant hosts).
///
/// Isolated `/show` uses a throwaway identity viewer (grove at the origin).
/// Forest present must use [`spawn_lod_scene_host_with_lod_ref`] so the first
/// [`lod::LodSceneLevel`] is the camera band, not distance-to-origin.
pub fn spawn_lod_scene_host<T>(
	commands: &mut Commands,
	host: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: LodScene + Component + Clone + Send + Sync + 'static,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	spawn_lod_scene_host_with_lod_ref(commands, host, transform, bounds, &lod_ref)
}

/// Like [`spawn_lod_scene_host`], stamping desired from `lod_ref` (the viewer).
pub fn spawn_lod_scene_host_with_lod_ref<T>(
	commands: &mut Commands,
	host: &T,
	transform: Transform,
	bounds: Aabb3d,
	lod_ref: &LodRef,
) -> Vec<Entity>
where
	T: LodScene + Component + Clone + Send + Sync + 'static,
{
	let level = host.scene_lod_level(lod_ref);
	let pending = lod_host_scene_pending(level, bounds);
	let entity = commands
		.spawn_scene((
			pending,
			bsn! {
				template_value(transform)
				Visibility::default()
			},
		))
		.id();
	commands.entity(entity).insert(host.clone());
	vec![entity]
}

/// Approximate AABB from stick/foliage placements at High (for adapter LodRef bounds).
pub fn vegetation_bounds(vegetation: &impl VegetationComponents) -> Aabb3d {
	let mut min = bevy::math::Vec3::splat(f32::INFINITY);
	let mut max = bevy::math::Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	for node in vegetation.stick_nodes_for_level(LodSceneLevel::High).flatten() {
		if let Some(collection) = &node.collection {
			if let Some((cmin, cmax)) = collection.aabb() {
				min = min.min(cmin);
				max = max.max(cmax);
				any = true;
				continue;
			}
		}
		let c = crate::lod_band::placement_center(&node.placement);
		let e = crate::lod_band::characteristic_extent_abs(&node.placement);
		min = min.min(c - bevy::math::Vec3::splat(e));
		max = max.max(c + bevy::math::Vec3::splat(e));
		any = true;
	}
	for node in vegetation.foliage_nodes_for_level(LodSceneLevel::High).flatten() {
		if let Some(collection) = node.geometry.as_frond_collection() {
			if let Some((cmin, cmax)) = collection.aabb() {
				min = min.min(cmin);
				max = max.max(cmax);
				any = true;
				continue;
			}
		}
		if let Some(collection) = node.geometry.as_cheap_ball_collection() {
			if let Some((cmin, cmax)) = collection.aabb() {
				min = min.min(cmin);
				max = max.max(cmax);
				any = true;
				continue;
			}
		}
		let c = crate::lod_band::placement_center(&node.placement);
		let e = crate::lod_band::characteristic_extent_abs(&node.placement);
		min = min.min(c - bevy::math::Vec3::splat(e));
		max = max.max(c + bevy::math::Vec3::splat(e));
		any = true;
	}
	if any {
		Aabb3d::from_min_max(min, max)
	} else {
		Aabb3d::from_min_max(bevy::math::Vec3::ZERO, bevy::math::Vec3::ONE)
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::prelude::{Entity, Transform, Vec3};

	#[derive(Clone)]
	struct TwoBalls;

	impl VegetationComponents for TwoBalls {
		fn foliage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FoliageNode> {
			Layers::from_free(vec![
				FoliageNode::cheap_ball(Placement::foliage_uniform(Vec3::ZERO, 0.3)),
				FoliageNode::cheap_ball(Placement::foliage_uniform(Vec3::Y, 0.3)),
			])
		}
	}

	#[test]
	fn flattened_chunks_charge_kit_scene_ref_weight() {
		let camera = Transform::from_translation(Vec3::new(0.0, 2.0, 8.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		let SceneChunk::Lazy { remaining_primitives, remaining_weight, .. } =
			flattened_vegetation_scene_chunks(&TwoBalls, &lod_ref, LodSceneLevel::High)
		else {
			panic!("expected lazy kit producer");
		};
		assert_eq!(remaining_primitives, 2);
		assert_eq!(remaining_weight, 2 * FLATTENED_KIT_CHUNK_WEIGHT);
		let SceneChunk::SubChunks(nested) =
			vegetation_scene_chunks(&TwoBalls, &lod_ref, LodSceneLevel::High)
		else {
			panic!("expected one nested host chunk per ball");
		};
		assert_eq!(nested.len(), 2);
	}

	#[test]
	fn canopy_proxy_chunks_merge_balls_and_do_not_host() {
		let camera = Transform::from_translation(Vec3::new(0.0, 2.0, 8.0));
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &camera,
			current_transform: &camera,
			bounds: &bounds,
		};
		let SceneChunk::Primitive { weight, .. } =
			flattened_canopy_proxy_chunks(&TwoBalls, &lod_ref, LodSceneLevel::Low)
		else {
			panic!("expected one merged cheap-ball kit");
		};
		assert_eq!(weight, FLATTENED_KIT_CHUNK_WEIGHT);
	}

	#[test]
	fn merge_canopy_proxies_keeps_stick_and_leaf_recipes() {
		use crate::materials::{leaf_material_ref, stick_material_ref};
		let nodes = vec![
			FoliageNode::cheap_ball(Placement::foliage_uniform(Vec3::ZERO, 0.4))
				.with_material(stick_material_ref()),
			FoliageNode::cheap_ball(Placement::foliage_uniform(Vec3::Y, 0.8))
				.with_material(leaf_material_ref()),
		];
		let merged = FoliageNode::merge_canopy_proxies(nodes);
		assert_eq!(merged.len(), 2);
		let names: Vec<_> = merged.iter().map(|n| n.material.name.clone()).collect();
		assert!(names.contains(&material_ref::MaterialId::named(STICK_MATERIAL)));
		assert!(names.contains(&material_ref::MaterialId::named(LEAF_MATERIAL)));
	}
}
