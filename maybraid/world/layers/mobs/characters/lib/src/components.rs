//! Domain IR + structural [`lod::LodScene`] host for characters.

use bevy::math::bounding::Aabb3d;
use bevy::math::{Quat, Vec3};
use bevy::prelude::{Component, Transform, Visibility};
use bevy::scene::prelude::{bsn, template_value, Scene};
use character_items::{ClothingHost, ClothingMaterial, ClothingMesh, ItemColor};
use lod::lod_ref::LodRef;
use lod::scene::{LodScene, LodSceneCulls, LodSceneLevel, LodSceneStatus};
use lod::SceneChunk;
use material_ref::MaterialRef;
use std::f32::consts::FRAC_PI_2;

use crate::assembly::CharacterPartSlot;
use crate::assets::AssetNormalization;
use crate::layer::Layers;
use crate::member::CharacterRoot;
use crate::nodes::{PartNode, RigNode};
use character_motion::{motion_policy, CharacterHeading};
use character_rigs::ResolvedRigPose;
use rigs::AssemblyRoot;

use crate::scene_children::{maybe_component, scene_children};
use crate::socket::{RigId, SkinRef};

/// Rest-pose locomotion hull. Matches Avian `Collider::capsule(radius, length)`:
/// `length` is the cylinder; total height is `length + 2 * radius`.
///
/// [`Self::pronograde`] marks a horizontal body (quadrupeds). The motor still
/// uses this vertical capsule; live hit-tests use [`Self::hit_capsule`].
/// Oversized orthograde heads add a query-only [`Self::head_capsule`].
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct LocomotionCapsule {
	pub radius: f32,
	pub length: f32,
	pub pronograde: bool,
	/// Rest half-width of the query hull (pronograde). Ignored by the motor.
	pub girdle: f32,
	/// Orthograde head-socket scale per axis (`Vec3::ONE` = stock). Ignored by the motor.
	/// A scalar `with_head_scale` / `humanoid_from_pose` is **Y-only** so a tall
	/// head stretches the query capsule without fattening XZ.
	pub head: Vec3,
}

/// Query-only horizontal hull for a pronograde body. Avian capsules are Y-up;
/// [`Self::local_transform`] lays this along mesh `+Z` (nose) / `-Z` (tail).
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HitCapsule {
	pub radius: f32,
	pub length: f32,
	/// Parent-local Z of the capsule center (negative shifts the hull aft).
	pub along: f32,
}

impl HitCapsule {
	/// Rest wheelbase half from [`character_motion::pitch`] (`QUADRUPED_HALF_SPAN`).
	pub const REST_HALF_SPAN: f32 = 1.2;
	/// Rest stance half from pitch (`QUADRUPED_HALF_WIDTH`).
	pub const REST_HALF_WIDTH: f32 = 0.45;
	/// Extra aft coverage past the hind girdle, at [`LocomotionCapsule::QUADRUPED`] size.
	pub const REST_TAIL: f32 = 0.85;

