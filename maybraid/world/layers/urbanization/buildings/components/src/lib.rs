//! Reusable Richmond building scene components.
//!
//! Per domain: [`style`](floors::FloorStyle) + geometry + [`Placement`] → node (`LodScene`).

pub mod arc_kit;
pub mod assets;
pub mod doors;
pub mod floors;
pub mod furniture;
pub mod joints;
pub(crate) mod kit_merge;
pub mod labels;
pub mod layer;
pub mod lod_band;
pub mod lod_host_helper;
pub mod massing;
pub mod math;
pub mod panels;
pub mod parent_confines;
pub mod partitions;
pub mod placed;
pub mod roofs;
pub mod scene_children;
pub mod stairs;
pub mod structural_probe;

pub use arc_kit::{arc_ring_dir, arc_ring_dir_deg, decompose_arc_sweep, ArcKit};
pub use assets::AssetPath;
pub use doors::DoorNode;
pub use floors::FloorNode;
pub use furniture::{
	FurnitureAbutment, FurnitureGeometry, FurnitureNode, FurnitureStyle, FurnitureUsage,
	FurnitureUsageNode, FurnitureWireframePlugin,
};
pub use joints::{JointGeometry, JointNode, JointStyle};
pub use labels::{LabelGeometry, LabelNode, LabelStyle, LabelWireframePlugin};
pub use layer::{Layer, Layers};
pub use lod_band::{placement_bounds, warm_mesh_lod_culls, warm_mesh_lod_culls_at_depth};
pub use lod_host_helper::LodHostHelper;
pub use massing::{
	is_massing_level, massing_box_scene, massing_box_transform, massing_scene, ring_strip_xz,
	MassingKind, MassingRoof, MassingSilhouettePlugin, MassingVolume,
};
pub use math::normalize_xz;
pub use panels::{
	dihedral_kink, fitted_tile_count, to_centered_rect_placement, triangle_normal,
	update_panel_host_levels, with_wall_standup_pitch, PanelGeometry, PanelKitCaps, PanelLodBand,
	PanelLodProbe, PanelNode, PanelStyle, Rectangle as PanelRectangle,
	RightTriangle as PanelRightTriangle, TessellatedTriangle, DEFAULT_MIN_JOINT_ANGLE,
	DEFAULT_TILE_WIDTH, PANEL_HIGH_FACTOR, PANEL_LOW_FACTOR, PANEL_MEDIUM_FACTOR,
	PANEL_ULTRA_LOW_RECTANGLE, PANEL_ULTRA_LOW_RIGHT_TRIANGLE,
};
pub use parent_confines::{
	apply_parent_confines, confined_scene, distance_to_segment, InternalShape, ParentConfines,
	INTERNAL_REVEAL_FACTOR,
};
pub use partitions::{
	update_partition_host_levels, Partition, PartitionGeometry, PartitionLodBand,
	PartitionLodProbe, PartitionMeshSet, PartitionMeshTier, PartitionNode, PartitionStyle,
	LINEAR_HIGH_FACTOR, LINEAR_LOW_FACTOR, LINEAR_MEDIUM_FACTOR, SLICE_KIT_HEIGHT,
};
pub use placed::{Placed, Placement};
pub use roofs::{
	update_roof_host_levels, Pitch, RoofGeometry, RoofLodBand, RoofLodProbe, RoofNode, RoofStyle,
	ROOF_HIGH_FACTOR, ROOF_LOW_FACTOR, ROOF_MEDIUM_FACTOR,
};
pub use scene_children::{
	pose, posed_glb, posed_scene, scene_children, wireframe_box_with_handles, with_pose,
};
pub use stairs::StairNode;
pub use structural_probe::{
	distance_outside_aabb2d_xz, distance_outside_footprints,
	update_building_structural_host_levels, BuildingStructuralLodPlugin,
	BuildingStructuralLodProbe, STRUCTURAL_HIGH_OUTSIDE_METERS, STRUCTURAL_LOW_OUTSIDE_METERS,
	STRUCTURAL_MEDIUM_OUTSIDE_METERS,
};

