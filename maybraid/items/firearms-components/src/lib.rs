//! Reusable firearm scene components.
//!
//! Per domain: kit GLB + optional [`SocketRef`] → node (`LodScene`). Recipes in
//! [`firearms`](../../firearms/) implement [`FirearmComponents`] and present via
//! [`ComponentsOnly`].

pub mod assets;
pub mod layer;
pub mod nodes;
pub mod plugin;
pub mod scene_children;

pub use assets::AssetPath;
pub use layer::{Layer, Layers};
pub use nodes::{FirearmPartSlot, PartNode, RigNode, RECEIVER_LANDMARKS};
pub use plugin::{add_firearm_components_host, FirearmComponentsPlugin, FirearmHostSystems};
pub use rigs::{
	ActiveRigPose, AssemblyHost, AssemblyMembers as FirearmMembers, AssemblyRoot, BindPose,
	BoneMap, BoneScale, MemberOf, PoseApplied, ResolvedRigPose, RigPoseLayer, RigRoot, SocketRef,
	SocketRefApplied, SocketRefRoot,
};

use bevy::math::bounding::Aabb3d;
use bevy::math::Vec3;
use bevy::prelude::{Commands, CommandsSceneExt, Component, Entity, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::{lod_host_scene_pending, SceneChunk};

use crate::scene_children::scene_children;

/// Domain marker on the firearm assembly host.
#[derive(Component, Clone, Copy, Default)]
pub struct FirearmRoot;

/// Domain IR exposed by a firearm (or firearm wrapper) for structural composition.
///
/// Each method returns nodes of one domain type, grouped by provenance [`Layer`]
/// (see [`Layers`]). Layer identity is **not** node-type identity. Prefer
/// [`Layers::free`] until a provenance name is meaningful.
pub trait FirearmComponents {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::new()
	}

	fn body_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	fn barrel_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	fn trigger_box_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	fn grip_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	fn stock_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	fn sight_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}
}

impl<T: FirearmComponents + ?Sized> FirearmComponents for &T {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		(**self).rig_nodes_for_level(level)
	}

	fn body_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).body_nodes_for_level(level)
	}

	fn barrel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).barrel_nodes_for_level(level)
	}

	fn trigger_box_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).trigger_box_nodes_for_level(level)
	}

	fn grip_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).grip_nodes_for_level(level)
	}

	fn stock_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).stock_nodes_for_level(level)
	}

	fn sight_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).sight_nodes_for_level(level)
	}
}

/// Newtype: present a [`FirearmComponents`] value as a structural [`LodScene`] host.
///
/// Chunk fulfill nests fine-phase domain nodes via [`LodScene::host`].
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

impl<T: Default + Send + Sync + 'static> Default for ComponentsOnly<T> {
	fn default() -> Self {
		Self(T::default())
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

impl<T: FirearmComponents + Send + Sync + 'static> FirearmComponents for ComponentsOnly<T> {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.0.rig_nodes_for_level(level)
	}

	fn body_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.body_nodes_for_level(level)
	}

	fn barrel_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.barrel_nodes_for_level(level)
	}

	fn grip_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.grip_nodes_for_level(level)
	}

	fn trigger_box_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.trigger_box_nodes_for_level(level)
	}

	fn stock_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.stock_nodes_for_level(level)
	}

	fn sight_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.sight_nodes_for_level(level)
	}
}

impl<T: FirearmComponents + Send + Sync + 'static> LodScene for ComponentsOnly<T> {
	fn scene_lod_level(&self, _lod_ref: &LodRef) -> LodSceneLevel {
		LodSceneLevel::High
	}

	fn scene_lod_status(&self, _lod_ref: &LodRef) -> LodSceneStatus {
		LodSceneStatus::Unchanged
	}

	fn scene_lod_culls(&self, _lod_ref: &LodRef, _current: LodSceneLevel) -> LodSceneCulls {
		LodSceneCulls::None
	}

	fn scene_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> impl Scene + 'static {
		component_only_scene(&self.0, lod_ref, level)
	}

	fn scene_chunks_with_level(&self, lod_ref: &LodRef, level: LodSceneLevel) -> SceneChunk {
		firearm_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		firearm_bounds(&self.0)
	}

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let _ = lod_ref;
		let host = self.clone();
		bsn! {
			template_value(host)
			AssemblyRoot
			FirearmRoot
			Visibility::default()
		}
	}
}