	const GIRDLE_BONES: &'static [&'static str] = &["shoulder.L", "shoulder.R", "hip.L", "hip.R"];
	const FLESH_BONES: &'static [&'static str] = &[
		"lateral_shoulder_protrusion.L",
		"lateral_shoulder_protrusion.R",
		"lateral_hip_protrusion.L",
		"lateral_hip_protrusion.R",
		"later_hip_protrusion.R",
	];
	const TORSO_BONES: &'static [&'static str] = &[
		"chest_thickness",
		"anterior_mid_back",
		"posterior_mid_back",
		"waist.L",
		"waist.R",
		"lower_chest_width.L",
		"lower_chest_width.R",
	];

	pub fn for_quadruped(hull: LocomotionCapsule) -> Self {
		let stock = LocomotionCapsule::QUADRUPED.radius;
		let scale = if stock <= 0.0 { 1.0 } else { (hull.radius / stock).max(0.0) };
		let tail = Self::REST_TAIL * scale;
		let extent = 2.0 * Self::REST_HALF_SPAN * scale + tail;
		let radius = if hull.girdle > 1e-4 { hull.girdle } else { Self::REST_HALF_WIDTH * scale };
		Self { radius, length: (extent - 2.0 * radius).max(0.0), along: -tail * 0.5 }
	}

	/// Composed rest-pose span: max girdle length, then torso mass, times flesh.
	pub fn girdle_scale(pose: &ResolvedRigPose) -> f32 {
		let span = Self::max_length(pose, Self::GIRDLE_BONES);
		let flesh = Self::max_extent(pose, Self::FLESH_BONES).max(1.0);
		let torso = Self::max_extent(pose, Self::TORSO_BONES).max(1.0);
		span.max(torso) * flesh
	}

	pub fn half_width_from(pose: &ResolvedRigPose) -> f32 {
		Self::REST_HALF_WIDTH * Self::girdle_scale(pose)
	}

	fn max_length(pose: &ResolvedRigPose, bones: &[&str]) -> f32 {
		bones.iter().map(|bone| pose.scale_for_bone(bone).y).fold(0.0, f32::max)
	}

	fn max_extent(pose: &ResolvedRigPose, bones: &[&str]) -> f32 {
		bones
			.iter()
			.map(|bone| pose.scale_for_bone(bone).max_element())
			.fold(0.0, f32::max)
	}

	pub fn local_transform(self) -> Transform {
		Transform {
			translation: Vec3::new(0.0, 0.0, self.along),
			rotation: Quat::from_rotation_x(FRAC_PI_2),
			scale: Vec3::ONE,
		}
	}

	/// Parent-local Z of the aft tip (hind + tail).
	pub fn aft_extent(self) -> f32 {
		self.along - (self.length * 0.5 + self.radius)
	}
}

/// Query-only vertical hull for an oversized orthograde head.
///
/// Radius follows XZ socket scale. Cylinder length follows Y so a tall head
/// (Spibmom ears) is a capsule, not a uniformly scaled sphere.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub struct HeadCapsule {
	pub radius: f32,
	pub length: f32,
	/// Parent-local Y of the capsule center.
	pub above: f32,
}

impl HeadCapsule {
	/// Stock head half-extent (motor top hemisphere).
	pub const REST_HALF: f32 = LocomotionCapsule::HUMANOID.radius;

	pub fn for_hull(hull: LocomotionCapsule) -> Option<Self> {
		let sx = hull.head.x.max(0.0);
		let sy = hull.head.y.max(0.0);
		let sz = hull.head.z.max(0.0);
		let radial = sx.max(sz);
		if sy <= 1.0 + 1e-4 && radial <= 1.0 + 1e-4 {
			return None;
		}
		let radius = Self::REST_HALF * radial.max(1.0);
		// Y grows the *cylinder*, not the radius. `2*sy - 1` keeps a visible
		// stem at 2× (length 1.6 m) instead of a near-sphere pill.
		let half = Self::REST_HALF * (2.0 * sy - 1.0).max(1.0);
		let half = half.max(radius);
		let socket = hull.half_height() - Self::REST_HALF;
		Some(Self { radius, length: (half - radius) * 2.0, above: socket + half })
	}

	pub fn local_transform(self) -> Transform {
		Transform::from_translation(Vec3::new(0.0, self.above, 0.0))
	}

	/// Parent-local Y of the top of the head volume.
	pub fn crown_y(self) -> f32 {
		self.above + self.length * 0.5 + self.radius
	}
}

impl LocomotionCapsule {
	/// Standing ~1.8 m humanoid (Braidman and other unscaled bipeds).
	pub const HUMANOID: Self =
		Self { radius: 0.4, length: 1.0, pronograde: false, girdle: 0.0, head: Vec3::ONE };
	/// Low vertical stand-in for a quadruped (not a horizontal body hull).
	pub const QUADRUPED: Self = Self {
		radius: 0.35,
		length: 0.4,
		pronograde: true,
		girdle: HitCapsule::REST_HALF_WIDTH,
		head: Vec3::ONE,
	};
	const GROUND_CLEARANCE: f32 = 0.15;

	pub fn scaled(self, scale: f32) -> Self {
		Self {
			radius: self.radius * scale.max(0.0),
			length: self.length * scale.max(0.0),
			pronograde: self.pronograde,
			girdle: self.girdle * scale.max(0.0),
			head: self.head,
		}
	}

	/// Stretch the cylinder so the capsule bottom sits at `-half_height`.
	pub fn with_half_height(self, half_height: f32) -> Self {
		let half = half_height.max(self.radius);
		Self {
			radius: self.radius,
			length: (half - self.radius) * 2.0,
			pronograde: self.pronograde,
			girdle: self.girdle,
			head: self.head,
		}
	}

