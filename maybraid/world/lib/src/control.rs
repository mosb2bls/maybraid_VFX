//! Apply [`CharacterIntent`] to the vegetation capsule / camera-relative wish.

use bevy::prelude::*;
use durham::{
	terrain_collider_covers_xz, CascadeChunk, DurhamSurface, TerrainCellLayout,
	TerrainTrimeshCollider,
};
use game_commands::command::{CommandConsoleOutput, TextEntryFocus};
use maybraid_character_controller::CharacterIntent;
use maybraid_skill_map::SkillMapEnabled;
use maybraid_sky::{SkyDome, SKY_HORIZON};
use player::{
	apply_character_controller, Buoyant, CharacterController, CharacterStance, JumpWish, Jumping,
	LocomotionCapsule, MotorTraction, MoveWish as PlayerMoveWish, Player as MaybraidPlayer,
	PlayerCameraAim, PlayerLook, PlayerYawOwner, RestLocomotionCapsule, Sprinting, StanceKind,
	Wading,
};
use player_camera::CameraController;
use world_player::{
	MoveWish, MovementAction, Player, PlayerPhysicsEnabled, PlayerSpawnXz, PlaygroundMode,
};

use crate::map_view::WorldMapView;

/// When `false`, world movement / POV intents are ignored (menus, pause overlay).
/// Pause must not be treated as a play-session teardown.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGameplayEnabled(pub bool);

/// Explicit play-session teardown. The game shell writes this when leaving
/// to Home. Pause must not write it.
#[derive(Message, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct WorldPlaySessionEnded;

/// Keep third-person follow while the in-game inventory editor is open.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct InventoryEditCameraFollow(pub bool);

impl Default for WorldGameplayEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Sky, world player, and fog. Off on menu shells so navy clear is the preview backdrop.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldSceneryVisible(pub bool);

impl Default for WorldSceneryVisible {
	fn default() -> Self {
		Self(true)
	}
}

/// Local spawn collider + composed height are ready for Discovery drop-in.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WorldSurfaceReady(pub bool);

/// Terrain column under the player. The shell unveils after this.
#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct WorldSurfaceSet;

pub(crate) fn update_world_surface_ready(
	streaming: Res<terrain_layer_model::TerrainStreaming<durham::Durham>>,
	surface: DurhamSurface,
	spawn: Res<PlayerSpawnXz>,
	players: Query<&Transform, With<Player>>,
	colliders: Query<&CascadeChunk, With<TerrainTrimeshCollider>>,
	mut ready: ResMut<WorldSurfaceReady>,
) {
	// Menu shells keep streaming off. Leave the ready bit alone.
	if !streaming.enabled {
		return;
	}
	let xz = discovery_xz(&spawn, &players, surface.layout());
	let at = Vec3::new(xz.x, 0.0, xz.y);
	let height_ready = matches!(surface.height_at(xz), Ok(Some(_)));
	ready.0 = terrain_collider_covers_xz(at, colliders.iter()) && height_ready;
}

fn discovery_xz(
	spawn: &PlayerSpawnXz,
	players: &Query<&Transform, With<Player>>,
	layout: &TerrainCellLayout,
) -> Vec2 {
	if let Some(xz) = spawn.0 {
		return xz;
	}
	if let Ok(player) = players.single() {
		return player.translation.xz();
	}
	layout.region_center_xz().xz()
}

/// Mark the world capsule as a Lubikan owner. [`lubikans::LubikansPlugin`]
/// attaches the Self ledger in `LubikanSystems::Owner`; do not insert
/// `LubikanLedger::with_self` here. [#1112](https://github.com/ramate-io/maybraid/issues/1112)
/// can drop its temporary attach once this grant is on the stack.
pub(crate) fn grant_world_player_lubikan_owner(
	mut commands: Commands,
	players: Query<Entity, (With<Player>, Without<lubikans::LubikanOwner>)>,
) {
	for player in &players {
		commands.entity(player).insert(lubikans::LubikanOwner);
	}
}

pub(crate) fn sync_lubikan_session(
	mut ended: MessageReader<WorldPlaySessionEnded>,
	mut lives: MessageWriter<lubikans::LubikanOwnerLifetime>,
) {
	if ended.read().next().is_none() {
		return;
	}
	lives.write(lubikans::LubikanOwnerLifetime::new(
		None,
		None,
		lubikans::LubikanOwnerLifetimeKind::RestartSession,
	));
}

