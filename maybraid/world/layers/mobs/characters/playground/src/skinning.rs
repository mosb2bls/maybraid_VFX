//! Bone dump against the live [`characters`] bone map.

use bevy::prelude::*;
use characters::{BoneMap, CharacterRig};

#[derive(Resource, Default)]
pub struct DumpBonesRequest(pub bool);

pub fn dump_bones_to_console(
	mut request: ResMut<DumpBonesRequest>,
	mut console: ResMut<game_commands::command::CommandConsoleOutput>,
	rig_roots: Query<(Entity, &Children, &BoneMap, &CharacterRig)>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
) {
	if !request.0 {
		return;
	}
	request.0 = false;

	let mut output = String::from("Rig bone hierarchies:\n");
	let mut any = false;
	for (_rig_root, children, bone_map, rig) in &rig_roots {
		any = true;
		output.push_str(&format!("--- {:?} rig ---\n", rig.role));
		if bone_map.by_name.is_empty() {
			output.push_str("(bone map empty; scene may still be loading)\n");
			continue;
		}
		for child in children.iter() {
			append_bone_tree(child, 0, &mut output, &children_q, &names_q);
		}
	}

	if !any {
		output.push_str("No character rig spawned.\n");
	}

	console.0 = output;
}

fn append_bone_tree(
	entity: Entity,
	indent: usize,
	output: &mut String,
	children_q: &Query<&Children>,
	names_q: &Query<&Name>,
) {
	if let Ok(name) = names_q.get(entity) {
		output.push_str(&format!("{:indent$}{name}\n", "", indent = indent * 2));
	}

	if let Ok(children) = children_q.get(entity) {
		for child in children.iter() {
			append_bone_tree(child, indent + 1, output, children_q, names_q);
		}
	}
}