	pub fn with_girdle(self, girdle: f32) -> Self {
		Self { girdle: girdle.max(0.0), ..self }
	}

	/// Rest-pose half-width from composed shoulder / hip / torso bone scales.
	pub fn with_pose_girdle(self, pose: &ResolvedRigPose) -> Self {
		if self.pronograde {
			self.with_girdle(HitCapsule::half_width_from(pose))
		} else {
			self
		}
	}

	const HUMANOID_LEG_BONES: &'static [&'static str] = &["femur.L", "femur.R", "shin.L", "shin.R"];
	const HUMANOID_SPINE_BONES: &'static [&'static str] = &["lumbar", "chest"];
	const HUMANOID_NECK_BONES: &'static [&'static str] = &["lower_neck", "upper_neck", "neck"];
	const HUMANOID_WIDTH_BONES: &'static [&'static str] =
		&["shoulder.L", "shoulder.R", "pelvis.L", "pelvis.R"];
	const HUMANOID_LEG_WEIGHT: f32 = 0.50;
	const HUMANOID_SPINE_WEIGHT: f32 = 0.28;
	const HUMANOID_NECK_WEIGHT: f32 = 0.10;
	/// Share of standing height treated as the head (socket scale above 1.0).
	pub const HUMANOID_HEAD_WEIGHT: f32 = 0.12;

	fn max_bone_length(pose: &ResolvedRigPose, bones: &[&str]) -> f32 {
		bones.iter().map(|bone| pose.scale_for_bone(bone).y).fold(0.0, f32::max)
	}

	/// Weighted rest-pose stature: legs, spine, neck, plus head-socket scale.
	pub fn humanoid_height_scale(pose: &ResolvedRigPose, head_scale: f32) -> f32 {
		Self::HUMANOID_LEG_WEIGHT * Self::max_bone_length(pose, Self::HUMANOID_LEG_BONES)
			+ Self::HUMANOID_SPINE_WEIGHT * Self::max_bone_length(pose, Self::HUMANOID_SPINE_BONES)
			+ Self::HUMANOID_NECK_WEIGHT * Self::max_bone_length(pose, Self::HUMANOID_NECK_BONES)
			+ Self::HUMANOID_HEAD_WEIGHT * head_scale.max(0.0)
	}

	/// Rest-pose half-width scale from shoulder / pelvis length.
	pub fn humanoid_width_scale(pose: &ResolvedRigPose) -> f32 {
		Self::max_bone_length(pose, Self::HUMANOID_WIDTH_BONES)
	}

	/// Standing hull from composed humanoid bone scales. One vertical capsule.
	///
	/// `head_y` is socket height only; XZ stay 1.0 so the query hull is a
	/// vertical capsule rather than a fatter sphere.
	pub fn humanoid_from_pose(pose: &ResolvedRigPose, head_y: f32) -> Self {
		Self::humanoid_from_pose_axes(pose, Vec3::new(1.0, head_y.max(0.0), 1.0))
	}

	pub fn humanoid_from_pose_axes(pose: &ResolvedRigPose, head: Vec3) -> Self {
		let head = head.max(Vec3::ZERO);
		let width = Self::humanoid_width_scale(pose);
		let height = Self::humanoid_height_scale(pose, head.y);
		if (width - 1.0).abs() < 1e-5 && (height - 1.0).abs() < 1e-5 && head == Vec3::ONE {
			return Self::HUMANOID;
		}
		let radius = Self::HUMANOID.radius * width;
		let half = (Self::HUMANOID.half_height() * height).max(radius);
		Self { radius, length: (half - radius) * 2.0, pronograde: false, girdle: 0.0, head }
	}

	/// Extra motor height plus a **Y-only** head socket (whelps / Spibmom).
	pub fn with_head_scale(self, head_y: f32) -> Self {
		let extra = (head_y - 1.0).max(0.0) * Self::HUMANOID_HEAD_WEIGHT;
		self.with_half_height(self.half_height() * (1.0 + extra)).with_head(Vec3::new(
			1.0,
			head_y.max(0.0),
			1.0,
		))
	}

	pub fn with_head(self, head: Vec3) -> Self {
		Self { head: head.max(Vec3::ZERO), ..self }
	}

	/// Horizontal hit hull when this is a quadruped motor stand-in.
	pub fn hit_capsule(self) -> Option<HitCapsule> {
		self.pronograde.then(|| HitCapsule::for_quadruped(self))
	}

	/// Vertical hit hull when the head socket is larger than stock.
	pub fn head_capsule(self) -> Option<HeadCapsule> {
		HeadCapsule::for_hull(self)
	}

	/// Rest-pose feet below the visual/capsule origin.
	///
	/// Thigh + shin at rest (`character_rigs::quadruped::LegSegmentLengths`), times
	/// the species / slider / lanky length product.
	pub fn quadruped_feet_below_origin(limb_scale: f32) -> f32 {
		let legs = character_rigs::quadruped::LegSegmentLengths::default();
		(legs.upper + legs.lower) * limb_scale.max(0.0)
	}

	/// Standing quadruped hull whose bottom matches rest-pose foot depth.
	///
	/// `limb_scale` is the product of species baseline, slider, and lanky length
	/// layers (`1.0` = stock thigh+shin).
	pub fn quadruped_for_limb_length(limb_scale: f32) -> Self {
		Self::QUADRUPED.with_half_height(Self::quadruped_feet_below_origin(limb_scale))
	}

	pub fn half_height(self) -> f32 {
		self.radius + self.length * 0.5
	}

	pub fn spawn_height(self) -> f32 {
		self.half_height() + Self::GROUND_CLEARANCE
	}

	/// Upper half of the top hemisphere, in local Y of the capsule origin.
	pub fn headshot_min_local_y(self) -> f32 {
		self.length * 0.5 + self.radius * 0.5
	}

	/// Squat: same radius, shorter vertical pill. `drop` from
	/// `Squat::peak_vertical_drop(LegSegmentLengths)` so the top matches the pose.
	pub fn squat(self, drop: f32) -> Self {
		self.with_half_height((self.half_height() - drop.max(0.0)).max(self.radius))
	}

	/// Prone motor: Y-up stand-in (sphere / short pill) so CharacterController
	/// and the NEG_Y shape caster stay valid. Height ≈ `2 * radius` (~0.8 m).
	pub fn prone_motor(self) -> Self {
		Self { length: 0.0, ..self }
	}

	/// Query hull along mesh +Z (nose). Same pattern as pronograde [`HitCapsule`].
	pub fn prone_hit_capsule(self) -> HitCapsule {
		let height = self.half_height() * 2.0;
		HitCapsule {
			radius: self.radius,
			length: (height - 2.0 * self.radius).max(0.0),
			along: 0.0,
		}
	}

	/// Keep feet planted when the centered capsule changes half-height.
	pub fn origin_delta(self, next: Self) -> Vec3 {
		Vec3::new(0.0, next.half_height() - self.half_height(), 0.0)
	}
}

