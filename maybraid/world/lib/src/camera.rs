//! World-specific extensions around the shared player follow camera.

use avian3d::prelude::{Collider, ShapeCastConfig, SpatialQuery, SpatialQueryFilter};
use bevy::core_pipeline::prepass::DepthPrepass;
use bevy::prelude::*;
use durham::WorldBaseTerrain;
use game_commands::command::TextEntryFocus;
use lod_avian::PhysicsInteractionLayer;
use maybraid_input::{PadButton, VirtualPad};
use player::{CameraFollow, Player};
use player_camera::{
	spawn_follow_camera, CameraController, CameraLookSuppressed, CameraPov, CameraPovLocked,
	FollowCamera, PlayerCameraSystems,
};
use world_player::{
	player::holding_elevation, Player as VegetationPlayer, PlayerSpawnXz, PlaygroundMode,
};

use crate::control::{InventoryEditCameraFollow, WorldGameplayEnabled};
use crate::map_view::{WorldMapSet, WorldMapView};

const CAMERA_COLLISION_RADIUS: f32 = 0.18;
const CAMERA_COLLISION_SKIN: f32 = 0.08;
const CAMERA_COLLISION_MIN: f32 = 0.12;
const WORLD_CAMERA_FAR: f32 = 8_000.0;
const WORLD_CAMERA_NEAR: f32 = 0.1;
const WORLD_FREE_CAMERA_SPEED: f32 = 40.0;