use bevy::ecs::template::template;
use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{
	Commands, CommandsSceneExt, Component, Entity, GlobalTransform, Transform, Visibility,
};
use bevy::scene::prelude::{bsn, template_value};
use bevy::scene::{ResolveContext, ResolvedScene, Scene};
use lod::lod_ref::LodRef;
use lod::scene::{cull_named_from_factor, LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::{lod_host_scene_pending, SceneChunk};
use std::sync::Arc;

/// Domain IR exposed by a building (or building part) for structural composition.
///
/// Each method returns nodes of one domain type, grouped by provenance [`Layer`]
/// (see [`Layers`]). Layer identity is **not** node-type identity—it records where
/// geometry came from so parents can apply policy. Prefer [`Layers::free`] until a
/// provenance name is meaningful. Buildings compose by [`Layers::extend`].
pub trait BuildingComponents {
	fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
		Layers::new()
	}

	fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
		Layers::new()
	}

	fn floor_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FloorNode> {
		Layers::new()
	}

	fn roof_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RoofNode> {
		Layers::new()
	}

	fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
		Layers::new()
	}

	fn door_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<DoorNode> {
		Layers::new()
	}

	fn joint_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<JointNode> {
		Layers::new()
	}

	fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
		Layers::new()
	}

	fn furniture_usage_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureUsageNode> {
		Layers::new()
	}

	fn label_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<LabelNode> {
		Layers::new()
	}

	/// When set, [`ComponentsOnly`] bands High / Medium / Low / UltraLow via this probe.
	///
	/// Low and UltraLow replace nested kit hosts with the probe's [`MassingVolume`]s.
	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		None
	}
}

impl<T: BuildingComponents + ?Sized> BuildingComponents for &T {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		(**self).panel_nodes_for_level(level)
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		(**self).partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		(**self).floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		(**self).roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		(**self).stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		(**self).door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		(**self).joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		(**self).furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureUsageNode> {
		(**self).furniture_usage_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		(**self).label_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		(**self).structural_lod()
	}
}

impl<T: BuildingComponents + ?Sized> BuildingComponents for Arc<T> {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		(**self).panel_nodes_for_level(level)
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		(**self).partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		(**self).floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		(**self).roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		(**self).stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		(**self).door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		(**self).joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		(**self).furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureUsageNode> {
		(**self).furniture_usage_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		(**self).label_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		(**self).structural_lod()
	}
}

/// Newtype: present a [`BuildingComponents`] value as a structural [`LodScene`] host.
///
/// Prefer this over a custom `LodScene` when the building has no silhouette, lights, or
/// other non-node extras. Chunk fulfill drains posed kit *content* (no nested
/// panel / partition / … hosts), matching flattened vegetation.
///
/// Footprints on [`BuildingComponents::structural_lod`] stay host-local. The
/// second field is the host pose used to map a world-space viewer (the
/// [`LodRef`] camera) into that frame — the same mapping
/// [`update_building_structural_host_levels`] does via [`GlobalTransform`].
///
/// ```ignore
/// spawn_building_components(commands, &bedroom, transform, bounds);
/// ```
#[derive(Debug, Clone, PartialEq, Component)]
pub struct ComponentsOnly<T: Send + Sync + 'static>(pub T, Transform);

impl<T: Send + Sync + 'static> ComponentsOnly<T> {
	pub fn new(building: T) -> Self {
		Self(building, Transform::IDENTITY)
	}

	pub fn posed(building: T, host_transform: Transform) -> Self {
		Self(building, host_transform)
	}

	pub fn host_transform(&self) -> Transform {
		self.1
	}

	pub fn into_inner(self) -> T {
		self.0
	}

	fn host_global(&self) -> GlobalTransform {
		GlobalTransform::from(self.1)
	}
}

impl<T: Send + Sync + 'static> From<T> for ComponentsOnly<T> {
	fn from(value: T) -> Self {
		Self::new(value)
	}
}

impl<T: Send + Sync + 'static> std::ops::Deref for ComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T: Send + Sync + 'static> std::ops::DerefMut for ComponentsOnly<T> {
	fn deref_mut(&mut self) -> &mut T {
		&mut self.0
	}
}