impl Default for LocomotionCapsule {
	fn default() -> Self {
		Self::HUMANOID
	}
}

/// Domain IR exposed by a character (or character wrapper) for structural composition.
pub trait CharacterComponents {
	fn rig_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<RigNode> {
		Layers::new()
	}

	fn part_nodes_for_level(&self, _level: LodSceneLevel) -> Layers<PartNode> {
		Layers::new()
	}

	/// Physics hull baked from rest-pose proportions (species / slider / lanky
	/// limb length), not the animated mesh.
	fn locomotion_capsule(&self) -> LocomotionCapsule {
		LocomotionCapsule::HUMANOID
	}
}

/// Config → inner recipe plus clothing. Clothing is never part of the inner species.
pub trait CharacterRecipe {
	type Components: CharacterComponents + Clone + Send + Sync + 'static;

	fn components(&self) -> Self::Components;

	fn clothing_layers(&self) -> Vec<ClothingLayer>;

	fn clothed(&self) -> Clothed<Self::Components> {
		Clothed::new(self.components(), self.clothing_layers())
	}

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.components().locomotion_capsule()
	}
}

/// Map selected clothing meshes to [`ClothingLayer`]s fitted to `host`.
pub fn clothing_layers(
	clothing: impl IntoIterator<Item = ClothingMesh>,
	host: ClothingHost,
	mut material: impl FnMut(ClothingMesh) -> ClothingMaterial,
	mut color: impl FnMut(ClothingMesh) -> ItemColor,
) -> Vec<ClothingLayer> {
	clothing
		.into_iter()
		.map(|mesh| ClothingLayer::new(mesh, color(mesh), host).with_material(material(mesh)))
		.collect()
}

