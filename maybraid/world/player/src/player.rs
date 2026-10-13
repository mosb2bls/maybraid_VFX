//! Capsule player + third-person character mode for the terrain playground.
//!
//! Movement logic follows Avian's `dynamic_character_3d` example (dynamic body,
//! shape-cast grounded check, jump impulse), with walk direction relative to
//! the camera yaw.

use std::f32::consts::PI;

use avian3d::prelude::*;
use bevy::{ecs::query::Has, prelude::*};
use durham::{
	terrain_collider_covers_xz, BaseTerrainNoise, CascadeChunk, DurhamSurface, TerrainCellLayout,
	TerrainTrimeshCollider,
};
use game_commands::command::TextEntryFocus;
use lod_avian::PhysicsInteractionLayer;
use maybraid_input::{PadButton, VirtualPad};

use crate::{camera::CameraController, WorldBaseTerrain};

pub(crate) const CAPSULE_RADIUS: f32 = 0.4;
pub(crate) const CAPSULE_LENGTH: f32 = 1.0;
const MOVE_SPEED: f32 = 7.0;
const MOVE_ACCEL: f32 = 40.0;
const MOVE_BRAKE: f32 = 50.0;
const AIR_CONTROL: f32 = 0.25;
const JUMP_IMPULSE: f32 = 8.0;
/// Default maximum uphill drive (~81°). Downhill travel remains unrestricted.
const DEFAULT_MAX_SLOPE_ANGLE: f32 = PI * 0.45;
/// Reject near-vertical side contacts as support while retaining steep cliffs.
const MIN_SUPPORT_NORMAL_Y: f32 = 0.05;
/// Third-person orbit for a ~2 m humanoid (capsule center is hip height).
pub const CAMERA_DISTANCE: f32 = 3.6;
pub const CAMERA_HEIGHT: f32 = 1.1;
pub const CAMERA_LOOK_HEIGHT: f32 = 0.65;
const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;
const PLAY_GRAVITY_SCALE: f32 = 1.25;
/// Hold above base noise until composed height exists (`height_scale` is 500).
const HOLD_ABOVE_BASE_FACTOR: f32 = 0.35;
/// Bedrock is `-4 * height_scale`. Recover only after falling through that floor.
const VOID_FLOOR_MARGIN: f32 = 500.0;
const VOID_RESPAWN_DELAY_SECS: f32 = 0.75;

/// Camera-relative WASD wish on XZ. Zero when no move input.
#[derive(Component, Default)]
pub struct MoveWish(pub Vec3);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PlayerControlSystems;

/// Grounded-walk feel. Insert before [`PlayerPlugin`] to override the default
/// (~81°) uphill limit. The limit only removes drive toward the top of steeper
/// surfaces; downhill and contour movement remain available.
#[derive(Resource, Clone, Copy, Debug, PartialEq)]
pub struct CharacterLocomotion {
	/// Uphill drive is blocked above this angle (radians from up).
	pub max_slope_angle: f32,
}

impl Default for CharacterLocomotion {
	fn default() -> Self {
		Self { max_slope_angle: DEFAULT_MAX_SLOPE_ANGLE }
	}
}

/// Playground interaction mode.
#[derive(Resource, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlaygroundMode {
	/// Free-look fly camera (default).
	#[default]
	Free,
	/// Capsule character with third-person camera.
	Character,
}

#[derive(Component)]
pub struct Player;

/// Gravity off until composed height and a terrain trimesh exist.
#[derive(Component, Clone, Copy, Debug, Default)]
pub struct AwaitingTerrainSurface;

/// Stand on a local collider instead of streamed terrain.
/// Fall recovery returns to [`Self::translation`].
#[derive(Component, Clone, Copy, Debug)]
pub struct OffTerrainAnchor {
	pub translation: Vec3,
}

/// Delayed recovery after falling through bedrock into the void.
#[derive(Resource, Debug, Clone, Copy, Default)]
pub struct PlayerRespawn {
	queued_at: Option<f32>,
}

/// Optional Discovery / playground spawn on XZ (metres). `None` uses the
/// terrain layout center.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayerSpawnXz(pub Option<Vec2>);

/// Whether the player rigid body may participate in physics.
///
/// Playgrounds default this on. The game shell keeps it off until Discovery
/// drop-in (gravity 0, no `RigidBodyDisabled` / `ColliderDisabled`).
#[derive(Resource, Debug, Clone, Copy)]
pub struct PlayerPhysicsEnabled(pub bool);