impl<T: BuildingComponents + Send + Sync + 'static> BuildingComponents for ComponentsOnly<T> {
	fn panel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PanelNode> {
		self.0.panel_nodes_for_level(level)
	}

	fn partition_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartitionNode> {
		self.0.partition_nodes_for_level(level)
	}

	fn floor_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FloorNode> {
		self.0.floor_nodes_for_level(level)
	}

	fn roof_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RoofNode> {
		self.0.roof_nodes_for_level(level)
	}

	fn stair_nodes_for_level(&self, level: LodSceneLevel) -> Layers<StairNode> {
		self.0.stair_nodes_for_level(level)
	}

	fn door_nodes_for_level(&self, level: LodSceneLevel) -> Layers<DoorNode> {
		self.0.door_nodes_for_level(level)
	}

	fn joint_nodes_for_level(&self, level: LodSceneLevel) -> Layers<JointNode> {
		self.0.joint_nodes_for_level(level)
	}

	fn furniture_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureNode> {
		self.0.furniture_nodes_for_level(level)
	}

	fn furniture_usage_nodes_for_level(&self, level: LodSceneLevel) -> Layers<FurnitureUsageNode> {
		self.0.furniture_usage_nodes_for_level(level)
	}

	fn label_nodes_for_level(&self, level: LodSceneLevel) -> Layers<LabelNode> {
		self.0.label_nodes_for_level(level)
	}

	fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
		self.0.structural_lod()
	}
}

impl<T: BuildingComponents + Send + Sync + 'static> LodScene for ComponentsOnly<T> {
	fn scene_lod_level(&self, lod_ref: &LodRef) -> LodSceneLevel {
		self.0
			.structural_lod()
			.map(|p| p.level_for_world(lod_ref.current_transform.translation, &self.host_global()))
			.unwrap_or(LodSceneLevel::High)
	}

	fn scene_lod_status(&self, lod_ref: &LodRef) -> LodSceneStatus {
		match self.0.structural_lod() {
			Some(probe) => probe.status_for_world(
				lod_ref.previous_transform.translation,
				lod_ref.current_transform.translation,
				&self.host_global(),
			),
			None => LodSceneStatus::Unchanged,
		}
	}

	fn scene_lod_culls(&self, lod_ref: &LodRef, current: LodSceneLevel) -> LodSceneCulls {
		let Some(probe) = self.0.structural_lod() else {
			return LodSceneCulls::None;
		};
		let d = probe
			.distance_outside_world(lod_ref.current_transform.translation, &self.host_global());
		let (high, medium, low) = probe.band_meters();
		// Kit roots are heavy: drop Medium as soon as Low massing is current.
		// High/Medium keep the adjacent band warm for walking in and out.
		let adjacent_depth = match current {
			LodSceneLevel::Low | LodSceneLevel::UltraLow => Some(0.0),
			_ => None,
		};
		cull_named_from_factor(d, high, medium, low, adjacent_depth).with_customs()
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		flattened_component_scene(&self.0, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		building_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		self.0
			.structural_lod()
			.map(|p| p.footprint_aabb())
			.unwrap_or_else(|| building_bounds(&self.0))
	}

	fn scene_with_lod(&self, lod_ref: &LodRef) -> impl Scene + 'static {
		let level = self.scene_lod_level(lod_ref);
		lod_host_scene_pending(level, self.scene_bounds())
	}
}

/// Drain weight for one posed kit ([`scene_ref::SceneRef`] + later WorldAsset admit).
///
/// Same as vegetation: weight 1 treated a GLB instance like an empty transform.
pub const FLATTENED_KIT_CHUNK_WEIGHT: u32 = 4;

enum FlattenedKit {
	Panel(PanelNode),
	Partition(PartitionNode),
	Floor(FloorNode),
	Roof(RoofNode),
	Joint(JointNode),
	Stair(StairNode),
	Door(DoorNode),
}

impl FlattenedKit {
	fn scene(&self, lod_ref: &LodRef, level: LodSceneLevel) -> Box<dyn Scene> {
		match self {
			Self::Panel(node) => Box::new(node.scene_with_level(lod_ref, level)),
			Self::Partition(node) => Box::new(node.scene_with_level(lod_ref, level)),
			Self::Floor(node) => Box::new(node.scene_with_level(lod_ref, level)),
			Self::Roof(node) => Box::new(node.scene_with_level(lod_ref, level)),
			Self::Joint(node) => Box::new(node.scene_with_level(lod_ref, level)),
			Self::Stair(node) => Box::new(node.scene_with_level(lod_ref, level)),
			Self::Door(node) => Box::new(node.scene_with_level(lod_ref, level)),
		}
	}
}