impl<T: CharacterComponents + ?Sized> CharacterComponents for &T {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		(**self).rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		(**self).part_nodes_for_level(level)
	}

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		(**self).locomotion_capsule()
	}
}

/// Clothing layers composed over an inner [`CharacterComponents`] recipe.
#[derive(Debug, Clone, PartialEq)]
pub struct ClothingLayer {
	pub mesh: ClothingMesh,
	pub color: ItemColor,
	pub material: ClothingMaterial,
	pub host: ClothingHost,
}

impl ClothingLayer {
	pub fn new(mesh: ClothingMesh, color: ItemColor, host: ClothingHost) -> Self {
		Self { mesh, color, material: ClothingMaterial::Cloth, host }
	}

	pub fn with_material(mut self, material: ClothingMaterial) -> Self {
		self.material = material;
		self
	}

	pub fn part_node(&self) -> PartNode {
		self.part_node_skinned(true)
	}

	/// Bind-pose mesh with no body armature — used for isolated item previews.
	pub fn preview_part_node(&self) -> PartNode {
		self.part_node_skinned(false)
	}

	fn part_node_skinned(&self, skinned: bool) -> PartNode {
		let node = PartNode::glb(
			CharacterPartSlot::Clothing,
			self.mesh.label(),
			self.mesh.path_on(self.host),
			AssetNormalization::IDENTITY,
		)
		.with_material(
			MaterialRef::named(self.material.recipe_id()).with_palette([self.color.color()]),
		);
		if skinned {
			node.skinned(SkinRef::to(RigId::Body))
		} else {
			node
		}
	}
}

/// Higher-order character: inner recipe plus clothing parts under `"clothing"`.
#[derive(Debug, Clone, PartialEq)]
pub struct Clothed<T> {
	pub inner: T,
	pub clothing: Vec<ClothingLayer>,
}

impl<T> Clothed<T> {
	pub fn new(inner: T, clothing: Vec<ClothingLayer>) -> Self {
		Self { inner, clothing }
	}
}

impl<T: CharacterComponents> CharacterComponents for Clothed<T> {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.inner.rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		let mut out = self.inner.part_nodes_for_level(level);
		if !self.clothing.is_empty() {
			out.extend_under(
				"clothing",
				Layers::from_free(self.clothing.iter().map(ClothingLayer::part_node).collect()),
			);
		}
		out
	}

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.inner.locomotion_capsule()
	}
}

/// Newtype: present a [`CharacterComponents`] value as a structural [`LodScene`] host.
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

impl<T: Default> Default for Clothed<T> {
	fn default() -> Self {
		Self { inner: T::default(), clothing: Vec::new() }
	}
}

impl<T: Send + Sync + 'static> std::ops::Deref for ComponentsOnly<T> {
	type Target = T;

	fn deref(&self) -> &T {
		&self.0
	}
}

impl<T: CharacterComponents + Send + Sync + 'static> CharacterComponents for ComponentsOnly<T> {
	fn rig_nodes_for_level(&self, level: LodSceneLevel) -> Layers<RigNode> {
		self.0.rig_nodes_for_level(level)
	}

	fn part_nodes_for_level(&self, level: LodSceneLevel) -> Layers<PartNode> {
		self.0.part_nodes_for_level(level)
	}

	fn locomotion_capsule(&self) -> LocomotionCapsule {
		self.0.locomotion_capsule()
	}
}

impl<T: CharacterComponents + Send + Sync + 'static> LodScene for ComponentsOnly<T> {
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
		character_scene_chunks(&self.0, lod_ref, level)
	}

	fn scene_bounds(&self) -> Aabb3d {
		character_bounds(&self.0)
	}

	fn host_contents(&self, lod_ref: &LodRef) -> impl Scene + 'static
	where
		Self: Component + Clone + Default + Unpin + Sized,
	{
		let level = self.scene_lod_level(lod_ref);
		let policy = motion_policy(level);
		let host = self.clone();
		(
			bsn! {
				template_value(host)
				AssemblyRoot
				CharacterRoot
				CharacterHeading::default()
				Visibility::default()
			},
			maybe_component(policy.apply_terrain_pitch()),
		)
	}
}

