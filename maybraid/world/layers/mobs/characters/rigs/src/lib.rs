pub mod arm_reach;
pub mod articulation;
pub mod authoring;
pub mod debug;
pub mod humanoid;
pub mod pose;
pub mod quadruped;
pub mod rigs;
pub mod sliders;

pub use pose::{BoneRotation, BoneScale, BoneTranslation, ResolvedRigPose, RigPoseLayer};

use bevy::prelude::*;
use std::{collections::HashMap, fmt};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Name(pub String);

impl Name {
	pub fn new(name: impl Into<String>) -> Self {
		Self(name.into())
	}

	pub fn as_str(&self) -> &str {
		&self.0
	}
}

impl From<&str> for Name {
	fn from(value: &str) -> Self {
		Self::new(value)
	}
}

impl From<String> for Name {
	fn from(value: String) -> Self {
		Self::new(value)
	}
}

impl fmt::Display for Name {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		self.0.fmt(f)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
	Left,
	Right,
}

impl Side {
	pub fn suffix(self) -> &'static str {
		match self {
			Self::Left => "L",
			Self::Right => "R",
		}
	}

	pub fn opposite(self) -> Self {
		match self {
			Self::Left => Self::Right,
			Self::Right => Self::Left,
		}
	}

	/// Left = `+1`, right = `−1`. Useful for mirroring yaw / roll magnitudes.
	pub fn sign(self) -> f32 {
		match self {
			Self::Left => 1.0,
			Self::Right => -1.0,
		}
	}

	/// Left = 0, right = 1. Indexes bilateral pose arrays.
	pub fn index(self) -> usize {
		match self {
			Self::Left => 0,
			Self::Right => 1,
		}
	}

	/// Alternating gait timing. This does not choose a rotation axis or a mirror sign.
	pub fn phase_offset(self) -> f32 {
		match self {
			Self::Left => 0.0,
			Self::Right => 0.5,
		}
	}
}

/// The local axes a rigged bone uses for procedural articulation.
///
/// These are expressed in the same local space as the bone's `Transform.rotation`.
/// Animation code supplies `swing` / `flex` / `twist` magnitudes; the rig
/// decides which concrete local axes those use.
///
/// Local +Y is along the bone ([`articulation::BONE_LENGTH_AXIS`]). On
/// [`Self::DEFAULT`] that means **swing** is roll about the length, **flex**
/// is side-to-side, and **twist** is up / down.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RiggedAxis {
	pub swing_axis: Vec3,
	pub flex_axis: Vec3,
	pub twist_axis: Vec3,
}

impl Default for RiggedAxis {
	fn default() -> Self {
		Self::DEFAULT
	}
}

impl RiggedAxis {
	pub const DEFAULT: Self = Self { swing_axis: Vec3::Y, flex_axis: Vec3::Z, twist_axis: Vec3::X };
}

#[derive(Debug, Clone, PartialEq)]
pub struct BoneDefinition {
	/// Typically used as a constant to reference the loaded bone.
	pub name: Name,
	/// The orientation of the bone in the rig, relative to the intended geometry.
	///
	/// Often this will just be the default.
	pub relative_axis: RiggedAxis,
}

/// Backwards-compatible short name for static bone metadata.
pub type Bone = BoneDefinition;

#[derive(Debug, Clone)]
pub struct BoneTable(HashMap<Name, Bone>);

impl BoneTable {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn insert(&mut self, bone: Bone) {
		self.0.insert(bone.name.clone(), bone);
	}

	pub fn get(&self, name: &Name) -> Option<&Bone> {
		self.0.get(name)
	}

	pub fn get_mut(&mut self, name: &Name) -> Option<&mut Bone> {
		self.0.get_mut(name)
	}

	pub fn remove(&mut self, name: &Name) {
		self.0.remove(name);
	}

	pub fn iter(&self) -> impl Iterator<Item = &Bone> {
		self.0.values()
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}
}