type PlayerFollowQuery<'w, 's> = Query<
	'w,
	's,
	(Entity, &'static Transform),
	(With<Player>, With<CameraFollow>, Without<Camera3d>),
>;
type CameraFollowQuery<'w, 's> = Query<
	'w,
	's,
	(&'static mut Transform, &'static CameraController, &'static FollowCamera),
	(With<Camera3d>, Without<Player>),
>;

#[derive(Component)]
struct WorldFreeCamera {
	speed: f32,
}

fn world_follow_camera() -> FollowCamera {
	FollowCamera { near: WORLD_CAMERA_NEAR, far: WORLD_CAMERA_FAR, ..default() }
}

pub(crate) fn spawn_world_camera(
	mut commands: Commands,
	spawn: Res<PlayerSpawnXz>,
	base: Option<Res<WorldBaseTerrain>>,
) {
	let follow = world_follow_camera();
	let camera = spawn_follow_camera(&mut commands);
	if let Some(xz) = spawn.0 {
		let y = base
			.as_ref()
			.map(|base| holding_elevation(&base.0, xz.x, xz.y))
			.unwrap_or(follow.height);
		let look = Vec3::new(xz.x, y + follow.look_height, xz.y);
		let eye = look + Vec3::new(-follow.distance, follow.height, 0.0);
		commands
			.entity(camera)
			.insert(Transform::from_translation(eye).looking_at(look, Vec3::Y));
	}
	commands.entity(camera).insert((
		follow,
		Projection::Perspective(PerspectiveProjection {
			fov: follow.third_person_fov,
			near: follow.near,
			far: follow.far,
			..default()
		}),
		DistanceFog {
			color: Color::srgba(0.55, 0.65, 0.72, 1.0),
			directional_light_color: Color::srgba(1.0, 0.92, 0.78, 0.35),
			directional_light_exponent: 24.0,
			falloff: FogFalloff::Linear { start: 700.0, end: 4500.0 },
		},
		Msaa::Off,
		DepthPrepass,
		WorldFreeCamera { speed: WORLD_FREE_CAMERA_SPEED },
	));
}

/// Enable shared gameplay follow only while the world owns gameplay input.
pub(crate) fn sync_camera_mode(
	mut commands: Commands,
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	inventory_edit: Option<Res<InventoryEditCameraFollow>>,
	players: Query<(Entity, Has<CameraFollow>), With<VegetationPlayer>>,
) {
	// Map is [`CameraPov::Map`] on the follow camera, not a detached eye.
	let follow = *mode == PlaygroundMode::Character
		&& (gameplay.0 || inventory_edit.is_some_and(|edit| edit.0));
	for (entity, following) in &players {
		if follow && !following {
			commands.entity(entity).insert(CameraFollow);
		} else if !follow && following {
			commands.entity(entity).remove::<CameraFollow>();
		}
	}
}

pub(crate) fn sync_inventory_edit_look(
	edit: Res<InventoryEditCameraFollow>,
	map: Option<Res<WorldMapView>>,
	mut locked: Option<ResMut<CameraPovLocked>>,
	mut suppressed: Option<ResMut<CameraLookSuppressed>>,
	mut cameras: Query<&mut CameraController, With<FollowCamera>>,
) {
	let map_open = map.is_some_and(|map| map.open);
	if let Some(locked) = locked.as_deref_mut() {
		locked.0 = edit.0 || map_open;
	}
	if let Some(suppressed) = suppressed.as_deref_mut() {
		suppressed.0 = map_open;
	}
	if !edit.0 || map_open {
		return;
	}
	for mut controller in &mut cameras {
		if controller.pov.is_map() {
			continue;
		}
		controller.pov = CameraPov::ThirdPerson;
		controller.focus = 0.0;
		controller.ads = 0.0;
	}
}

/// Preserve the playground's debug free-fly mode on the sole shared camera.
fn fly_world_camera(
	keyboard: Res<ButtonInput<KeyCode>>,
	pad: Res<VirtualPad>,
	time: Res<Time>,
	text_focus: Res<TextEntryFocus>,
	mode: Res<PlaygroundMode>,
	gameplay: Res<WorldGameplayEnabled>,
	mut cameras: Query<
		(&mut Transform, &CameraController, &FollowCamera, &WorldFreeCamera, &mut Projection),
		With<Camera3d>,
	>,
) {
	if *mode != PlaygroundMode::Free {
		return;
	}
	let Ok((mut transform, controller, follow, free, mut projection)) = cameras.single_mut() else {
		return;
	};
	transform.rotation =
		Quat::from_rotation_y(controller.yaw) * Quat::from_rotation_x(controller.pitch);
	if let Projection::Perspective(perspective) = projection.as_mut() {
		perspective.fov = follow.third_person_fov;
	}
	if text_focus.0 || !gameplay.0 {
		return;
	}

	let mut movement = Vec3::ZERO;
	movement += *transform.forward() * pad.move_stick.y;
	movement += *transform.right() * pad.move_stick.x;
	if pad.pressed(PadButton::A) {
		movement += Vec3::Y;
	}
	if keyboard.pressed(KeyCode::ShiftLeft) || keyboard.pressed(KeyCode::ShiftRight) {
		movement -= Vec3::Y;
	}
	if movement.length_squared() > 0.0 {
		transform.translation += movement.normalize() * free.speed * time.delta_secs();
	}
}

/// Pull only the third-person translation toward the player when Fixed geometry blocks it.
pub(crate) fn obstruct_world_camera(
	mode: Res<PlaygroundMode>,
	spatial: SpatialQuery,
	players: PlayerFollowQuery,
	mut cameras: CameraFollowQuery,
) {
	if *mode != PlaygroundMode::Character {
		return;
	}
	let Ok((player_entity, player)) = players.single() else {
		return;
	};
	let Ok((mut camera, controller, follow)) = cameras.single_mut() else {
		return;
	};
	if controller.pov != CameraPov::ThirdPerson {
		return;
	}

	let yaw = Quat::from_rotation_y(controller.yaw);
	let target =
		player.translation + Vec3::Y * follow.look_height + yaw * Vec3::X * follow.shoulder_offset;
	camera.translation =
		obstructed_camera_translation(&spatial, target, camera.translation, player_entity);
}

fn obstructed_camera_translation(
	spatial: &SpatialQuery,
	origin: Vec3,
	desired: Vec3,
	exclude: Entity,
) -> Vec3 {
	let delta = desired - origin;
	let Ok(direction) = Dir3::new(delta) else {
		return desired;
	};
	let distance = delta.length();
	let filter = SpatialQueryFilter::from_mask(PhysicsInteractionLayer::Fixed)
		.with_excluded_entities([exclude]);
	let shape = Collider::sphere(CAMERA_COLLISION_RADIUS);
	let config = ShapeCastConfig::from_max_distance(distance);
	let hit_distance = spatial
		.cast_shape(&shape, origin, Quat::IDENTITY, direction, &config, &filter)
		.map(|hit| hit.distance);
	let travel =
		camera_cast_travel(distance, hit_distance, CAMERA_COLLISION_SKIN, CAMERA_COLLISION_MIN);
	origin + *direction * travel
}

fn camera_cast_travel(
	desired_distance: f32,
	hit_distance: Option<f32>,
	skin: f32,
	min_distance: f32,
) -> f32 {
	match hit_distance {
		Some(distance) => (distance - skin).clamp(min_distance, desired_distance),
		None => desired_distance,
	}
}

pub(crate) fn configure(app: &mut App) {
	app.add_systems(Startup, spawn_world_camera)
		.add_systems(
			Update,
			(sync_camera_mode, sync_inventory_edit_look)
				.after(WorldMapSet::Toggle)
				.before(PlayerCameraSystems::Look),
		)
		.add_systems(
			Update,
			obstruct_world_camera
				.after(PlayerCameraSystems::Follow)
				.before(PlayerCameraSystems::Apply),
		)
		.add_systems(Update, fly_world_camera.after(PlayerCameraSystems::Apply));
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn world_camera_keeps_world_view_range() {
		let follow = world_follow_camera();
		assert_eq!(follow.near, WORLD_CAMERA_NEAR);
		assert_eq!(follow.far, WORLD_CAMERA_FAR);
	}

	#[test]
	fn world_camera_setup_spawns_one_shared_gameplay_camera() {
		let mut app = App::new();
		app.init_resource::<world_player::PlayerSpawnXz>();
		app.add_systems(Startup, spawn_world_camera);
		app.update();
		let mut cameras = app
			.world_mut()
			.query_filtered::<(&FollowCamera, &CameraController), With<Camera3d>>();
		assert_eq!(cameras.iter(app.world()).count(), 1);
	}

	#[test]
	fn miss_keeps_desired_distance() {
		assert_eq!(camera_cast_travel(3.6, None, 0.08, 0.12), 3.6);
	}

	#[test]
	fn hit_pulls_in_by_skin() {
		assert!((camera_cast_travel(3.6, Some(1.0), 0.08, 0.12) - 0.92).abs() < 1e-5);
	}

	#[test]
	fn near_hit_does_not_cross_the_target() {
		assert_eq!(camera_cast_travel(3.6, Some(0.05), 0.08, 0.12), 0.12);
	}

	#[test]
	fn inventory_edit_keeps_follow_while_paused() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use player::{CameraFollow, Player};
		use world_player::Player as VegetationPlayer;

		use crate::InventoryEditCameraFollow;

		let mut world = World::new();
		world.insert_resource(PlaygroundMode::Character);
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(InventoryEditCameraFollow(true));
		let player = world.spawn((VegetationPlayer, Player)).id();
		world
			.run_system_once(sync_camera_mode)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.get::<CameraFollow>(player).is_some());
		Ok(())
	}
}