fn flattened_kits(building: &impl BuildingComponents, level: LodSceneLevel) -> Vec<FlattenedKit> {
	let mut kits = Vec::new();
	kits.extend(
		building
			.panel_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(FlattenedKit::Panel),
	);
	kits.extend(
		building
			.partition_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(FlattenedKit::Partition),
	);
	kits.extend(
		building
			.floor_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(FlattenedKit::Floor),
	);
	kits.extend(
		building
			.roof_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(FlattenedKit::Roof),
	);
	kits.extend(
		building
			.joint_nodes_for_level(level)
			.flatten()
			.into_iter()
			.map(FlattenedKit::Joint),
	);
	// Circulation stays readable on Medium.
	// Furniture presents on a separate 50 m host neighborhood, not this tree.
	if matches!(level, LodSceneLevel::High | LodSceneLevel::Medium) {
		kits.extend(
			building
				.stair_nodes_for_level(level)
				.flatten()
				.into_iter()
				.map(FlattenedKit::Stair),
		);
		kits.extend(
			building
				.door_nodes_for_level(level)
				.flatten()
				.into_iter()
				.map(FlattenedKit::Door),
		);
	}
	// Label IR stays on the building for packers / tests. It is not drawn.
	kits
}

/// Weighted chunks for one structural level: posed kits, no nested domain hosts.
///
/// Kits are produced lazily so begin does not box every `scene_with_level` up front.
/// Each kit costs [`FLATTENED_KIT_CHUNK_WEIGHT`]. Shared kit GLBs stay posed
/// [`scene_ref::SceneRef`]s so the renderer instances them; do not bake them into
/// unique [`scene_ref::MultiSceneMerge`] meshes.
pub fn building_scene_chunks(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	if is_massing_level(level) && building.structural_lod().is_some() {
		return SceneChunk::primitive(massing_scene(building, level));
	}
	let kits = flattened_kits(building, level);
	let n = kits.len();
	if n == 0 {
		return SceneChunk::primitive(scene_children(Vec::new()));
	}

	let prev = *lod_ref.previous_transform;
	let curr = *lod_ref.current_transform;
	let bounds = *lod_ref.bounds;
	let entity = lod_ref.entity;
	let kit_w = FLATTENED_KIT_CHUNK_WEIGHT;
	let mut index = 0usize;
	SceneChunk::lazy(n as u32 * kit_w, n, move || {
		if index >= kits.len() {
			return None;
		}
		let kit_lod =
			LodRef { entity, previous_transform: &prev, current_transform: &curr, bounds: &bounds };
		let scene = kits[index].scene(&kit_lod, level);
		index += 1;
		Some(SceneChunk::weighted(kit_w, scene))
	})
}

/// Append every domain node from `building` at `level` as nested [`LodScene`] hosts.
///
/// Each child is embedded via [`LodScene::host`] (pending host + typed component).
/// Provenance is flattened away ([`Layers::flatten`]) for presentation today.
/// Prefer [`append_flattened_component_scenes`] for world / `ComponentsOnly` presentation.
pub fn append_component_scenes(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for kit in flattened_kits(building, level) {
		match kit {
			FlattenedKit::Panel(node) => children.push(Box::new(node.host(lod_ref))),
			FlattenedKit::Partition(node) => children.push(Box::new(node.host(lod_ref))),
			FlattenedKit::Floor(node) => children.push(Box::new(node.host(lod_ref))),
			FlattenedKit::Roof(node) => children.push(Box::new(node.host(lod_ref))),
			FlattenedKit::Joint(node) => children.push(Box::new(node.host(lod_ref))),
			FlattenedKit::Stair(node) => children.push(Box::new(node.host(lod_ref))),
			FlattenedKit::Door(node) => children.push(Box::new(node.host(lod_ref))),
		}
	}
}

/// Append posed kit *content* (GLB / wireframe scenes), not nested [`LodScene`] hosts.
pub fn append_flattened_component_scenes(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for kit in flattened_kits(building, level) {
		children.push(kit.scene(lod_ref, level));
	}
}

/// Scene whose children are nested domain [`LodScene`] hosts at `level`.
pub fn component_only_scene(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	if is_massing_level(level) && building.structural_lod().is_some() {
		return Box::new(massing_scene(building, level)) as Box<dyn Scene>;
	}
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(building, lod_ref, level, &mut children);
	Box::new(scene_children(children)) as Box<dyn Scene>
}

