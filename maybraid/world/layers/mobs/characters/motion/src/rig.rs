//! Rig identity the mailbox and pitch math need. Not species recipes.

use bevy::prelude::*;

pub use rigs::BoneMap;

/// Skeleton family used to gate bone-map readiness and pitch weights.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RigSkeletonKind {
	#[default]
	Humanoid,
	Quadruped,
	Forelimbed,
	/// Multi-bone neck armature (`neck_base` … `head_socket`).
	Neck,
}

impl RigSkeletonKind {
	pub fn from_body_rig_label(label: &str) -> Self {
		match label {
			"Quadruped" => Self::Quadruped,
			"Forelimbed" => Self::Forelimbed,
			_ => Self::Humanoid,
		}
	}

	pub fn landmark_bones(self) -> &'static [&'static str] {
		match self {
			Self::Humanoid => &["root", "pelvis.L", "chest.L", "waist.L"],
			Self::Quadruped => &["head_socket", "shoulder.L", "tailbone", "waist.L"],
			Self::Forelimbed => &["head_socket", "shoulder.L", "tailbone", "upper_mid_spine"],
			Self::Neck => &["neck_base", "head_socket"],
		}
	}
}

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CharacterRigRole {
	#[default]
	Body,
	Neck,
	Head,
}

impl CharacterRigRole {
	pub const fn key(self) -> rigs::RigKey {
		match self {
			Self::Body => rigs::RigKey::named("body"),
			Self::Neck => rigs::RigKey::named("neck"),
			Self::Head => rigs::RigKey::named("head"),
		}
	}
}

impl From<CharacterRigRole> for rigs::RigKey {
	fn from(role: CharacterRigRole) -> Self {
		role.key()
	}
}

#[derive(Component, Clone, Copy, Default)]
pub struct CharacterRig {
	pub role: CharacterRigRole,
	pub skeleton: RigSkeletonKind,
}

pub fn bone_map_ready(map: &BoneMap, skeleton: RigSkeletonKind) -> bool {
	map.landmarks_ready(skeleton.landmark_bones())
}

pub fn missing_landmark_bones(map: &BoneMap, skeleton: RigSkeletonKind) -> Vec<&'static str> {
	map.missing_landmarks(skeleton.landmark_bones())
}