impl Default for PlayerPhysicsEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// Debug capsule mesh parented to [`Player`] (hidden when a character visual is set).
#[derive(Component)]
pub struct PlayerCapsule;

#[derive(Component)]
struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Grounded;

/// Last valid support plane.
///
/// A descending capsule can briefly outrun its down-cast between terrain
/// facets. Reuse this only while [`Grounded`] bridges that probe miss.
#[derive(Component, Clone, Copy, Debug, PartialEq)]
pub(crate) struct WalkableGround {
	pub(crate) normal: Vec3,
}

impl Default for WalkableGround {
	fn default() -> Self {
		Self { normal: Vec3::Y }
	}
}

/// Space jump is in flight. Cleared on landing, not on shapecast misses.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub struct Jumping {
	left_ground: bool,
}

#[derive(Component)]
struct MovementAcceleration(f32);

#[derive(Component)]
struct JumpImpulse(f32);

#[derive(Component)]
struct MaxSlopeAngle(f32);

#[derive(Message, Clone, Copy, Debug)]
pub enum MovementAction {
	Move(Vec2),
	Jump,
}

/// When false, this crate skips grounded / wish / jump on the capsule so a
/// downstream motor (world player buoyancy) can own velocity. Snap and
/// fall-respawn still run.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct VegetationPlayerMotor(pub bool);

impl Default for VegetationPlayerMotor {
	fn default() -> Self {
		Self(true)
	}
}

/// When false, a downstream controller writes [`MovementAction`] / [`MoveWish`].
#[derive(Resource, Clone, Copy, Debug)]
pub struct PadMovementEnabled(pub bool);

impl Default for PadMovementEnabled {
	fn default() -> Self {
		Self(true)
	}
}

/// When false, a downstream system owns the character camera (world first/third POV).
#[derive(Resource, Clone, Copy, Debug)]
pub struct CharacterCameraFollowEnabled(pub bool);

impl Default for CharacterCameraFollowEnabled {
	fn default() -> Self {
		Self(true)
	}
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PlaygroundMode>()
			.init_resource::<PadMovementEnabled>()
			.init_resource::<VegetationPlayerMotor>()
			.init_resource::<CharacterCameraFollowEnabled>()
			.init_resource::<CharacterLocomotion>()
			.init_resource::<PlayerRespawn>()
			.init_resource::<PlayerPhysicsEnabled>()
			.init_resource::<PlayerSpawnXz>()
			.add_message::<MovementAction>()
			.add_systems(Startup, spawn_player)
			.add_systems(
				Update,
				(
					keyboard_movement_input,
					update_grounded,
					apply_character_movement,
					follow_character_camera,
					queue_void_player_respawn,
					recover_void_player,
				)
					.chain()
					.in_set(PlayerControlSystems),
			);
	}
}

fn spawn_player(
	mut commands: Commands,
	mut meshes: ResMut<Assets<Mesh>>,
	mut materials: ResMut<Assets<StandardMaterial>>,
	layout: Res<TerrainCellLayout>,
	base: Res<WorldBaseTerrain>,
	locomotion: Res<CharacterLocomotion>,
	spawn_xz: Res<PlayerSpawnXz>,
) {
	// Startup: composed cells are not in the store yet. Hold above base noise
	// so the capsule / follow-cam are not born under jersey plateaus.
	let xz = spawn_xz.0.unwrap_or(layout.region_center_xz().xz());
	let spawn = player_spawn_point_at(xz, holding_elevation(&base.0, xz.x, xz.y));
	let player =
		spawn_player_body(&mut commands, &mut meshes, &mut materials, locomotion.as_ref(), spawn);
	commands.entity(player).insert(AwaitingTerrainSurface);
}

