//! Skin-remap fulfill for [`crate::socket::SkinRef`] and GLTF duplicate-armature prune.

use std::collections::HashSet;

use bevy::{
	mesh::skinning::SkinnedMesh,
	prelude::*,
	world_serialization::{WorldInstance, WorldInstanceSpawner},
};

use crate::member::{find_member_rig, CharacterMembers, MemberOf};
use crate::rig::{
	BoneMap, CharacterPart, CharacterRig, NeedsDuplicateScenePrune, NeedsSkinRemap,
	NoMatchingArmature, PartRigRef,
};
use crate::socket::{SkinRefApplied, SkinRefRoot};

/// Drop [`SkinRefApplied`] when the identity changes so fulfill re-resolves.
pub fn invalidate_changed_skin_ref_roots(
	mut commands: Commands,
	changed: Query<Entity, (Changed<SkinRefRoot>, With<SkinRefApplied>)>,
) {
	for entity in &changed {
		commands.entity(entity).remove::<SkinRefApplied>();
	}
}

pub fn fulfill_skin_ref_roots(
	mut commands: Commands,
	pending: Query<(Entity, &SkinRefRoot), (With<CharacterPart>, Without<SkinRefApplied>)>,
	member_of: Query<&MemberOf>,
	members: Query<&CharacterMembers>,
	rigs: Query<(Entity, &CharacterRig, &BoneMap)>,
) {
	for (entity, SkinRefRoot(skin)) in &pending {
		let Ok(MemberOf(root)) = member_of.get(entity) else {
			continue;
		};
		let Ok(character_members) = members.get(*root) else {
			continue;
		};
		let Some((rig_root, map)) = find_member_rig(character_members, skin.target.role(), &rigs)
		else {
			continue;
		};
		if map.by_name.is_empty() {
			continue;
		}
		commands
			.entity(entity)
			.insert((PartRigRef { rig_root }, NeedsSkinRemap, SkinRefApplied));
	}
}

pub fn remap_part_skin_to_rig(
	mut commands: Commands,
	part_roots: Query<
		(Entity, &Children, &CharacterPart, &PartRigRef, &NeedsSkinRemap),
		(With<CharacterPart>, Without<NoMatchingArmature>),
	>,
	rig_maps: Query<&BoneMap, With<CharacterRig>>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
	mut skinned_meshes: Query<&mut SkinnedMesh>,
	scene_instances: Query<&WorldInstance>,
	scene_spawner: Res<WorldInstanceSpawner>,
) {
	for (part_root, children, part, rig_ref, _needs_remap) in &part_roots {
		let Ok(rig_map) = rig_maps.get(rig_ref.rig_root) else {
			continue;
		};
		if rig_map.by_name.is_empty() {
			continue;
		}

		let mut stack: Vec<Entity> = children.iter().collect();
		let mut any_skinned = false;
		let mut all_meshes_ok = true;
		let mut missing_joints = HashSet::new();
		let mut remapped_meshes = Vec::new();

		while let Some(entity) = stack.pop() {
			if let Ok(mut skin) = skinned_meshes.get_mut(entity) {
				any_skinned = true;
				let mut new_joints = Vec::with_capacity(skin.joints.len());
				let mut mesh_ok = true;

				for old_joint in &skin.joints {
					let Ok(old_name) = names_q.get(*old_joint) else {
						mesh_ok = false;
						continue;
					};

					match rig_map.by_name.get(old_name.as_str()) {
						Some(new_joint) => new_joints.push(*new_joint),
						None => {
							missing_joints.insert(old_name.to_string());
							mesh_ok = false;
						}
					}
				}

				if mesh_ok && new_joints.len() == skin.joints.len() {
					skin.joints = new_joints;
					remapped_meshes.push(entity);
				} else {
					all_meshes_ok = false;
				}
			}

			if let Ok(children) = children_q.get(entity) {
				stack.extend(children.iter());
			}
		}

		if !any_skinned {
			let mut instance_ready = false;
			let mut found_instance = false;
			let mut instance_stack = vec![part_root];
			while let Some(current) = instance_stack.pop() {
				if let Ok(instance) = scene_instances.get(current) {
					found_instance = true;
					if scene_spawner.instance_is_ready(**instance) {
						instance_ready = true;
						break;
					}
				}
				if let Ok(children) = children_q.get(current) {
					instance_stack.extend(children.iter());
				}
			}
			if !found_instance || !instance_ready {
				continue;
			}
			commands.entity(part_root).try_remove::<NeedsSkinRemap>();
			continue;
		}

		if !all_meshes_ok {
			let mut missing: Vec<_> = missing_joints.into_iter().collect();
			missing.sort();
			warn!(
				"Character part {:?} skin remap failed ({} missing rig joints): {}",
				part.slot,
				missing.len(),
				missing.join(", ")
			);
			commands
				.entity(part_root)
				.try_insert(NoMatchingArmature { missing_joints: missing });
		} else {
			for entity in &remapped_meshes {
				commands.entity(*entity).try_insert(ChildOf(part_root));
			}
			commands
				.entity(part_root)
				.try_insert(NeedsDuplicateScenePrune { keep: remapped_meshes });
		}

		commands.entity(part_root).try_remove::<NeedsSkinRemap>();
	}
}

pub fn prune_duplicate_part_scenes(
	mut commands: Commands,
	part_roots: Query<(Entity, Option<&Children>, &NeedsDuplicateScenePrune), With<CharacterPart>>,
) {
	for (part_root, children, prune) in &part_roots {
		let keep: HashSet<_> = prune.keep.iter().copied().collect();
		if let Some(children) = children {
			for child in children.iter() {
				if !keep.contains(&child) {
					commands.entity(child).try_despawn();
				}
			}
		}
		commands.entity(part_root).try_remove::<NeedsDuplicateScenePrune>();
	}
}
