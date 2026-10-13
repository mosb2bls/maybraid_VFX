//! Capsule player + third-person character mode for the terrain playground.
//!
//! Movement logic follows Avian's `dynamic_character_3d` example (dynamic body,
//! shape-cast grounded check, jump impulse), with walk direction relative to
//! the camera yaw.

use avian3d::prelude::*;
use bevy::ecs::query::Has;
use bevy::prelude::*;
use durham::TerrainCellLayout;
use game_commands::command::TextEntryFocus;
use lod_avian::PhysicsInteractionLayer;
use std::f32::consts::PI;

use crate::camera::CameraController;
use crate::WorldBaseTerrain;

pub(crate) const CAPSULE_RADIUS: f32 = 0.4;
pub(crate) const CAPSULE_LENGTH: f32 = 1.0;
const MOVE_ACCEL: f32 = 40.0;
const MOVE_DAMPING: f32 = 0.92;
const JUMP_IMPULSE: f32 = 8.0;
const MAX_SLOPE_ANGLE: f32 = PI * 0.45;
/// Third-person orbit for a ~2 m humanoid (capsule center is hip height).
const CAMERA_DISTANCE: f32 = 3.6;
const CAMERA_HEIGHT: f32 = 1.1;
const CAMERA_LOOK_HEIGHT: f32 = 0.65;
const GROUND_CAST_DISTANCE: f32 = 0.45;
const GROUND_SNAP_SPEED: f32 = 1.5;

/// Camera-relative WASD wish on XZ. Zero when no move input.
#[derive(Component, Default)]
pub(crate) struct MoveWish(pub Vec3);

#[derive(SystemSet, Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct PlayerControlSystems;

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

/// Debug capsule mesh parented to [`Player`] (hidden when a character visual is set).
#[derive(Component)]
pub struct PlayerCapsule;

#[derive(Component)]
struct CharacterController;

#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Grounded;

/// Space jump is in flight. Cleared on landing, not on shapecast misses.
#[derive(Component)]
#[component(storage = "SparseSet")]
pub(crate) struct Jumping {
	left_ground: bool,
}

#[derive(Component)]
struct MovementAcceleration(f32);

#[derive(Component)]
struct MovementDampingFactor(f32);

#[derive(Component)]
struct JumpImpulse(f32);

#[derive(Component)]
struct MaxSlopeAngle(f32);

#[derive(Message)]
enum MovementAction {
	Move(Vec2),
	Jump,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<PlaygroundMode>()
			.add_message::<MovementAction>()
			.add_systems(Startup, spawn_player)
			.add_systems(
				Update,
				(
					keyboard_movement_input,
					update_grounded,
					apply_character_movement,
					apply_movement_damping,
					follow_character_camera,
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
) {
	// Startup: only base noise exists yet; `generate_cells` repositions onto composed height.
	let center = layout.region_center_xz();
	let spawn = player_spawn_point(&layout, base.0.height_at(center.x, center.z));
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
		))
		.insert((
			MovementAcceleration(MOVE_ACCEL),
			MovementDampingFactor(MOVE_DAMPING),
			JumpImpulse(JUMP_IMPULSE),
			MaxSlopeAngle(MAX_SLOPE_ANGLE),
			MoveWish::default(),
			Friction::ZERO.with_combine_rule(CoefficientCombine::Min),
			Restitution::ZERO.with_combine_rule(CoefficientCombine::Min),
			GravityScale(1.25),
		))
		.id();
	commands.spawn((
		Name::new("PlayerCapsule"),
		PlayerCapsule,
		ChildOf(player),
		Mesh3d(meshes.add(Capsule3d::new(CAPSULE_RADIUS, CAPSULE_LENGTH))),
		MeshMaterial3d(materials.add(Color::srgb(0.85, 0.55, 0.35))),
	));
}

pub(crate) fn capsule_half_height() -> f32 {
	CAPSULE_RADIUS + CAPSULE_LENGTH * 0.5
}

pub fn player_spawn_point(layout: &TerrainCellLayout, elevation: f32) -> Vec3 {
	let center = layout.region_center_xz();
	Vec3::new(center.x, elevation + capsule_half_height() + 0.5, center.z)
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
	keyboard: Res<ButtonInput<KeyCode>>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut wishes: Query<&mut MoveWish, With<Player>>,
	mut writer: MessageWriter<MovementAction>,
) {
	if *mode != PlaygroundMode::Character || text_focus.0 {
		for mut wish in &mut wishes {
			wish.0 = Vec3::ZERO;
		}
		return;
	}

	let up = keyboard.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]);
	let down = keyboard.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]);
	let left = keyboard.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]);
	let right = keyboard.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]);

	let direction =
		Vec2::new(right as i8 as f32 - left as i8 as f32, up as i8 as f32 - down as i8 as f32)
			.clamp_length_max(1.0);

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
	if keyboard.just_pressed(KeyCode::Space) {
		writer.write(MovementAction::Jump);
	}
}

fn update_grounded(
	mode: Res<PlaygroundMode>,
	mut commands: Commands,
	mut query: Query<
		(
			Entity,
			&ShapeHits,
			&LinearVelocity,
			Option<&MaxSlopeAngle>,
			Has<Grounded>,
			Option<&mut Jumping>,
		),
		With<CharacterController>,
	>,
) {
	if *mode != PlaygroundMode::Character {
		return;
	}

	for (entity, hits, velocity, max_slope_angle, was_grounded, jumping) in &mut query {
		let mut is_grounded = hits.iter().any(|hit| {
			if let Some(angle) = max_slope_angle {
				(-hit.normal2).angle_between(Vec3::Y).abs() <= angle.0
			} else {
				true
			}
		});
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

fn apply_character_movement(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	time: Res<Time>,
	cameras: Query<&CameraController, With<Camera3d>>,
	mut reader: MessageReader<MovementAction>,
	mut controllers: Query<
		(Entity, &MovementAcceleration, &JumpImpulse, &mut LinearVelocity, Has<Grounded>),
		With<CharacterController>,
	>,
) {
	if *mode != PlaygroundMode::Character {
		for _ in reader.read() {}
		return;
	}

	let Ok(camera) = cameras.single() else {
		for _ in reader.read() {}
		return;
	};

	let yaw = Quat::from_axis_angle(Vec3::Y, camera.yaw);
	let forward = yaw * -Vec3::Z;
	let right = yaw * Vec3::X;
	let dt = time.delta_secs();

	for action in reader.read() {
		for (entity, accel, jump, mut velocity, grounded) in &mut controllers {
			match action {
				MovementAction::Move(direction) => {
					let wish = (right * direction.x + forward * direction.y).normalize_or_zero();
					velocity.x += wish.x * accel.0 * dt;
					velocity.z += wish.z * accel.0 * dt;
				}
				MovementAction::Jump => {
					if grounded {
						velocity.y = jump.0;
						commands.entity(entity).insert(Jumping { left_ground: false });
					}
				}
			}
		}
	}
}

fn apply_movement_damping(
	mode: Res<PlaygroundMode>,
	mut query: Query<(&MovementDampingFactor, &mut LinearVelocity), With<CharacterController>>,
) {
	if *mode != PlaygroundMode::Character {
		return;
	}
	for (damping, mut velocity) in &mut query {
		velocity.x *= damping.0;
		velocity.z *= damping.0;
	}
}

fn follow_character_camera(
	mode: Res<PlaygroundMode>,
	players: Query<&Transform, (With<Player>, Without<Camera3d>)>,
	mut cameras: Query<(&mut Transform, &CameraController), With<Camera3d>>,
) {
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