/// Spawn the terrain player's dynamic capsule at a known body-center position.
///
/// Call [`player_position_above_surface`] when the input is a terrain/POI surface point.
pub fn spawn_player_body(
	commands: &mut Commands,
	meshes: &mut Assets<Mesh>,
	materials: &mut Assets<StandardMaterial>,
	locomotion: &CharacterLocomotion,
	spawn: Vec3,
) -> Entity {
	let collider = Collider::capsule(CAPSULE_RADIUS, CAPSULE_LENGTH);
	let mut caster_shape = collider.clone();
	caster_shape.set_scale(Vec3::splat(0.99), 10);

	let player = commands
		.spawn((
			Name::new("Player"),
			Player,
			CharacterController,
			Transform::from_translation(spawn),
			Visibility::default(),
			RigidBody::Dynamic,
			collider,
			PhysicsInteractionLayer::animated_layers(),
			ShapeCaster::new(caster_shape, Vec3::ZERO, Quat::IDENTITY, Dir3::NEG_Y)
				.with_max_distance(GROUND_CAST_DISTANCE)
				.with_query_filter(SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)),
			LockedAxes::ROTATION_LOCKED,
			ActiveCollisionHooks::MODIFY_CONTACTS,
		))
		.insert((
			MovementAcceleration(MOVE_ACCEL),
			JumpImpulse(JUMP_IMPULSE),
			MaxSlopeAngle(locomotion.max_slope_angle),
			MoveWish::default(),
			WalkableGround::default(),
			Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
			Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
			GravityScale(0.0),
		))
		.id();
	commands.spawn((
		Name::new("PlayerCapsule"),
		PlayerCapsule,
		ChildOf(player),
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
	player
}

pub(crate) fn capsule_half_height() -> f32 {
	CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5
}

/// Lift a terrain/POI surface point to a safe player capsule center.
pub fn player_position_above_surface(surface: Vec3) -> Vec3 {
	surface + Vec3::Y * (capsule_half_height() + 0.5)
}

/// Elevation used before the cell underfoot is published.
pub fn holding_elevation(base: &BaseTerrainNoise, x: f32, z: f32) -> f32 {
	base.height_at(x, z) + base.height_scale * HOLD_ABOVE_BASE_FACTOR
}

/// Composed height at `xz`, or [`holding_elevation`] while the cell is unpublished or busy.
///
/// Spawn placement must not use [`DurhamSurface::height_or_fallback`]: raw base
/// noise sits under jersey plateaus. Hold above the base until snap can drop
/// onto the published cell.
pub fn surface_or_hold_at(surface: &DurhamSurface, xz: Vec2) -> f32 {
	surface
		.composed_height_at(xz)
		.unwrap_or_else(|| holding_elevation(surface.base(), xz.x, xz.y))
}

pub fn player_spawn_point_at(xz: Vec2, elevation: f32) -> Vec3 {
	player_position_above_surface(Vec3::new(xz.x, elevation, xz.y))
}

pub fn player_spawn_point(layout: &TerrainCellLayout, elevation: f32) -> Vec3 {
	player_spawn_point_at(layout.region_center_xz().xz(), elevation)
}

pub(crate) fn snap_player_to_composed_surface(
	mut commands: Commands,
	physics: Res<PlayerPhysicsEnabled>,
	surface: DurhamSurface,
	awaiting: Query<Entity, (With<Player>, With<AwaitingTerrainSurface>)>,
	mut players: Query<
		(Entity, &mut Transform, &mut LinearVelocity, &mut GravityScale, Option<&OffTerrainAnchor>),
		With<Player>,
	>,
	terrain_colliders: Query<&CascadeChunk, With<TerrainTrimeshCollider>>,
) {
	let Ok((entity, mut transform, mut velocity, mut gravity, anchor)) = players.single_mut()
	else {
		return;
	};

	if !physics.0 {
		gravity.0 = 0.0;
		**velocity = Vec3::ZERO;
		return;
	}

	if anchor.is_some() {
		gravity.0 = PLAY_GRAVITY_SCALE;
		commands.entity(entity).remove::<AwaitingTerrainSurface>();
		return;
	}

	let xz = transform.translation.xz();
	let elevation = match surface.height_at(xz) {
		Ok(Some(elevation)) => elevation,
		Ok(None) | Err(lod::hcsg::Busy) => {
			gravity.0 = 0.0;
			**velocity = Vec3::ZERO;
			return;
		}
	};

	let target = player_spawn_point_at(xz, elevation);
	if awaiting.single().is_ok() {
		transform.translation = target;
		**velocity = Vec3::ZERO;
	}

	if terrain_collider_covers_xz(target, terrain_colliders.iter()) {
		gravity.0 = PLAY_GRAVITY_SCALE;
		if let Ok(entity) = awaiting.single() {
			commands.entity(entity).remove::<AwaitingTerrainSurface>();
		}
	} else {
		// Lost or not-yet-streamed column: freeze in place. Do not re-insert
		// [`AwaitingTerrainSurface`] — that marker is first-drop only, and snap
		// would teleport a released body back onto the heightfield sample.
		gravity.0 = 0.0;
		**velocity = Vec3::ZERO;
	}
}

fn void_floor_y(base: &BaseTerrainNoise) -> f32 {
	-base.height_scale * 4.0 - VOID_FLOOR_MARGIN
}

fn queue_void_player_respawn(
	time: Res<Time>,
	physics: Res<PlayerPhysicsEnabled>,
	base: Res<WorldBaseTerrain>,
	mut respawn: ResMut<PlayerRespawn>,
	mut player: Query<
		(&Transform, &mut LinearVelocity, &mut GravityScale),
		(With<Player>, Without<AwaitingTerrainSurface>),
	>,
) {
	if !physics.0 || respawn.queued_at.is_some() {
		return;
	}
	let Ok((transform, mut velocity, mut gravity)) = player.single_mut() else {
		return;
	};
	if transform.translation.y >= void_floor_y(&base.0) {
		return;
	}
	respawn.queued_at = Some(time.elapsed_secs() + VOID_RESPAWN_DELAY_SECS);
	gravity.0 = 0.0;
	**velocity = Vec3::ZERO;
}

fn recover_void_player(
	time: Res<Time>,
	surface: DurhamSurface,
	mut respawn: ResMut<PlayerRespawn>,
	mut commands: Commands,
	mut player: Query<
		(Entity, &mut Transform, &mut LinearVelocity, Option<&OffTerrainAnchor>),
		With<Player>,
	>,
) {
	let Some(at) = respawn.queued_at else {
		return;
	};
	if time.elapsed_secs() < at {
		return;
	}
	let Ok((entity, mut transform, mut velocity, anchor)) = player.single_mut() else {
		respawn.queued_at = None;
		return;
	};
	if let Some(anchor) = anchor {
		transform.translation = anchor.translation;
		**velocity = Vec3::ZERO;
		respawn.queued_at = None;
		return;
	}
	let xz = transform.translation.xz();
	let elevation = match surface.height_at(xz) {
		Ok(Some(elevation)) => elevation,
		Ok(None) | Err(lod::hcsg::Busy) => holding_elevation(surface.base(), xz.x, xz.y),
	};
	transform.translation = player_spawn_point_at(xz, elevation);
	**velocity = Vec3::ZERO;
	respawn.queued_at = None;
	// First-drop hold so snap waits for composed height + a collider column.
	commands.entity(entity).insert(AwaitingTerrainSurface);
}

/// Reposition the player after terrain layout regeneration.
pub fn respawn_player_on_layout(
	layout: &TerrainCellLayout,
	elevation: f32,
	transform: &mut Transform,
	velocity: &mut LinearVelocity,
) {
	transform.translation = player_spawn_point(layout, elevation);
	**velocity = Vec3::ZERO;
}

fn keyboard_movement_input(
	mode: Res<PlaygroundMode>,
	text_focus: Res<TextEntryFocus>,
	pad_movement: Res<PadMovementEnabled>,
	pad: Res<VirtualPad>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut wishes: Query<&mut MoveWish, With<Player>>,
	mut writer: MessageWriter<MovementAction>,
) {
	if !pad_movement.0 {
		return;
	}
	if *mode != PlaygroundMode::Character || text_focus.0 {
		for mut wish in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		return;
	}

	let direction = pad.move_stick.clamp_length_max(1.0);

	let wish_dir = if direction != Vec2::ZERO {
		if let Ok(camera) = cameras.single() {
			let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
			let forward = yaw * -Vec3::Z;
			let right_dir = yaw * Vec3::X;
			(right_dir * direction.x + forward * direction.y).normalize_or_zero()
		} else {
			Vec3::ZERO
		}
	} else {
		Vec3::ZERO
	};
	for mut wish in &mut wishes {
		wish.0 = wish_dir;
	}

	if direction != Vec2::ZERO {
		writer.write(MovementAction::Move(direction));
	}
	if pad.just_pressed(PadButton::A) {
		writer.write(MovementAction::Jump);
	}
}

fn update_grounded(
	mode: Res<PlaygroundMode>,
	motor: Res<VegetationPlayerMotor>,
	mut commands: Commands,
	mut query: Query<
		(
			Entity,
			&ShapeHits,
			&LinearVelocity,
			Has<Grounded>,
			Option<&mut Jumping>,
			&mut WalkableGround,
		),
		With<CharacterController>,
	>,
) {
	if !motor.0 || *mode != PlaygroundMode::Character {
		return;
	}

	for (entity, hits, velocity, was_grounded, jumping, mut walkable) in &mut query {
		let support = support_ground_normal(hits);
		if let Some(normal) = support {
			walkable.normal = normal;
		}
		let mut is_grounded = support.is_some();
		if !is_grounded
			&& was_grounded
			&& jumping.is_none()
			&& velocity.y > -GROUND_SNAP_SPEED
			&& velocity.y < GROUND_SNAP_SPEED
		{
			is_grounded = true;
		}
		let landed = jumping.as_ref().is_some_and(|jump| jump.left_ground);
		if is_grounded {
			commands.entity(entity).insert(Grounded);
			if landed {
				commands.entity(entity).remove::<Jumping>();
			}
		} else {
			commands.entity(entity).remove::<Grounded>();
			if let Some(mut jump) = jumping {
				jump.left_ground = true;
			}
		}
	}
}

/// Most upright support under the capsule, including slopes too steep to climb.
fn support_ground_normal(hits: &ShapeHits) -> Option<Vec3> {
	let mut best: Option<(f32, Vec3)> = None;
	for hit in hits.iter() {
		let normal = hit.normal1.normalize_or_zero();
		if normal.length_squared() < 1e-8 || normal.y <= MIN_SUPPORT_NORMAL_Y {
			continue;
		}
		let angle = normal.angle_between(Vec3::Y).abs();
		if best.is_none_or(|(best_angle, _)| angle < best_angle) {
			best = Some((angle, normal));
		}
	}
	best.map(|(_, normal)| normal)
}

/// Unit surface drive. Only the uphill component is removed above the climb cap.
fn ground_drive(wish: Vec3, normal: Vec3, max_slope: f32) -> Vec3 {
	let wish = Vec3::new(wish.x, 0.0, wish.z).normalize_or_zero();
	let normal = normal.normalize_or_zero();
	if wish.length_squared() < 1e-8 || normal.length_squared() < 1e-8 {
		return Vec3::ZERO;
	}
	let mut drive = wish - normal * wish.dot(normal);
	if normal.angle_between(Vec3::Y).abs() > max_slope {
		let uphill = (Vec3::Y - normal * normal.y).normalize_or_zero();
		let uphill_amount = drive.dot(uphill);
		if uphill_amount > 0.0 {
			drive -= uphill * uphill_amount;
		}
	}
	if drive.length_squared() < 1e-8 {
		Vec3::ZERO
	} else {
		drive.normalize()
	}
}

fn move_toward(current: Vec3, target: Vec3, max_delta: f32) -> Vec3 {
	let delta = target - current;
	if delta.length_squared() <= max_delta * max_delta {
		target
	} else {
		current + delta.normalize_or_zero() * max_delta
	}
}

fn control_ground_velocity(
	velocity: &mut LinearVelocity,
	wish: Vec3,
	normal: Vec3,
	max_slope: f32,
	accel: f32,
	dt: f32,
) {
	let normal = normal.normalize_or_zero();
	let tangent = **velocity - normal * velocity.dot(normal);
	let target = ground_drive(wish, normal, max_slope) * MOVE_SPEED;
	let rate = if target.length_squared() > 1e-8 { accel } else { MOVE_BRAKE };
	**velocity = move_toward(tangent, target, rate * dt);
}

fn control_air_velocity(velocity: &mut LinearVelocity, wish: Vec3, accel: f32, dt: f32) {
	let wish = Vec3::new(wish.x, 0.0, wish.z).normalize_or_zero();
	if wish.length_squared() < 1e-8 {
		return;
	}
	let horizontal = Vec3::new(velocity.x, 0.0, velocity.z);
	let next = move_toward(horizontal, wish * MOVE_SPEED, accel * AIR_CONTROL * dt);
	velocity.x = next.x;
	velocity.z = next.z;
}

fn apply_character_movement(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	motor: Res<VegetationPlayerMotor>,
	time: Res<Time>,
	mut reader: MessageReader<MovementAction>,
	mut controllers: Query<
		(
			Entity,
			&MoveWish,
			&ShapeHits,
			&MaxSlopeAngle,
			&WalkableGround,
			&MovementAcceleration,
			&JumpImpulse,
			&mut LinearVelocity,
			Has<Grounded>,
			Option<&Jumping>,
			Has<Player>,
		),
		With<CharacterController>,
	>,
) {
	if !motor.0 || *mode != PlaygroundMode::Character {
		for _ in reader.read() {}
		return;
	}

	let dt = time.delta_secs();
	let mut jump_requested = false;
	for action in reader.read() {
		jump_requested |= matches!(action, MovementAction::Jump);
	}

	for (
		entity,
		wish,
		hits,
		max_slope,
		walkable,
		accel,
		jump,
		mut velocity,
		grounded,
		jumping,
		is_player,
	) in &mut controllers
	{
		if let Some(normal) = grounded_plane(hits, walkable, grounded) {
			if jumping.is_none() {
				control_ground_velocity(&mut velocity, wish.0, normal, max_slope.0, accel.0, dt);
			} else {
				control_air_velocity(&mut velocity, wish.0, accel.0, dt);
			}
		} else {
			control_air_velocity(&mut velocity, wish.0, accel.0, dt);
		}
		if jump_requested && grounded && is_player {
			velocity.y = jump.0;
			commands.entity(entity).insert(Jumping { left_ground: false });
		}
	}
}

fn grounded_plane(hits: &ShapeHits, walkable: &WalkableGround, grounded: bool) -> Option<Vec3> {
	grounded.then(|| support_ground_normal(hits).unwrap_or(walkable.normal))
}

fn follow_character_camera(
	mode: Res<PlaygroundMode>,
	follow: Res<CharacterCameraFollowEnabled>,
	players: Query<&Transform, (With<Player>, Without<Camera3d>)>,
	mut cameras: Query<(&mut Transform, &CameraController), With<Camera3d>>,
) {
	if !follow.0 {
		return;
	}
	if *mode != PlaygroundMode::Character {
		return;
	}
	let Ok(player) = players.single() else {
		return;
	};
	let Ok((mut camera_transform, controller)) = cameras.single_mut() else {
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, controller.yaw);
	let pitch = Quat::from_axis_angle(Vec3::X, controller.pitch);
	let rotation = yaw * pitch;
	let offset = rotation * Vec3::new(0.0, 0.0, CAMERA_DISTANCE) + Vec3::Y * CAMERA_HEIGHT;
	let target = player.translation + Vec3::Y * CAMERA_LOOK_HEIGHT;
	camera_transform.translation = target + offset;
	camera_transform.look_at(target, Vec3::Y);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use durham::TerrainConfig;
	use lod::hcsg::HcsgStorage;

	use super::*;

	#[test]
	fn holding_elevation_sits_above_base_noise() {
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		let hold = holding_elevation(&base, 0.0, 0.0);
		assert!(hold > base.height_at(0.0, 0.0) + 50.0);
	}

	#[test]
	fn surface_or_hold_at_holds_above_unpublished_cells() -> anyhow::Result<()> {
		let mut world = World::new();
		let layout = TerrainCellLayout::default();
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		world.init_resource::<HcsgStorage>();
		world.insert_resource(layout);
		world.insert_resource(WorldBaseTerrain(base.clone()));

		let mut state = bevy::ecs::system::SystemState::<DurhamSurface>::new(&mut world);
		let surface = state.get(&world)?;
		let xz = Vec2::new(80.0, 5.0);
		let hold = surface_or_hold_at(&surface, xz);
		assert_eq!(hold, holding_elevation(&base, xz.x, xz.y));
		assert!(hold > surface.fallback_height_at(xz) + 50.0);
		Ok(())
	}

	#[test]
	fn void_floor_sits_below_bedrock() {
		let base = BaseTerrainNoise::from_config(&TerrainConfig::new(42));
		assert_eq!(void_floor_y(&base), -base.height_scale * 4.0 - VOID_FLOOR_MARGIN);
		assert!(void_floor_y(&base) < -2_000.0);
	}

	#[test]
	fn canyon_depth_does_not_queue_void_recovery() -> anyhow::Result<()> {
		let (mut world, player) = void_world(Vec3::new(0.0, 0.0, 0.0));
		world
			.run_system_once(queue_void_player_respawn)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<PlayerRespawn>().queued_at.is_none());
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(
			world.get::<GravityScale>(player).map(|gravity| gravity.0),
			Some(PLAY_GRAVITY_SCALE)
		);
		Ok(())
	}

	#[test]
	fn below_void_floor_queues_recovery_without_reawait() -> anyhow::Result<()> {
		let (mut world, player) = void_world(Vec3::new(0.0, -2_600.0, 0.0));
		world
			.run_system_once(queue_void_player_respawn)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<PlayerRespawn>().queued_at.is_some());
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(world.get::<GravityScale>(player).map(|gravity| gravity.0), Some(0.0));
		assert_eq!(
			world.get::<LinearVelocity>(player).map(|velocity| velocity.0),
			Some(Vec3::ZERO)
		);
		Ok(())
	}

	#[test]
	fn snap_freezes_released_player_without_teleport() -> anyhow::Result<()> {
		let pose = Vec3::new(4.0, 12.0, -3.0);
		let (mut world, player) = void_world(pose);
		world
			.run_system_once(snap_player_to_composed_surface)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(
			world.get::<Transform>(player).map(|transform| transform.translation),
			Some(pose)
		);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(world.get::<GravityScale>(player).map(|gravity| gravity.0), Some(0.0));
		Ok(())
	}

	#[test]
	fn void_recovery_snaps_and_holds_for_first_drop() -> anyhow::Result<()> {
		let (mut world, player) = void_world(Vec3::new(12.0, -2_600.0, -8.0));
		world.resource_mut::<PlayerRespawn>().queued_at = Some(0.0);
		world
			.run_system_once(recover_void_player)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let base = &world.resource::<WorldBaseTerrain>().0;
		let expected =
			player_spawn_point_at(Vec2::new(12.0, -8.0), holding_elevation(base, 12.0, -8.0));
		assert_eq!(
			world.get::<Transform>(player).map(|transform| transform.translation),
			Some(expected)
		);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_some());
		assert!(world.resource::<PlayerRespawn>().queued_at.is_none());
		Ok(())
	}

	#[test]
	fn default_locomotion_keeps_legacy_slope() {
		assert!(
			(CharacterLocomotion::default().max_slope_angle - DEFAULT_MAX_SLOPE_ANGLE).abs() < 1e-6
		);
	}

	#[test]
	fn playground_motor_stays_on_by_default() {
		assert!(VegetationPlayerMotor::default().0);
	}

	#[test]
	fn drop_in_ignores_distant_trimesh_columns() {
		let spawn = Vec3::new(0.5, 80.0, 0.5);
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
	fn regular_slopes_allow_equal_uphill_and_downhill_drive() {
		let slope = 45.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let uphill = ground_drive(Vec3::X, normal, 70.0_f32.to_radians());
		let downhill = ground_drive(Vec3::NEG_X, normal, 70.0_f32.to_radians());
		assert!((uphill.length() - 1.0).abs() < 1e-5);
		assert!((downhill.length() - 1.0).abs() < 1e-5);
		assert!((uphill + downhill).length() < 1e-5);
		assert!(uphill.y > 0.0);
		assert!(downhill.y < 0.0);
	}

	#[test]
	fn cliff_gate_removes_only_uphill_drive() {
		let slope = 80.0_f32.to_radians();
		let normal = Vec3::new(-slope.sin(), slope.cos(), 0.0);
		let uphill = ground_drive(Vec3::X, normal, 70.0_f32.to_radians());
		let downhill = ground_drive(Vec3::NEG_X, normal, 70.0_f32.to_radians());
		let contour = ground_drive(Vec3::Z, normal, 70.0_f32.to_radians());
		assert!(uphill.length_squared() < 1e-8);
		assert!((downhill.length() - 1.0).abs() < 1e-5);
		assert_eq!(contour, Vec3::Z);
	}

	#[test]
	fn grounded_probe_miss_reuses_last_support_plane() {
		let hits = ShapeHits::default();
		let normal = Vec3::new(-0.4, 0.9, 0.0).normalize();
		let walkable = WalkableGround { normal };
		assert_eq!(grounded_plane(&hits, &walkable, true), Some(normal));
		assert_eq!(grounded_plane(&hits, &walkable, false), None);
	}

	fn void_world(translation: Vec3) -> (World, Entity) {
		let mut world = World::new();
		world.init_resource::<Time>();
		world.insert_resource(PlayerPhysicsEnabled::default());
		world.insert_resource(PlayerRespawn::default());
		world.insert_resource(TerrainCellLayout::default());
		world.init_resource::<HcsgStorage>();
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		let player = world
			.spawn((
				Player,
				Transform::from_translation(translation),
				LinearVelocity(Vec3::Y * -20.0),
				GravityScale(PLAY_GRAVITY_SCALE),
			))
			.id();
		(world, player)
	}
}
