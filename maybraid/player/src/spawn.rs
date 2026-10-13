//! Spawn the capsule and attach a character visual.

use bevy::ecs::query::QueryFilter;
use bevy::prelude::*;
use characters::{
	spawn_fixed_character_visual, CharacterComponents, CharacterHeading, CharacterRoot,
};

use crate::body::{apply_locomotion_capsule, spawn_character_controller};
use crate::identity::{
	CameraFollow, Npc, Player, PlayerCameraAim, PlayerCapsule, PlayerLook, PlayerVisual,
	PlayerYawOwner,
};

pub use characters::LocomotionCapsule;

/// Humanoid default; live bodies carry [`LocomotionCapsule`] from the recipe.
pub const CAPSULE_RADIUS: f32 = LocomotionCapsule::HUMANOID.radius;
pub const CAPSULE_LENGTH: f32 = LocomotionCapsule::HUMANOID.length;

pub fn capsule_spawn_height() -> f32 {
	LocomotionCapsule::HUMANOID.spawn_height()
}

pub fn spawn_player(commands: &mut Commands) -> Entity {
	spawn_player_with_hull(commands, LocomotionCapsule::HUMANOID)
}

pub fn spawn_player_with_hull(commands: &mut Commands, hull: LocomotionCapsule) -> Entity {
	let player =
		spawn_character_controller(commands, Vec3::new(0.0, hull.spawn_height(), 0.0), hull);
	commands.entity(player).insert((
		Name::new("Player"),
		Player,
		CameraFollow,
		PlayerLook::default(),
		PlayerCameraAim::default(),
		PlayerYawOwner::Wish,
	));
	player
}

pub fn spawn_npc(commands: &mut Commands, translation: Vec3, look: PlayerLook) -> Entity {
	spawn_npc_with_hull(commands, translation, look, LocomotionCapsule::HUMANOID)
}

pub fn spawn_npc_with_hull(
	commands: &mut Commands,
	translation: Vec3,
	look: PlayerLook,
	hull: LocomotionCapsule,
) -> Entity {
	let npc = spawn_character_controller(commands, translation, hull);
	commands.entity(npc).insert((Name::new("Npc"), Npc, look, PlayerYawOwner::Wish));
	npc
}

pub fn spawn_player_capsule_mesh(
	commands: &mut Commands,
	body: Entity,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	spawn_capsule_mesh(commands, body, "PlayerCapsule", meshes, materials);
}

pub fn spawn_capsule_mesh(
	commands: &mut Commands,
	body: Entity,
	name: &'static str,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	spawn_capsule_mesh_with_hull(
		commands,
		body,
		name,
		LocomotionCapsule::HUMANOID,
		meshes,
		materials,
	);
}

pub fn spawn_capsule_mesh_with_hull(
	commands: &mut Commands,
	body: Entity,
	name: &'static str,
	hull: LocomotionCapsule,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) {
	commands.spawn((
		Name::new(name),
		PlayerCapsule,
		ChildOf(body),
		Visibility::Hidden,
		Mesh3d(meshes.add(Capsule3d::new(hull.radius, hull.length))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
}

pub fn spawn_player_visual<C: CharacterComponents + Send + Sync + 'static>(
	commands: &mut Commands,
	player: Entity,
	recipe: C,
	facing: Quat,
) -> Entity {
	spawn_character_visual(commands, player, recipe, facing, "player-visual", PlayerVisual)
}

pub fn spawn_npc_visual<C: CharacterComponents + Send + Sync + 'static>(
	commands: &mut Commands,
	npc: Entity,
	recipe: C,
	facing: Quat,
) -> Entity {
	spawn_character_visual(commands, npc, recipe, facing, "npc-visual", ())
}

fn spawn_character_visual<C: CharacterComponents + Send + Sync + 'static>(
	commands: &mut Commands,
	body: Entity,
	recipe: C,
	facing: Quat,
	name: &'static str,
	extra: impl Bundle,
) -> Entity {
	let hull = recipe.locomotion_capsule();
	apply_locomotion_capsule(commands, body, hull);
	let visual = spawn_fixed_character_visual(commands, body, recipe, facing, name);
	commands.entity(visual).insert((
		CharacterHeading::from_rotation(facing),
		PlayerYawOwner::Wish,
		extra,
	));
	visual
}

pub fn spawn_player_with_hidden_capsule(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) -> Entity {
	let player = spawn_player(commands);
	spawn_player_capsule_mesh(commands, player, meshes, materials);
	player
}

pub fn spawn_npc_with_hidden_capsule(
	commands: &mut Commands,
	translation: Vec3,
	look: PlayerLook,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
) -> Entity {
	let npc = spawn_npc(commands, translation, look);
	spawn_capsule_mesh(commands, npc, "NpcCapsule", meshes, materials);
	npc
}

pub fn needs_player_visual(
	players: Query<Entity, With<Player>>,
	visuals: Query<&ChildOf, With<PlayerVisual>>,
) -> Option<Entity> {
	needs_body_visual(players, visuals)
}

pub fn needs_npc_visual(
	npcs: Query<Entity, With<Npc>>,
	visuals: Query<&ChildOf, With<CharacterRoot>>,
) -> Option<Entity> {
	needs_body_visual(npcs, visuals)
}

fn needs_body_visual<BodyFilter: QueryFilter, VisualFilter: QueryFilter>(
	bodies: Query<Entity, BodyFilter>,
	visuals: Query<&ChildOf, VisualFilter>,
) -> Option<Entity> {
	bodies.iter().find(|&body| !visuals.iter().any(|child| child.parent() == body))
}