pub(crate) fn sync_combat_hud_visible(
	gameplay: Res<WorldGameplayEnabled>,
	hud: Option<ResMut<combat_hud::CombatHudVisible>>,
) {
	let Some(mut hud) = hud else {
		return;
	};
	if hud.0 != gameplay.0 {
		hud.0 = gameplay.0;
	}
}

pub(crate) fn sync_skill_map_enabled(
	gameplay: Res<WorldGameplayEnabled>,
	text_focus: Res<TextEntryFocus>,
	map: Option<Res<WorldMapView>>,
	mut enabled: ResMut<SkillMapEnabled>,
) {
	let next = gameplay.0 && !text_focus.0 && !map.is_some_and(|map| map.open);
	if enabled.0 != next {
		enabled.0 = next;
	}
}

pub(crate) fn apply_intents_to_movement(
	mode: Res<PlaygroundMode>,
	text_focus: Res<TextEntryFocus>,
	gameplay: Res<WorldGameplayEnabled>,
	map: Option<Res<WorldMapView>>,
	mut commands: Commands,
	mut intents: MessageReader<CharacterIntent>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut wishes: Query<&mut MoveWish, With<Player>>,
	mut player_wishes: Query<(Entity, &mut PlayerMoveWish, Has<Player>), With<CharacterController>>,
	mut movement: MessageWriter<MovementAction>,
) {
	if !gameplay.0
		|| *mode != PlaygroundMode::Character
		|| text_focus.0
		|| map.is_some_and(|map| map.open)
	{
		for _ in intents.read() {}
		for mut wish in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		for (entity, mut wish, _) in &mut player_wishes {
			wish.0 = Vec3::ZERO;
			commands.entity(entity).remove::<(JumpWish, Sprinting)>();
		}
		return;
	}

	let mut move_stick = Vec2::ZERO;
	let mut jump = false;
	for intent in intents.read() {
		match *intent {
			CharacterIntent::Move(value) => move_stick = value,
			CharacterIntent::Jump => jump = true,
			_ => {}
		}
	}

	let wish_dir = if move_stick != Vec2::ZERO {
		if let Ok(camera) = cameras.single() {
			let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
			let forward = yaw * -Vec3::Z;
			let right_dir = yaw * Vec3::X;
			(right_dir * move_stick.x + forward * move_stick.y).clamp_length_max(1.0)
		} else {
			Vec3::ZERO
		}
	} else {
		Vec3::ZERO
	};
	for mut wish in &mut wishes {
		wish.0 = wish_dir;
	}
	// Only the streamed vegetation body. A foreign [`CharacterController`] is
	// one that [`player`] already drove.
	for (entity, mut wish, world_body) in &mut player_wishes {
		if !world_body {
			continue;
		}
		wish.0 = wish_dir;
		if jump {
			commands.entity(entity).insert(JumpWish);
		}
	}

	if move_stick != Vec2::ZERO {
		movement.write(MovementAction::Move(move_stick));
	}
	if jump {
		movement.write(MovementAction::Jump);
	}
}

pub(crate) fn echo_character_intents(
	mut intents: MessageReader<CharacterIntent>,
	mut console: ResMut<CommandConsoleOutput>,
) {
	let mut parts = Vec::new();
	for intent in intents.read() {
		parts.push(match *intent {
			CharacterIntent::Move(value) => format!("move=({:.2},{:.2})", value.x, value.y),
			CharacterIntent::Look(value) => format!("look=({:.2},{:.2})", value.x, value.y),
			CharacterIntent::Focus(value) => format!("focus={value:.2}"),
			CharacterIntent::Ads(value) => format!("ads={value:.2}"),
			CharacterIntent::SkillMap => "skill-map".into(),
			CharacterIntent::UseItem(value) => format!("use={value:.2}"),
			other => other.label().to_string(),
		});
	}
	if !parts.is_empty() {
		console.0 = parts.join(" ");
	}
}

fn world_distance_fog() -> DistanceFog {
	DistanceFog {
		color: SKY_HORIZON,
		directional_light_color: Color::srgba(1.0, 0.86, 0.62, 0.4),
		directional_light_exponent: 24.0,
		falloff: FogFalloff::Linear { start: 700.0, end: 4500.0 },
	}
}

