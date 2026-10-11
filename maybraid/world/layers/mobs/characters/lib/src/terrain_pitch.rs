//! Re-export pitch math from `character-motion`, plus girdle prepare.

use bevy::ecs::relationship::RelationshipTarget;
use bevy::prelude::*;
use character_motion::markers::ApplyTerrainPitch;
use character_motion::{BoneMap, CharacterRig, CharacterRigRole, RigSkeletonKind};

use crate::member::{CharacterMembers, CharacterRoot};

pub use character_motion::pitch::{
	CharacterHeading, TerrainPitch, MAX_TILT, QUADRUPED_FRONT, QUADRUPED_HIND, QUADRUPED_LEFT,
	QUADRUPED_RIGHT, SUPPORT_RATE, TILT_RATE, TILT_SMOOTH,
};

/// Insert [`TerrainPitch`] on character roots that opted in with [`ApplyTerrainPitch`],
/// and refresh wheelbase from live girdles once the pose is in world space.
pub fn prepare_character_terrain_pitch(
	mut commands: Commands,
	new_visuals: Query<
		(Entity, &CharacterMembers, &Transform, Option<&CharacterHeading>),
		(With<CharacterRoot>, With<ApplyTerrainPitch>, Without<TerrainPitch>),
	>,
	mut pitched: Query<
		(&CharacterMembers, &mut TerrainPitch),
		(With<CharacterRoot>, With<ApplyTerrainPitch>),
	>,
	rigs: Query<(&CharacterRig, &BoneMap)>,
	globals: Query<&GlobalTransform>,
) {
	for (entity, members, transform, heading) in &new_visuals {
		let Some(kind) = body_skeleton(members, &rigs) else {
			continue;
		};
		let mut pitch =
			TerrainPitch::new(kind, kind.default_half_span(), kind.default_half_width());
		if let Some(bones) = body_bones(members, &rigs) {
			if let Some(samples) = quadruped_girdle_samples(kind, bones, &globals) {
				pitch.record_girdles(samples[0], samples[1], samples[2], samples[3]);
			}
		}
		let mut entity_commands = commands.entity(entity);
		// Preview / LOD refresh can despawn the host before this buffer applies.
		entity_commands.try_insert(pitch);
		if heading.is_none() {
			entity_commands.try_insert(CharacterHeading::from_rotation(transform.rotation));
		}
	}

	for (members, mut pitch) in &mut pitched {
		let Some(kind) = body_skeleton(members, &rigs) else {
			continue;
		};
		if let Some(bones) = body_bones(members, &rigs) {
			let Some(samples) = quadruped_girdle_samples(kind, bones, &globals) else {
				continue;
			};
			if !pitch.would_record_girdles(samples[0], samples[1], samples[2], samples[3]) {
				continue;
			}
			pitch.record_girdles(samples[0], samples[1], samples[2], samples[3]);
		}
	}
}

fn quadruped_girdle_samples(
	kind: RigSkeletonKind,
	bones: &BoneMap,
	globals: &Query<&GlobalTransform>,
) -> Option<[Option<Vec3>; 4]> {
	(kind == RigSkeletonKind::Quadruped).then(|| {
		[
			named_world(bones, "shoulder.L", globals),
			named_world(bones, "shoulder.R", globals),
			named_world(bones, "hip.L", globals),
			named_world(bones, "hip.R", globals),
		]
	})
}

fn body_skeleton(
	members: &CharacterMembers,
	rigs: &Query<(&CharacterRig, &BoneMap)>,
) -> Option<RigSkeletonKind> {
	body_rig(members, rigs).map(|(rig, _)| rig.skeleton)
}

fn body_bones<'a>(
	members: &CharacterMembers,
	rigs: &'a Query<(&CharacterRig, &BoneMap)>,
) -> Option<&'a BoneMap> {
	body_rig(members, rigs).map(|(_, bones)| bones)
}

fn body_rig<'a>(
	members: &CharacterMembers,
	rigs: &'a Query<(&CharacterRig, &BoneMap)>,
) -> Option<(&'a CharacterRig, &'a BoneMap)> {
	for member in members.iter() {
		let Ok((rig, bones)) = rigs.get(member) else {
			continue;
		};
		if rig.role == CharacterRigRole::Body {
			return Some((rig, bones));
		}
	}
	None
}

fn named_world(bones: &BoneMap, name: &str, globals: &Query<&GlobalTransform>) -> Option<Vec3> {
	let entity = bones.by_name.get(name)?;
	globals.get(*entity).ok().map(|g| g.translation())
}