/// Weighted chunks for one structural level: nested rig/part hosts.
pub fn firearm_scene_chunks(
	firearm: &impl FirearmComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let mut chunks = Vec::new();
	for node in firearm.rig_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in firearm.body_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in firearm.barrel_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in firearm.grip_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in firearm.trigger_box_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in firearm.stock_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in firearm.sight_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	if chunks.is_empty() {
		SceneChunk::primitive(scene_children(Vec::new()))
	} else {
		SceneChunk::chunks(chunks)
	}
}

pub fn append_component_scenes(
	firearm: &impl FirearmComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in firearm.rig_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in firearm.body_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in firearm.barrel_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in firearm.grip_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in firearm.trigger_box_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in firearm.stock_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in firearm.sight_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
}

pub fn component_only_scene(
	firearm: &impl FirearmComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(firearm, lod_ref, level, &mut children);
	scene_children(children)
}

/// Approximate AABB for a handheld firearm (bands are identical for now).
pub fn firearm_bounds(_firearm: &impl FirearmComponents) -> Aabb3d {
	assembled_firearm_bounds()
}

/// Envelope used for LOD, hold scale, and menu inspect framing.
pub fn assembled_firearm_bounds() -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(-0.5, -0.5, -2.2), Vec3::new(0.5, 1.4, 0.5))
}

/// 3/4 inspect camera: side of the bore (+Z), distance from FOV × AABB sphere.
///
/// Look at the receiver (origin), not the stock-heavy AABB center, so the kit
/// sits in the middle of the pane. Yaw is from +Z toward −X so the muzzle
/// reads toward screen-right. Distance still fits the envelope in `fov`.
pub fn firearm_preview_camera(vertical_fov: f32) -> (Vec3, Vec3) {
	firearm_preview_camera_for(assembled_firearm_bounds(), vertical_fov)
}

const PREVIEW_YAW_FROM_BORE: f32 = -1.2;
const PREVIEW_PITCH: f32 = 0.28;
const PREVIEW_PADDING: f32 = 1.2;

fn firearm_preview_camera_for(bounds: Aabb3d, vertical_fov: f32) -> (Vec3, Vec3) {
	let look_at = Vec3::new(0.0, 0.2, 0.0);
	let mut radius = 0.0_f32;
	for x in [bounds.min.x, bounds.max.x] {
		for y in [bounds.min.y, bounds.max.y] {
			for z in [bounds.min.z, bounds.max.z] {
				radius = radius.max(look_at.distance(Vec3::new(x, y, z)));
			}
		}
	}
	let dist = PREVIEW_PADDING * radius.max(0.1) / (vertical_fov.max(1e-3) * 0.5).tan();
	let yaw = PREVIEW_YAW_FROM_BORE;
	let pitch = PREVIEW_PITCH;
	let dir = Vec3::new(yaw.sin() * pitch.cos(), pitch.sin(), yaw.cos() * pitch.cos()).normalize();
	(look_at + dir * dist, look_at)
}

/// Spawn a [`ComponentsOnly`] firearm host; chunk fulfill streams the first level.
pub fn spawn_firearm_components<T>(
	commands: &mut Commands,
	firearm: &T,
	transform: Transform,
	bounds: Aabb3d,
) -> Vec<Entity>
where
	T: FirearmComponents + Clone + Default + Send + Sync + 'static,
{
	let identity = Transform::IDENTITY;
	let lod_ref = LodRef {
		entity: Entity::PLACEHOLDER,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let host = ComponentsOnly(firearm.clone());
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
	commands.entity(entity).insert((host, AssemblyRoot, FirearmRoot));
	vec![entity]
}

#[cfg(test)]
mod tests {
	use super::*;

	const DEFAULT_FOV: f32 = 0.8;

	#[test]
	fn inspect_camera_is_beside_the_bore_not_on_the_muzzle() {
		let (camera, look_at) = firearm_preview_camera(DEFAULT_FOV);
		let bounds = assembled_firearm_bounds();
		let offset = camera - look_at;
		let muzzle = Vec3::new(0.0, 0.0, bounds.max.z);
		assert!(offset.length() > 4.0, "dist {}", offset.length());
		assert!(offset.x < 0.0 && offset.x.abs() > offset.z.abs(), "side 3/4 {offset}");
		assert!(camera.distance(muzzle) > 3.5, "muzzle dist {}", camera.distance(muzzle));
		assert!(look_at.x.abs() < 1e-4 && look_at.z.abs() < 1e-4, "receiver {look_at}");
	}

	#[test]
	fn tighter_fov_pulls_the_camera_back() {
		let (wide, _) = firearm_preview_camera(1.0);
		let (tight, look_at) = firearm_preview_camera(0.5);
		assert!(tight.distance(look_at) > wide.distance(look_at));
	}
}