pub(crate) fn sync_world_scenery(
	visible: Res<WorldSceneryVisible>,
	mut commands: Commands,
	mut sky: Query<&mut Visibility, (With<SkyDome>, Without<Player>)>,
	mut players: Query<&mut Visibility, (With<Player>, Without<SkyDome>)>,
	cameras: Query<(Entity, Has<DistanceFog>), With<Camera3d>>,
) {
	let visibility = if visible.0 { Visibility::Inherited } else { Visibility::Hidden };
	for mut sky in &mut sky {
		*sky = visibility;
	}
	for mut player in &mut players {
		*player = visibility;
	}
	for (entity, has_fog) in &cameras {
		if visible.0 && !has_fog {
			commands.entity(entity).insert(world_distance_fog());
		} else if !visible.0 && has_fog {
			commands.entity(entity).remove::<DistanceFog>();
		}
	}
}

/// Vegetation capsule already has `ActiveCollisionHooks`; pair it with the shared motor marker.
pub(crate) fn stamp_vegetation_motor_traction(
	mut commands: Commands,
	players: Query<Entity, (With<Player>, Without<MotorTraction>)>,
) {
	for entity in &players {
		commands.entity(entity).insert(player::motor_traction_bundle());
	}
}

/// Put the world player on the player-crate capsule motor so column buoyancy runs.
pub(crate) fn stamp_world_player_motor(
	physics: Res<PlayerPhysicsEnabled>,
	mut commands: Commands,
	missing: Query<Entity, (With<Player>, Without<CharacterController>)>,
	present: Query<
		(
			Entity,
			Option<&CharacterStance>,
			Option<&RestLocomotionCapsule>,
			Option<&LocomotionCapsule>,
		),
		(With<Player>, With<CharacterController>),
	>,
) {
	if physics.0 {
		for entity in &missing {
			apply_world_player_motor(&mut commands, entity);
		}
		for (entity, stance, rest, live) in &present {
			if stance.is_none() {
				commands.entity(entity).insert(CharacterStance::settled(StanceKind::Stand));
			}
			if rest.is_none() {
				commands.entity(entity).insert(RestLocomotionCapsule(
					live.copied().unwrap_or(LocomotionCapsule::HUMANOID),
				));
			}
		}
	} else {
		for (entity, _, _, _) in &present {
			strip_world_player_motor(&mut commands, entity);
		}
	}
}

pub(crate) fn apply_world_player_motor(commands: &mut Commands, body: Entity) {
	apply_character_controller(commands, body, LocomotionCapsule::HUMANOID);
	commands.entity(body).insert((
		MaybraidPlayer,
		PlayerLook::default(),
		PlayerCameraAim::default(),
		PlayerYawOwner::Wish,
	));
}

pub(crate) fn strip_world_player_motor(commands: &mut Commands, body: Entity) {
	commands.entity(body).remove::<(
		CharacterController,
		PlayerMoveWish,
		JumpWish,
		Jumping,
		Sprinting,
		CharacterStance,
		RestLocomotionCapsule,
		player::Grounded,
		Buoyant,
		Wading,
	)>();
}

#[cfg(test)]
mod tests {
	use super::*;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{terrain_collider_covers_xz, CascadeChunk};

	#[test]
	fn surface_ready_requires_local_column() {
		let spawn = Vec3::ZERO;
		let local = CascadeChunk::unit_chunk();
		let distant = CascadeChunk {
			origin: Vec3::new(1_000.0, -2_000.0, 1_000.0),
			size: 160.0,
			..CascadeChunk::unit_chunk()
		};
		assert!(!terrain_collider_covers_xz(spawn, [&distant]));
		assert!(terrain_collider_covers_xz(spawn, [&local]));
	}

	#[test]
	fn skill_map_follows_gameplay_and_text_focus() {
		assert!(WorldGameplayEnabled(true).0 && !TextEntryFocus(false).0);
		assert!(!(WorldGameplayEnabled(false).0 && !TextEntryFocus(false).0));
		assert!(!(WorldGameplayEnabled(true).0 && !TextEntryFocus(true).0));
	}

