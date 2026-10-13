//! Runtime rig markers shared by LodScene hosts and preview systems.
//!
//! Identity types the mailbox needs live in `character-motion` and are
//! re-exported here so recipe code keeps `crate::rig::…` paths.

use bevy::prelude::*;

use crate::assembly::CharacterPartSlot;

pub use character_motion::{
	bone_map_ready, missing_landmark_bones, BoneMap, CharacterRig, CharacterRigRole,
	RigSkeletonKind,
};
pub use rigs::{
	build_bone_maps as build_rig_bone_map, ActiveRigPose, BindPose as RigBindScales,
	PoseApplied as ResolvedPoseApplied,
};

pub fn bind_scales_ready(bind: &RigBindScales, map: &BoneMap, skeleton: RigSkeletonKind) -> bool {
	rigs::bind_pose_ready(bind, map, skeleton.landmark_bones())
}

/// Marks a [`CharacterRig`] that came from a [`crate::RigNode`] LodScene host.
#[derive(Component, Clone, Copy, Default)]
pub struct LodCharacterRig;

/// Marks which logical character slot an instantiated part occupies.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct CharacterPart {
	pub slot: CharacterPartSlot,
}

#[derive(Component)]
pub struct PartRigRef {
	pub rig_root: Entity,
}

#[derive(Component)]
pub struct NeedsSkinRemap;

#[derive(Component)]
pub struct NeedsDuplicateScenePrune {
	pub keep: Vec<Entity>,
}

/// Part mesh was skinned to a skeleton that does not match the active rig.
#[derive(Component, Debug)]
pub struct NoMatchingArmature {
	pub missing_joints: Vec<String>,
}