impl Default for BoneTable {
	fn default() -> Self {
		Self::new()
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct BonePose {
	pub name: Name,
	pub transform: Transform,
	/// Forward/back swing magnitude (radians) about the bone's swing axis.
	pub swing: f32,
	/// Pitch or hinge flex magnitude (radians) about the bone's flex axis.
	pub flex: f32,
	/// Roll or long-axis twist magnitude (radians) about the bone's twist axis.
	pub twist: f32,
}

impl BonePose {
	pub fn new(name: impl Into<Name>, transform: Transform) -> Self {
		Self { name: name.into(), transform, swing: 0.0, flex: 0.0, twist: 0.0 }
	}

	pub fn with_articulation(name: impl Into<Name>, swing: f32, flex: f32) -> Self {
		Self { name: name.into(), transform: Transform::IDENTITY, swing, flex, twist: 0.0 }
	}

	pub fn with_pose(name: impl Into<Name>, swing: f32, flex: f32, translation: Vec3) -> Self {
		Self {
			name: name.into(),
			transform: Transform::from_translation(translation),
			swing,
			flex,
			twist: 0.0,
		}
	}

	/// Apply swing/flex/twist about this bone's rig-defined local axes.
	pub fn articulate(mut self, axis: RiggedAxis, swing: f32, flex: f32, twist: f32) -> Self {
		self.swing = swing;
		self.flex = flex;
		self.twist = twist;
		let rest = self.transform.rotation;
		self.transform.rotation =
			articulation::compose_parent_rotation(rest, axis, swing, flex, twist);
		self
	}

	/// Copy transform and articulation from `other`, leaving `self.name` unchanged.
	pub fn copy_fields_from(&mut self, other: &Self) {
		self.transform = other.transform;
		self.swing = other.swing;
		self.flex = other.flex;
		self.twist = other.twist;
	}

	/// Lerp translation/scale, slerp rotation, lerp swing/flex/twist.
	pub fn blend(&self, to: &Self, weight: f32) -> Self {
		let weight = weight.clamp(0.0, 1.0);
		Self {
			name: self.name.clone(),
			transform: Transform {
				translation: self.transform.translation.lerp(to.transform.translation, weight),
				rotation: self.transform.rotation.slerp(to.transform.rotation, weight),
				scale: self.transform.scale.lerp(to.transform.scale, weight),
			},
			swing: self.swing + (to.swing - self.swing) * weight,
			flex: self.flex + (to.flex - self.flex) * weight,
			twist: self.twist + (to.twist - self.twist) * weight,
		}
	}
}

#[derive(Debug, Clone)]
pub struct RigPose(HashMap<Name, BonePose>);

impl RigPose {
	pub fn new() -> Self {
		Self(HashMap::new())
	}

	pub fn insert(&mut self, pose: BonePose) {
		self.0.insert(pose.name.clone(), pose);
	}

	pub fn set_transform(&mut self, name: impl Into<Name>, transform: Transform) {
		let name = name.into();
		self.0
			.entry(name.clone())
			.and_modify(|pose| pose.transform = transform)
			.or_insert_with(|| BonePose::new(name, transform));
	}

	pub fn get(&self, name: &Name) -> Option<&BonePose> {
		self.0.get(name)
	}

	pub fn get_mut(&mut self, name: &Name) -> Option<&mut BonePose> {
		self.0.get_mut(name)
	}

	pub fn remove(&mut self, name: &Name) {
		self.0.remove(name);
	}

	pub fn iter(&self) -> impl Iterator<Item = (&Name, &BonePose)> {
		self.0.iter()
	}

	pub fn bone_poses(&self) -> impl Iterator<Item = BonePose> + '_ {
		self.0.values().cloned()
	}

	pub fn len(&self) -> usize {
		self.0.len()
	}

	pub fn is_empty(&self) -> bool {
		self.0.is_empty()
	}

	/// Backing map capacity (for reuse / benchmark checks).
	pub fn capacity(&self) -> usize {
		self.0.capacity()
	}

	/// Copy pose fields into an existing entry, or insert a clone when missing.
	pub fn upsert_from(&mut self, source: &BonePose) {
		if let Some(existing) = self.0.get_mut(&source.name) {
			existing.copy_fields_from(source);
		} else {
			self.0.insert(source.name.clone(), source.clone());
		}
	}

	/// Drop entries whose names are not in `retained`.
	pub fn retain_bones(&mut self, retained: &std::collections::HashSet<Name>) {
		self.0.retain(|name, _| retained.contains(name));
	}

	/// Blend two absolute poses. Missing bones are taken from the other side.
	pub fn blend(from: &Self, to: &Self, weight: f32) -> Self {
		let weight = weight.clamp(0.0, 1.0);
		let mut out = Self::new();
		let mut names: std::collections::HashSet<Name> = from.0.keys().cloned().collect();
		names.extend(to.0.keys().cloned());
		for name in names {
			match (from.get(&name), to.get(&name)) {
				(Some(a), Some(b)) => out.insert(a.blend(b, weight)),
				(Some(a), None) => out.insert(a.clone()),
				(None, Some(b)) => out.insert(b.clone()),
				(None, None) => {}
			}
		}
		out
	}

	/// Add the same translation to every named bone, preserving existing swing/flex.
	/// Inserts identity articulation for bones not yet in the pose.
	pub fn move_all(&mut self, bones: impl IntoIterator<Item = Name>, translation: Vec3) {
		for name in bones {
			self.0
				.entry(name.clone())
				.and_modify(|pose| pose.transform.translation += translation)
				.or_insert_with(|| BonePose::new(name, Transform::from_translation(translation)));
		}
	}
}

impl Default for RigPose {
	fn default() -> Self {
		Self::new()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn rig_pose_round_trips_by_name() {
		let mut pose = RigPose::new();
		let name = Name::from("femur.L");
		let transform = Transform::from_translation(Vec3::new(1.0, 2.0, 3.0));

		pose.insert(BonePose::new(name.clone(), transform));

		assert_eq!(pose.get(&name).map(|pose| pose.transform), Some(transform));
		assert_eq!(pose.len(), 1);
	}

	#[test]
	fn rig_pose_blend_midpoint_lerps_translation() {
		let name = Name::from("femur.L");
		let mut from = RigPose::new();
		from.insert(BonePose::new(name.clone(), Transform::from_translation(Vec3::ZERO)));
		let mut to = RigPose::new();
		to.insert(BonePose::new(name.clone(), Transform::from_translation(Vec3::X)));
		let mid = RigPose::blend(&from, &to, 0.5);
		let bone = mid.get(&name).expect("femur");
		assert!((bone.transform.translation - Vec3::splat(0.0).lerp(Vec3::X, 0.5)).length() < 1e-5);
	}

	#[test]
	fn rig_pose_move_all_preserves_articulation() {
		let mut pose = RigPose::new();
		let name = Name::from("femur.L");
		pose.insert(BonePose::with_pose(name.clone(), 0.5, 1.0, Vec3::X));
		pose.move_all([name.clone()], Vec3::new(0.0, -0.1, 0.0));

		let bone = pose.get(&name).expect("femur pose");
		assert_eq!(bone.swing, 0.5);
		assert_eq!(bone.flex, 1.0);
		assert_eq!(bone.transform.translation, Vec3::new(1.0, -0.1, 0.0));
	}

	#[test]
	fn rig_pose_upsert_from_reuses_entry_without_cloning_name() {
		let name = Name::from("femur.L");
		let mut pose = RigPose::new();
		pose.insert(BonePose::new(name.clone(), Transform::IDENTITY));
		let source = BonePose::with_pose(name.clone(), 0.5, 1.0, Vec3::X);
		pose.upsert_from(&source);

		let bone = pose.get(&name).expect("femur");
		assert_eq!(bone.swing, 0.5);
		assert_eq!(bone.flex, 1.0);
		assert_eq!(bone.transform.translation, Vec3::X);
	}

	#[test]
	fn rig_pose_retain_bones_drops_stale_entries() {
		let mut pose = RigPose::new();
		pose.insert(BonePose::new("femur.L", Transform::IDENTITY));
		pose.insert(BonePose::new("tibia.L", Transform::IDENTITY));
		let retained = std::collections::HashSet::from([Name::from("femur.L")]);
		pose.retain_bones(&retained);
		assert_eq!(pose.len(), 1);
		assert!(pose.get(&Name::from("femur.L")).is_some());
	}

	#[test]
	fn rig_pose_move_all_inserts_missing_bones() {
		let mut pose = RigPose::new();
		let name = Name::from("root");
		pose.move_all([name.clone()], Vec3::new(0.0, -0.2, 0.0));

		let bone = pose.get(&name).expect("root pose");
		assert_eq!(bone.transform.translation, Vec3::new(0.0, -0.2, 0.0));
		assert_eq!(bone.swing, 0.0);
	}
}