/// All kit content for `level` as siblings under one parent (no fine-phase hosts).
pub fn flattened_component_scene(
	building: &impl BuildingComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	if is_massing_level(level) && building.structural_lod().is_some() {
		return Box::new(massing_scene(building, level)) as Box<dyn Scene>;
	}
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_flattened_component_scenes(building, lod_ref, level, &mut children);
	Box::new(scene_children(children)) as Box<dyn Scene>
}

/// Spawn a [`ComponentsOnly`] building host; chunk fulfill streams the first level.
pub fn spawn_building_components<T>(
	commands: &mut Commands,
	building: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	spawn_building_components_for(commands, building, transform, bounds, None)
}

/// [`spawn_building_components`] using `viewer` for the first pending band.
pub fn spawn_building_components_for<T>(
	commands: &mut Commands,
	building: &T,
	transform: Transform,
	bounds: Aabb3d,
	viewer: Option<&LodRef>,
) -> Vec<Entity>
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	vec![commands
		.spawn_scene(building_components_host_for(building, transform, bounds, viewer))
		.id()]
}

/// A pending [`ComponentsOnly`] building host, for nesting in another scene.
pub fn building_components_host<T>(
	building: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> impl Scene + 'static
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	building_components_host_for(building, transform, bounds, None)
}

/// [`building_components_host`] that picks the first pending band from `viewer`.
///
/// `viewer` is the world-space camera. The host pose is `transform`. Without a
/// viewer the first band is computed as if the camera sat at the world origin.
pub fn building_components_host_for<T>(
	building: &T,
	transform: Transform,
	bounds: Aabb3d,
	viewer: Option<&LodRef>,
) -> impl Scene + 'static
where
	T: BuildingComponents + Clone + Send + Sync + 'static,
{
	let identity = Transform::IDENTITY;
	let fallback = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let lod_ref = viewer.unwrap_or(&fallback);
	let host = ComponentsOnly::posed(building.clone(), transform);
	let level = host.scene_lod_level(lod_ref);
	let pending = (
		lod_host_scene_pending(level, bounds),
		bsn! {
			template_value(transform)
			Visibility::default()
			template(move |_ctx| Ok(host.clone()))
		},
	);
	match building.structural_lod() {
		Some(probe) => Box::new((pending, bsn! { template_value(probe) })) as Box<dyn Scene>,
		None => Box::new(pending) as Box<dyn Scene>,
	}
}

/// Approximate AABB from domain node placements at High (for adapter LodRef bounds).
pub fn building_bounds(building: &impl BuildingComponents) -> Aabb3d {
	let mut min = bevy::math::Vec3::splat(f32::INFINITY);
	let mut max = bevy::math::Vec3::splat(f32::NEG_INFINITY);
	let mut any = false;
	let mut absorb = |bounds: Aabb3d| {
		min = min.min(Vec3::from(bounds.min));
		max = max.max(Vec3::from(bounds.max));
		any = true;
	};
	for node in building.panel_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.partition_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.floor_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.roof_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.stair_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.door_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.joint_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	for node in building.label_nodes_for_level(LodSceneLevel::High).flatten() {
		absorb(node.scene_bounds());
	}
	if any {
		Aabb3d::from_min_max(min, max)
	} else {
		Aabb3d::from_min_max(bevy::math::Vec3::ZERO, bevy::math::Vec3::ONE)
	}
}

pub(crate) fn empty_scene(_: &mut ResolveContext, _: &mut ResolvedScene) {}

/// Shared empty `LodScene` body for component placeholders.
macro_rules! impl_empty_lod_scene {
	($($ty:ty),+ $(,)?) => {
		$(
			impl ::lod::scene::LodScene for $ty {
				fn scene_lod_status(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
				) -> ::lod::scene::LodSceneStatus {
					::lod::scene::LodSceneStatus::Unchanged
				}

				fn scene_with_level(
					&self,
					_lod_ref: &::lod::lod_ref::LodRef,
					_level: ::lod::scene::LodSceneLevel,
				) -> impl ::bevy::scene::Scene + 'static {
					::bevy::scene::SceneFunction($crate::empty_scene)
				}
			}
		)+
	};
}

pub(crate) use impl_empty_lod_scene;