/// Weighted chunks for one structural level: nested rig/part hosts only.
///
/// Motion markers live on the character / body **host** and are synced from the
/// shown LOD band — not stamped into these chunks.
pub fn character_scene_chunks(
	character: &impl CharacterComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> SceneChunk {
	let mut chunks = Vec::new();
	for node in character.rig_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	for node in character.part_nodes_for_level(level).flatten() {
		chunks.push(SceneChunk::weighted(1, node.host(lod_ref)));
	}
	if chunks.is_empty() {
		SceneChunk::primitive(scene_children(Vec::new()))
	} else {
		SceneChunk::chunks(chunks)
	}
}

pub fn append_component_scenes(
	character: &impl CharacterComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
	children: &mut Vec<Box<dyn Scene>>,
) {
	for node in character.rig_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
	for node in character.part_nodes_for_level(level).flatten() {
		children.push(Box::new(node.host(lod_ref)));
	}
}

pub fn component_only_scene(
	character: &impl CharacterComponents,
	lod_ref: &LodRef,
	level: LodSceneLevel,
) -> impl Scene + 'static {
	let mut children: Vec<Box<dyn Scene>> = Vec::new();
	append_component_scenes(character, lod_ref, level, &mut children);
	scene_children(children)
}

/// Approximate AABB for a standing humanoid (High can be large; bands are identical).
pub fn character_bounds(_character: &impl CharacterComponents) -> Aabb3d {
	Aabb3d::from_min_max(Vec3::new(-1.5, -0.25, -1.5), Vec3::new(1.5, 2.75, 1.5))
}

#[cfg(test)]
mod tests {
	use super::*;
	use anyhow::{anyhow, Result};

	#[test]
	fn humanoid_hull_matches_the_legacy_capsule() {
		let hull = LocomotionCapsule::HUMANOID;
		assert!((hull.radius - 0.4).abs() < 1e-5);
		assert!((hull.length - 1.0).abs() < 1e-5);
		assert!(!hull.pronograde);
		assert!(hull.hit_capsule().is_none());
		assert!((hull.half_height() - 0.9).abs() < 1e-5);
		assert!((hull.spawn_height() - 1.05).abs() < 1e-5);
		assert!((hull.headshot_min_local_y() - 0.7).abs() < 1e-5);
	}

	#[test]
	fn scaled_hull_keeps_proportions() {
		let hull = LocomotionCapsule::HUMANOID.scaled(0.30);
		assert!((hull.radius - 0.12).abs() < 1e-5);
		assert!((hull.length - 0.30).abs() < 1e-5);
		assert!(!hull.pronograde);
		assert!(hull.hit_capsule().is_none());
	}

	#[test]
	fn quadruped_limb_hull_matches_rest_pose_foot_depth() {
		let hull = LocomotionCapsule::quadruped_for_limb_length(1.35);
		assert!((hull.radius - LocomotionCapsule::QUADRUPED.radius).abs() < 1e-5);
		assert!((hull.girdle - HitCapsule::REST_HALF_WIDTH).abs() < 1e-5);
		assert!(hull.pronograde);
		assert!((hull.half_height() - 1.35).abs() < 1e-5);
		assert!(
			(hull.half_height() - LocomotionCapsule::quadruped_feet_below_origin(1.35)).abs()
				< 1e-5
		);
	}

	#[test]
	fn quadruped_hit_capsule_covers_hind_and_tail() -> Result<()> {
		let stock = LocomotionCapsule::QUADRUPED
			.hit_capsule()
			.ok_or_else(|| anyhow!("quadruped motor hull is pronograde"))?;
		assert!((stock.radius - HitCapsule::REST_HALF_WIDTH).abs() < 1e-5);
		assert!((stock.along + HitCapsule::REST_TAIL * 0.5).abs() < 1e-5);
		assert!(
			(stock.aft_extent() + HitCapsule::REST_HALF_SPAN + HitCapsule::REST_TAIL).abs() < 1e-5
		);
		assert!(stock.aft_extent() < -HitCapsule::REST_HALF_SPAN);

		let claber = LocomotionCapsule::QUADRUPED
			.scaled(1.6)
			.hit_capsule()
			.ok_or_else(|| anyhow!("scaled quadruped stays pronograde"))?;
		assert!((claber.radius - HitCapsule::REST_HALF_WIDTH * 1.6).abs() < 1e-5);
		assert!(
			(claber.aft_extent() + (HitCapsule::REST_HALF_SPAN + HitCapsule::REST_TAIL) * 1.6)
				.abs() < 1e-5
		);

		let tall = LocomotionCapsule::quadruped_for_limb_length(1.35)
			.hit_capsule()
			.ok_or_else(|| anyhow!("limb-length hull stays pronograde"))?;
		assert!((tall.radius - HitCapsule::REST_HALF_WIDTH).abs() < 1e-5);
		assert!((tall.aft_extent() - stock.aft_extent()).abs() < 1e-5);
		Ok(())
	}

