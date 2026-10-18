//! Shared armature helpers: bone maps, socket fulfill, bind-relative pose.
//!
//! Domain crates (characters, firearms) own recipes and skeleton catalogs.
//! This crate indexes named bones, parents [`SocketRef`]s, and applies
//! [`ResolvedRigPose`] onto a captured bind pose.

pub mod bone_map;
pub mod member;
pub mod plugin;
pub mod pose;
pub mod socket;

pub use bone_map::{build_bone_maps, BoneMap, RigKey, RigRoot};
pub use member::{
	find_any_member_rig, find_member_rig, stamp_assembly_members, AssemblyHost, AssemblyMembers,
	AssemblyRoot, MemberOf,
};
pub use plugin::{RigPlugin, RigSystems};
pub use pose::{
	bind_pose_ready, maintain_bind_pose, ActiveRigPose, BindPose, BindPoseDirty, BoneRotation,
	BoneScale, BoneTranslation, PoseApplied, PoseSkipRotation, ResolvedRigPose, RigPoseLayer,
};
pub use socket::{
	fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots, resolve_socket_parent,
	SocketRef, SocketRefApplied, SocketRefRoot,
};
