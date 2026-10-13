//! Deferred socket and skin identities, parallel to [`scene_ref::SceneRef`].
//!
//! [`SocketRef`] names a bone on a semantic [`RigId`]. Fulfill is shared
//! ([`rigs`]). [`SkinRef`] names the rig that should receive a part's joint
//! remap. Lookups are scoped to the same [`crate::member::CharacterMembers`] set.

use bevy::prelude::*;

use crate::rig::CharacterRigRole;

pub use rigs::{
	fulfill_socket_ref_roots, invalidate_changed_socket_ref_roots, SocketRef, SocketRefApplied,
	SocketRefRoot,
};

/// Semantic rig identity used at scene-build time (entities do not exist yet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum RigId {
	#[default]
	Body,
	Neck,
	Head,
}

impl RigId {
	pub fn role(self) -> CharacterRigRole {
		match self {
			Self::Body => CharacterRigRole::Body,
			Self::Neck => CharacterRigRole::Neck,
			Self::Head => CharacterRigRole::Head,
		}
	}
}

impl From<RigId> for rigs::RigKey {
	fn from(id: RigId) -> Self {
		id.role().into()
	}
}

/// Deferred skin-remap target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SkinRef {
	pub target: RigId,
}

impl SkinRef {
	pub fn to(target: RigId) -> Self {
		Self { target }
	}
}

/// BSN / ECS marker: remap this part's `SkinnedMesh` joints onto [`SkinRef`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct SkinRefRoot(pub SkinRef);

/// Marker: [`SkinRefRoot`] has been resolved to a concrete rig entity.
#[derive(Component, Debug, Clone, Copy, Default)]
pub struct SkinRefApplied;