	#[test]
	fn hit_girdle_follows_composed_bone_scales() {
		use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

		let wide = ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("wide hips")
				.with_scale(BoneScale::length("hip.L", 1.4))
				.with_scale(BoneScale::length("hip.R", 1.4)),
		);
		assert!((HitCapsule::girdle_scale(&wide) - 1.4).abs() < 1e-5);
		let hull = LocomotionCapsule::QUADRUPED.with_pose_girdle(&wide);
		assert!((hull.girdle - HitCapsule::REST_HALF_WIDTH * 1.4).abs() < 1e-5);
		assert!((hull.radius - LocomotionCapsule::QUADRUPED.radius).abs() < 1e-5);

		let barrel = ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("torso")
				.with_scale(BoneScale::thickness("anterior_mid_back", 1.35))
				.with_scale(BoneScale::length("hip.L", 1.1)),
		);
		assert!((HitCapsule::girdle_scale(&barrel) - 1.35).abs() < 1e-5);

		let fleshed = ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("flesh")
				.with_scale(BoneScale::length("shoulder.L", 1.2))
				.with_scale(BoneScale::uniform("lateral_shoulder_protrusion.L", 1.1)),
		);
		assert!((HitCapsule::girdle_scale(&fleshed) - 1.2 * 1.1).abs() < 1e-5);
	}

	#[test]
	fn humanoid_from_pose_grows_with_legs_shoulders_and_head() -> Result<()> {
		use character_rigs::{BoneScale, ResolvedRigPose, RigPoseLayer};

		let stock = LocomotionCapsule::humanoid_from_pose(&ResolvedRigPose::new(), 1.0);
		assert_eq!(stock, LocomotionCapsule::HUMANOID);

		let tall = ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("legs")
				.with_scale(BoneScale::length("femur.L", 1.2))
				.with_scale(BoneScale::length("femur.R", 1.2)),
		);
		let tall = LocomotionCapsule::humanoid_from_pose(&tall, 1.0);
		assert!(tall.half_height() > LocomotionCapsule::HUMANOID.half_height());
		assert!((tall.radius - LocomotionCapsule::HUMANOID.radius).abs() < 1e-5);

		let wide = ResolvedRigPose::new().with_layer(
			RigPoseLayer::new("shoulders")
				.with_scale(BoneScale::length("shoulder.L", 2.0))
				.with_scale(BoneScale::length("shoulder.R", 2.0)),
		);
		let wide = LocomotionCapsule::humanoid_from_pose(&wide, 1.0);
		assert!((wide.radius - LocomotionCapsule::HUMANOID.radius * 2.0).abs() < 1e-5);

		let headed = LocomotionCapsule::humanoid_from_pose(&ResolvedRigPose::new(), 2.0);
		assert!(headed.half_height() > LocomotionCapsule::HUMANOID.half_height());
		assert!(headed.hit_capsule().is_none());
		let head = headed
			.head_capsule()
			.ok_or_else(|| anyhow!("oversized socket should get a head volume"))?;
		assert!((head.radius - HeadCapsule::REST_HALF).abs() < 1e-5);
		assert!((head.length - HeadCapsule::REST_HALF * 4.0).abs() < 1e-5);
		assert!(head.crown_y() > headed.half_height() + HeadCapsule::REST_HALF * 2.0);

		let whelp = LocomotionCapsule::HUMANOID.scaled(0.30).with_head_scale(1.85);
		assert!(whelp.half_height() > LocomotionCapsule::HUMANOID.scaled(0.30).half_height());
		assert!(whelp.head_capsule().is_some());
		assert!(LocomotionCapsule::HUMANOID.head_capsule().is_none());
		Ok(())
	}
}