/// `LodScene` that loads a GLB scene root via [`scene_ref::SceneRef`].
macro_rules! impl_glb_lod_scene {
	($ty:ty, $asset:expr) => {
		impl ::lod::scene::LodScene for $ty {
			fn scene_lod_status(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
			) -> ::lod::scene::LodSceneStatus {
				::lod::scene::LodSceneStatus::Unchanged
			}

			fn scene_with_level(
				&self,
				_lod_ref: &::lod::lod_ref::LodRef,
				_level: ::lod::scene::LodSceneLevel,
			) -> impl ::bevy::scene::Scene + 'static {
				($asset).scene_ref().scene()
			}
		}
	};
}

pub(crate) use impl_glb_lod_scene;

#[cfg(test)]
mod flatten_tests {
	use super::*;
	use crate::doors::DoorGeometry;
	use crate::stairs::StairGeometry;
	use bevy::prelude::{Entity, Transform};

	struct ShellAndFixture {
		panel: PanelNode,
		bed: FurnitureNode,
	}

	impl BuildingComponents for ShellAndFixture {
		fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
			Layers::from_free(vec![self.panel.clone()])
		}

		fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
			Layers::from_free(vec![self.bed.clone()])
		}
	}

	fn sample() -> ShellAndFixture {
		ShellAndFixture {
			panel: PanelNode::rough_stone(PanelGeometry::rectangle(), Placement::IDENTITY),
			bed: FurnitureNode::bed(Placement::IDENTITY),
		}
	}

	fn lod_ref<'a>(tf: &'a Transform, bounds: &'a Aabb3d) -> LodRef<'a> {
		LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: tf,
			current_transform: tf,
			bounds,
		}
	}

	#[test]
	fn high_chunks_are_lazy_kits_weighted_like_vegetation() {
		let building = sample();
		let tf = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let chunks = building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::High);
		assert_eq!(chunks.total_primitives(), 1);
		assert_eq!(chunks.total_weight(), FLATTENED_KIT_CHUNK_WEIGHT);
	}

	#[test]
	fn medium_omits_interior_fixtures() {
		let building = sample();
		let tf = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let chunks =
			building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::Medium);
		assert_eq!(chunks.total_primitives(), 1);
		assert_eq!(chunks.total_weight(), FLATTENED_KIT_CHUNK_WEIGHT);
	}

	struct CirculationAndFixture {
		panel: PanelNode,
		stair: StairNode,
		door: DoorNode,
		bed: FurnitureNode,
	}

	impl BuildingComponents for CirculationAndFixture {
		fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
			Layers::from_free(vec![self.panel.clone()])
		}

		fn stair_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<StairNode> {
			Layers::from_free(vec![self.stair.clone()])
		}

		fn door_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<DoorNode> {
			Layers::from_free(vec![self.door.clone()])
		}

		fn furniture_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<FurnitureNode> {
			Layers::from_free(vec![self.bed.clone()])
		}
	}

	#[test]
	fn medium_keeps_circulation_omits_furniture() {
		let building = CirculationAndFixture {
			panel: PanelNode::rough_stone(PanelGeometry::rectangle(), Placement::IDENTITY),
			stair: StairNode::rough_stone(StairGeometry::straight(), Placement::IDENTITY),
			door: DoorNode::wood(DoorGeometry::leaf(), Placement::IDENTITY),
			bed: FurnitureNode::bed(Placement::IDENTITY),
		};
		let tf = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let medium =
			building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::Medium);
		assert_eq!(medium.total_primitives(), 3);
		let high = building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::High);
		assert_eq!(high.total_primitives(), 3);
	}

	struct TwoWalls {
		a: PanelNode,
		b: PanelNode,
	}

	impl BuildingComponents for TwoWalls {
		fn panel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PanelNode> {
			Layers::from_free(vec![self.a.clone(), self.b.clone()])
		}
	}

	#[test]
	fn same_style_panels_stay_separate_posed_chunks() {
		let building = TwoWalls {
			a: PanelNode::rough_stone(PanelGeometry::rectangle(), Placement::IDENTITY),
			b: PanelNode::rough_stone(PanelGeometry::rectangle(), Placement::new(Vec3::X, 0.0)),
		};
		let tf = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let chunks = building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::High);
		assert_eq!(chunks.total_primitives(), 2);
		assert_eq!(chunks.total_weight(), 2 * FLATTENED_KIT_CHUNK_WEIGHT);
	}

	#[test]
	fn distinct_panel_styles_stay_separate() {
		let building = TwoWalls {
			a: PanelNode::rough_stone(PanelGeometry::rectangle(), Placement::IDENTITY),
			b: PanelNode::shepherds_thatch(PanelGeometry::rectangle(), Placement::IDENTITY),
		};
		let tf = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let chunks = building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::High);
		assert_eq!(chunks.total_primitives(), 2);
	}

	struct TwoPartitions {
		a: PartitionNode,
		b: PartitionNode,
	}

	impl BuildingComponents for TwoPartitions {
		fn partition_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartitionNode> {
			Layers::from_free(vec![self.a.clone(), self.b.clone()])
		}
	}

	#[test]
	fn offset_host_bands_from_the_world_viewer() {
		let building = OffsetStub;
		let pose = Transform::from_xyz(2_000.0, 0.0, 0.0);
		let host = ComponentsOnly::posed(building, pose);
		let bounds = host.scene_bounds();
		let beside = Transform::from_xyz(2_000.0, 1.5, 0.0);
		let four_hundred_out = Transform::from_xyz(1_600.0, 1.5, 0.0);
		assert_eq!(
			host.scene_lod_level(&lod_ref(&beside, &bounds)),
			LodSceneLevel::High,
			"standing next to an offset building must be High"
		);
		assert_eq!(
			host.scene_lod_level(&lod_ref(&four_hundred_out, &bounds)),
			LodSceneLevel::Low,
			"400 m from an offset footprint is Low (520 m far edge)"
		);
	}

	#[test]
	fn world_viewer_without_host_pose_is_the_old_stuck_band() {
		let host = ComponentsOnly::new(OffsetStub);
		let bounds = host.scene_bounds();
		let beside_world = Transform::from_xyz(2_000.0, 1.5, 0.0);
		assert_eq!(
			host.scene_lod_level(&lod_ref(&beside_world, &bounds)),
			LodSceneLevel::UltraLow,
			"without a stored pose the world camera is read as local — the HCSG regression"
		);
	}

	#[derive(Clone)]
	struct OffsetStub;

	impl BuildingComponents for OffsetStub {
		fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
			Some(
				BuildingStructuralLodProbe::from_aabb3d_xz(
					Vec3::new(-5.0, 0.0, -5.0),
					Vec3::new(5.0, 3.0, 5.0),
				)
				.with_high_outside_meters(80.0)
				.with_medium_outside_meters(220.0)
				.with_low_outside_meters(520.0),
			)
		}
	}

	#[test]
	fn internal_and_external_partitions_do_not_merge() {
		let building = TwoPartitions {
			a: PartitionNode::rough_stone(PartitionGeometry::linear(), Placement::IDENTITY),
			b: PartitionNode::rough_stone(PartitionGeometry::linear(), Placement::IDENTITY)
				.with_confines(ParentConfines::internal(Vec3::ZERO, 4.0)),
		};
		let tf = Transform::IDENTITY;
		let bounds = Aabb3d::from_min_max(Vec3::ZERO, Vec3::ONE);
		let chunks = building_scene_chunks(&building, &lod_ref(&tf, &bounds), LodSceneLevel::High);
		assert_eq!(chunks.total_primitives(), 2);
	}
}