	#[test]
	fn world_player_receives_player_crate_motor_when_physics_is_on() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(true));
		let player = world.spawn(Player).id();
		world
			.run_system_once(stamp_world_player_motor)
			.expect("stamp world player motor");
		assert!(world.get::<CharacterController>(player).is_some());
		assert!(world.get::<LocomotionCapsule>(player).is_some());
		assert!(world.get::<MaybraidPlayer>(player).is_some());
		assert!(world.get::<PlayerMoveWish>(player).is_some());
	}

	#[test]
	fn live_gameplay_does_not_drive_a_foreign_capsule() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<MovementAction>>();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(true));
		let body = world.spawn((CharacterController, PlayerMoveWish(Vec3::X))).id();
		world
			.run_system_once(apply_intents_to_movement)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<PlayerMoveWish>(body).map(|wish| wish.0), Some(Vec3::X));
		Ok(())
	}

	#[test]
	fn paused_gameplay_zeros_every_capsule_wish() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<CharacterIntent>>();
		world.init_resource::<Messages<MovementAction>>();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(TextEntryFocus(false));
		world.insert_resource(WorldGameplayEnabled(false));
		let body = world.spawn((CharacterController, PlayerMoveWish(Vec3::X))).id();
		world
			.run_system_once(apply_intents_to_movement)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(world.get::<PlayerMoveWish>(body).map(|wish| wish.0), Some(Vec3::ZERO));
		Ok(())
	}

	#[test]
	fn world_player_motor_is_stripped_when_physics_is_off() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(false));
		let player = world.spawn(Player).id();
		apply_world_player_motor(&mut world.commands(), player);
		world.flush();
		assert!(world.get::<CharacterController>(player).is_some());
		world
			.run_system_once(stamp_world_player_motor)
			.expect("strip world player motor");
		assert!(world.get::<CharacterController>(player).is_none());
		assert!(world.get::<PlayerMoveWish>(player).is_none());
	}

	#[test]
	fn stamp_restores_stance_after_it_is_stripped() {
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.insert_resource(PlayerPhysicsEnabled(true));
		let player = world.spawn(Player).id();
		apply_world_player_motor(&mut world.commands(), player);
		world.flush();
		world.entity_mut(player).remove::<CharacterStance>();
		assert!(world.get::<CharacterStance>(player).is_none());
		world.run_system_once(stamp_world_player_motor).expect("restore stance");
		assert!(world.get::<CharacterStance>(player).is_some());
		assert!(world.get::<RestLocomotionCapsule>(player).is_some());
	}

	#[test]
	fn grant_marks_vegetation_player_without_inserting_a_ledger() {
		use bevy::ecs::system::RunSystemOnce;
		use lubikans::{LubikanLedger, LubikanOwner};

		let mut world = World::new();
		let player = world.spawn(Player).id();
		world.run_system_once(grant_world_player_lubikan_owner).expect("grant owner");
		assert!(world.get::<LubikanOwner>(player).is_some());
		assert!(world.get::<LubikanLedger>(player).is_none());
		world
			.run_system_once(grant_world_player_lubikan_owner)
			.expect("grant is idempotent");
		assert!(world.get::<LubikanOwner>(player).is_some());
	}

	#[test]
	fn pause_does_not_end_the_play_session() {
		use bevy::ecs::system::RunSystemOnce;
		use lubikans::{LubikanOwnerLifetime, LubikanOwnerLifetimeKind};

		fn take_notices(
			mut notices: MessageReader<LubikanOwnerLifetime>,
		) -> Vec<LubikanOwnerLifetime> {
			notices.read().copied().collect()
		}

		let mut world = World::new();
		world.init_resource::<Messages<LubikanOwnerLifetime>>();
		world.init_resource::<Messages<WorldPlaySessionEnded>>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.run_system_once(sync_lubikan_session).expect("initial sync");
		world.resource_mut::<WorldGameplayEnabled>().0 = false;
		world.run_system_once(sync_lubikan_session).expect("paused sync");
		let paused = world.run_system_once(take_notices).expect("read pause notices");
		assert!(paused.is_empty());
		world.write_message(WorldPlaySessionEnded);
		world.run_system_once(sync_lubikan_session).expect("ended sync");
		let notices = world.run_system_once(take_notices).expect("read restart notices");
		assert_eq!(notices.len(), 1);
		assert_eq!(notices[0].kind, LubikanOwnerLifetimeKind::RestartSession);
	}
}
