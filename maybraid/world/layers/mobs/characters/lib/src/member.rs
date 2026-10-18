//! Character membership: which nested hosts belong to which character root.
//!
//! [`MemberOf`] cannot be stamped from [`lod::LodScene::host`] because
//! [`lod::LodRef::entity`] is [`Entity::PLACEHOLDER`] at recipe build. Walk
//! [`ChildOf`] to [`rigs::AssemblyRoot`] **before** socket fulfill (and retries
//! until the parent chain is ready); after sockets, [`ChildOf`] is the bone and
//! [`MemberOf`] still means “this character.”

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::CommandsSceneExt;
use bevy::prelude::*;
use lod::lod_ref::LodRef;
use lod::scene::LodScene;
use rigs::AssemblyMembers;

use crate::assembly::CharacterPartSlot;
use crate::nodes::PartNode;
use crate::rig::{BoneMap, CharacterRig, CharacterRigRole};

pub use rigs::{stamp_assembly_members as stamp_character_members, MemberOf};

/// Domain marker on the character assembly host (see [`lod::LodScene::host`]).
#[derive(Component, Clone, Copy, Default)]
pub struct CharacterRoot;

/// All nested hosts that belong to a character [`AssemblyRoot`].
pub type CharacterMembers = AssemblyMembers;

/// The rig among `members` whose [`CharacterRig::role`] matches `role`.
pub fn find_member_rig<'a>(
	members: &CharacterMembers,
	role: CharacterRigRole,
	rigs: &'a Query<(Entity, &CharacterRig, &BoneMap)>,
) -> Option<(Entity, &'a BoneMap)> {
	members.iter().find_map(|member| {
		let (entity, rig, map) = rigs.get(member).ok()?;
		(rig.role == role).then_some((entity, map))
	})
}

/// First part member under `root` matching `slot`, preferring `label` when set.
pub fn find_part_member(
	root: Entity,
	slot: CharacterPartSlot,
	label: Option<&str>,
	members: &Query<&CharacterMembers>,
	parts: &Query<&PartNode>,
) -> Option<Entity> {
	let list = members.get(root).ok()?;
	let mut fallback = None;
	for member in list.iter() {
		let Ok(part) = parts.get(member) else {
			continue;
		};
		if part.slot != slot {
			continue;
		}
		match label {
			Some(label) if part.label == label => return Some(member),
			Some(_) => {
				if fallback.is_none() {
					fallback = Some(member);
				}
			}
			None => return Some(member),
		}
	}
	fallback
}

/// Hide or restore socketed part hosts whose slot matches `hide`.
pub fn hide_socketed_parts(
	members: &CharacterMembers,
	parts: &Query<&PartNode>,
	visibilities: &mut Query<&mut Visibility>,
	hide: impl Fn(CharacterPartSlot) -> bool,
	hidden: bool,
) {
	let visibility = if hidden { Visibility::Hidden } else { Visibility::Inherited };
	for member in members.iter() {
		let Ok(part) = parts.get(member) else {
			continue;
		};
		if !hide(part.slot) {
			continue;
		}
		if let Ok(mut vis) = visibilities.get_mut(member) {
			*vis = visibility;
		}
	}
}

/// Spawn `part` as a nested LodScene host parented to `root`.
///
/// [`stamp_character_members`] records [`MemberOf`]; socket fulfill then reparents
/// to the named bone.
pub fn attach_part_node(commands: &mut Commands, root: Entity, part: PartNode) -> Entity {
	let identity = Transform::IDENTITY;
	let bounds = part.scene_bounds();
	let lod_ref = LodRef {
		entity: root,
		previous_transform: &identity,
		current_transform: &identity,
		bounds: &bounds,
	};
	let entity = commands.spawn_scene(part.host(&lod_ref)).id();
	commands.entity(entity).insert(ChildOf(root));
	entity
}
