//! Preview debug helpers. Socket, skin, pose, and bone maps live in `characters`.

use bevy::prelude::*;

pub use characters::{
	bind_scales_ready, bone_map_ready, missing_landmark_bones, ActiveRigPose, BoneMap,
	CharacterPart, CharacterRig, CharacterRigRole, NeedsDuplicateScenePrune, NeedsSkinRemap,
	NoMatchingArmature, ResolvedPoseApplied, RigBindScales,
};

#[derive(Resource, Default)]
pub struct DumpBonesRequest(pub bool);

pub fn request_dump_bones(commands: &mut Commands) {
	commands.queue(|world: &mut World| {
		world.resource_mut::<DumpBonesRequest>().0 = true;
	});
}

pub fn preview_debug_enabled() -> bool {
	std::env::var("CROZON_PREVIEW_DEBUG").is_ok()
}

pub fn dump_bones_to_console(
	mut request: ResMut<DumpBonesRequest>,
	mut console: ResMut<game_commands::command::CommandConsoleOutput>,
	rig_roots: Query<(Entity, &Children, &BoneMap, &CharacterRig)>,
	failed_parts: Query<(&CharacterPart, &NoMatchingArmature)>,
	children_q: Query<&Children>,
	names_q: Query<&Name>,
) {
	if !request.0 {
		return;
	}
	request.0 = false;

	let mut output = String::from("Concept rig bone hierarchies:\n");
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
		output.push_str("No concept rigs spawned.\n");
	}

	for (part, failure) in &failed_parts {
		output.push_str(&format!(
			"Part {:?} missing joints: {}\n",
			part.slot,
			failure.missing_joints.join(", ")
		));
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