#[cfg(test)]
mod lod_refresh_tests {
	use super::*;
	use bevy::ecs::entity_disabling::Disabled;
	use bevy::prelude::*;
	use bevy::scene::ScenePlugin;
	use lod::{
		add_lod_refresh_chunk_full_for, lod_root_is_shown, LodLevelRoot, LodLevelRoots,
		LodRefreshCorePlugin, LodSceneHost, LodSceneLevel, LodViewer,
	};

	#[derive(Clone)]
	struct OffsetStub;

	impl BuildingComponents for OffsetStub {
		fn structural_lod(&self) -> Option<BuildingStructuralLodProbe> {
			Some(
				BuildingStructuralLodProbe::from_aabb3d_xz(
					Vec3::new(-5.0, 0.0, -5.0),
					Vec3::new(5.0, 3.0, 5.0),
				)
				.with_high_outside_meters(80.0)
				.with_medium_outside_meters(220.0)
				.with_low_outside_meters(520.0),
			)
		}
	}

	const HOST_AT: Vec3 = Vec3::new(2_000.0, 0.0, 0.0);
	const FAR: Vec3 = Vec3::new(1_600.0, 1.5, 0.0);
	const BESIDE: Vec3 = Vec3::new(2_000.0, 1.5, 0.0);

	fn refresh_app() -> App {
		let mut app = App::new();
		app.add_plugins((MinimalPlugins, TransformPlugin, AssetPlugin::default(), ScenePlugin))
			.init_asset::<Mesh>()
			.init_asset::<StandardMaterial>()
			.init_asset::<bevy::world_serialization::WorldAsset>()
			.add_plugins(LodRefreshCorePlugin)
			.add_plugins((BuildingStructuralLodPlugin, MassingSilhouettePlugin));
		add_lod_refresh_chunk_full_for::<ComponentsOnly<OffsetStub>>(&mut app);
		app
	}

	fn spawn_offset_host(app: &mut App, viewer_at: Vec3) -> Entity {
		let building = OffsetStub;
		let transform = Transform::from_translation(HOST_AT);
		let bounds = ComponentsOnly::posed(building.clone(), transform).scene_bounds();
		let viewer = Transform::from_translation(viewer_at);
		let lod_ref = LodRef {
			entity: Entity::PLACEHOLDER,
			previous_transform: &viewer,
			current_transform: &viewer,
			bounds: &bounds,
		};
		let host = {
			let mut commands = app.world_mut().commands();
			spawn_building_components_for(
				&mut commands,
				&building,
				transform,
				bounds,
				Some(&lod_ref),
			)[0]
		};
		app.world_mut().flush();
		host
	}

	fn host_level(app: &App, host: Entity) -> LodSceneLevel {
		*app.world().get::<LodSceneLevel>(host).expect("host LodSceneLevel")
	}

	fn shown_root_levels(app: &App, host: Entity) -> Vec<LodSceneLevel> {
		let world = app.world();
		let Some(children) = world.get::<Children>(host) else {
			return Vec::new();
		};
		let mut shown = Vec::new();
		for child in children.iter() {
			if world.get::<LodLevelRoots>(child).is_none() {
				continue;
			}
			let Some(roots) = world.get::<Children>(child) else {
				continue;
			};
			for root in roots.iter() {
				let Some(level) = world.get::<LodLevelRoot>(root) else {
					continue;
				};
				let hidden = world.get::<Disabled>(root).is_some()
					|| world
						.get::<Visibility>(root)
						.is_some_and(|visibility| !lod_root_is_shown(*visibility));
				if !hidden {
					shown.push(level.0);
				}
			}
		}
		shown
	}

	fn move_viewer(app: &mut App, viewer: Entity, at: Vec3) {
		app.world_mut().entity_mut(viewer).insert(Transform::from_translation(at));
	}

	fn update_until(app: &mut App, mut done: impl FnMut(&mut App) -> bool) {
		for _ in 0..120 {
			if done(app) {
				return;
			}
			app.update();
		}
	}

	#[test]
	fn walking_up_to_an_offset_building_upgrades_to_high_and_hides_low() {
		let mut app = refresh_app();
		let viewer = app.world_mut().spawn((LodViewer, Transform::from_translation(FAR))).id();
		let host = spawn_offset_host(&mut app, FAR);
		assert!(app.world().get::<LodSceneHost>(host).is_some(), "spawned ComponentsOnly host");

		update_until(&mut app, |app| host_level(app, host) == LodSceneLevel::Low);
		assert_eq!(
			host_level(&app, host),
			LodSceneLevel::Low,
			"400 m from a 2 km building must start Low, not the spawn-at-origin High"
		);

		update_until(&mut app, |app| shown_root_levels(app, host).contains(&LodSceneLevel::Low));

		move_viewer(&mut app, viewer, BESIDE);
		update_until(&mut app, |app| host_level(app, host) == LodSceneLevel::High);
		assert_eq!(
			host_level(&app, host),
			LodSceneLevel::High,
			"walking up to the offset host must upgrade; treating the world camera as local stays UltraLow"
		);

		update_until(&mut app, |app| {
			let shown = shown_root_levels(app, host);
			shown.contains(&LodSceneLevel::High) && !shown.contains(&LodSceneLevel::Low)
		});
		let shown = shown_root_levels(&app, host);
		assert!(
			shown.contains(&LodSceneLevel::High),
			"High root should be visible after fulfill, got {shown:?}"
		);
		assert!(
			!shown.contains(&LodSceneLevel::Low) && !shown.contains(&LodSceneLevel::UltraLow),
			"lower roots must hide or despawn once High is up, got {shown:?}"
		);
	}
}
