//! Character / player host and diagnostics used by `maybraid-world`.
//!
//! The runnable playground binary is retired; see `maybraid/PLAYGROUNDS.md`.
//! Character / camera stay on [`VegetationHostPlugin`]. Mesh stats stay on
//! [`MeshStatsPlugin`]. Terrain fill is `Generate<Mode, OnTerrain<Durham>>`.

pub mod camera;
pub mod character;
pub mod commands;
pub mod diagnostics;
pub mod player;
pub mod policy;
pub mod seat;
mod ui;

pub use camera::CameraController;
pub use character::{
	CharacterSpecies, PlayerVisual, RequestSetCharacter, RequestSetCharacterAppearance,
};
pub use commands::{RequestMeshStats, RequestModeCharacter, RequestModeFree};
pub use diagnostics::{MeshStatsPlugin, PlaygroundDiag, PlaygroundTimingPlugin, RequestFpsToggle};
pub use durham::{TerrainCoverage, WorldBaseTerrain, WORLD_FINE_HALF_EXTENT_CELLS};
pub use game_commands::command::PendingStartupCommand;
pub use player::{
	holding_elevation, player_position_above_surface, player_spawn_point_at, spawn_player_body,
	surface_or_hold_at, AwaitingTerrainSurface, CharacterCameraFollowEnabled, CharacterLocomotion,
	Jumping, MoveWish, MovementAction, OffTerrainAnchor, PadMovementEnabled, Player, PlayerCapsule,
	PlayerControlSystems, PlayerPhysicsEnabled, PlayerPlugin, PlayerSpawnXz, PlaygroundMode,
	VegetationPlayerMotor,
};
pub use policy::{
	ModePlayerPolicies, ModePlayerPolicy, PlayerLifeEnded, PlayerLifeSet, RespawnOrigin,
};
pub use seat::PlayerSeat;

use avian3d::prelude::LinearVelocity;
use bevy::prelude::*;
use camera::{
	camera_controller, refocus_camera_on_elevation, release_modifiers_on_focus_change,
	setup_camera, surface_or_hold,
};
use character::{apply_set_character, drive_player_locomotion};
use characters::{CharacterHostsPlugin, CharacterMotionSystems};
use durham::DurhamSurface;
use game_commands::command::TextEntryFocus;
use game_commands::ui::GameCommandStatusText;
use maybraid_input::{PadGameplayEnabled, VirtualPadPlugin, VirtualPadSystems};
use player::{respawn_player_on_layout, snap_player_to_composed_surface};

/// Character, camera, snap, and locomotion without owning Durham fill.
pub struct VegetationHostPlugin {
	/// Spawn and drive the playground fly/follow camera.
	pub register_camera: bool,
}

impl Default for VegetationHostPlugin {
	fn default() -> Self {
		Self { register_camera: true }
	}
}

impl Plugin for VegetationHostPlugin {
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<VirtualPadPlugin>() {
			app.add_plugins(VirtualPadPlugin::default());
		}
		if !app.is_plugin_added::<PlayerPlugin>() {
			app.add_plugins(PlayerPlugin);
		}
		if !app.is_plugin_added::<CharacterHostsPlugin>() {
			app.add_plugins(CharacterHostsPlugin);
		}
		app.add_systems(PreUpdate, sync_pad_gameplay.before(VirtualPadSystems::Produce))
			.add_systems(
				Update,
				(
					apply_set_character,
					apply_mode_commands.after(apply_set_character),
					snap_player_to_composed_surface
						.after(apply_mode_commands)
						.before(PlayerControlSystems),
					drive_player_locomotion
						.after(PlayerControlSystems)
						.before(CharacterMotionSystems::Anim),
				),
			);
		if self.register_camera {
			app.add_systems(Startup, setup_camera).add_systems(
				Update,
				(release_modifiers_on_focus_change.before(camera_controller), camera_controller),
			);
		}
	}
}

fn apply_mode_commands(
	mut commands: Commands,
	mut mode: ResMut<PlaygroundMode>,
	mut status: Option<ResMut<GameCommandStatusText>>,
	surface: DurhamSurface,
	free: Query<Entity, With<RequestModeFree>>,
	character: Query<Entity, With<RequestModeCharacter>>,
	mut players: Query<(Entity, &mut Transform, &mut LinearVelocity), With<Player>>,
	mut cameras: Query<(&mut Transform, &mut CameraController), (With<Camera3d>, Without<Player>)>,
) {
	for entity in &free {
		*mode = PlaygroundMode::Free;
		ui::write_status(&mut status, "mode free");
		if let Ok((mut cam_t, mut controller)) = cameras.single_mut() {
			refocus_camera_on_elevation(
				surface.layout(),
				surface_or_hold(&surface),
				&mut cam_t,
				&mut controller,
			);
		}
		commands.entity(entity).despawn();
	}

	for entity in &character {
		// `/mode character` is a mode switch. Reset to the layout-center spawn only when
		// entering from free camera. Visual attach must not reuse this as a teleport.
		let reset_to_layout_spawn = *mode != PlaygroundMode::Character;
		*mode = PlaygroundMode::Character;
		ui::write_status(&mut status, "mode character — WASD move, mouse look, Space jump");
		if reset_to_layout_spawn {
			if let Ok((player, mut transform, mut velocity)) = players.single_mut() {
				let layout = surface.layout();
				if let Ok(Some(elevation)) = surface.height_at(layout.region_center_xz().xz()) {
					respawn_player_on_layout(layout, elevation, &mut transform, &mut velocity);
				}
				commands.entity(player).insert(AwaitingTerrainSurface);
			}
		}
		commands.entity(entity).despawn();
	}
}

fn sync_pad_gameplay(focus: Option<Res<TextEntryFocus>>, mut enabled: ResMut<PadGameplayEnabled>) {
	// Only the console focus lane. A name-modal sets `TextEntryBlocked` so `/`
	// stays off the field; the pad must stay up for the keypad.
	enabled.set_text(!focus.is_some_and(|focus| focus.0));
}

#[cfg(test)]
mod tests {
	use super::*;
	use avian3d::prelude::GravityScale;
	use bevy::ecs::system::RunSystemOnce;
	use durham::{BaseTerrainNoise, TerrainCellLayout, TerrainConfig};
	use lod::hcsg::HcsgStorage;
	use player::AwaitingTerrainSurface;

	#[test]
	fn appearance_attach_in_character_mode_does_not_await_layout_center() -> anyhow::Result<()> {
		let pose = Vec3::new(1_000.0, 14.0, -80.0);
		let (mut world, player) = mode_world(PlaygroundMode::Character, pose);

		world
			.run_system_once(emit_attach_mode_request)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(apply_mode_commands)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(snap_player_to_composed_surface)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(
			world.get::<Transform>(player).map(|transform| transform.translation),
			Some(pose)
		);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		assert_eq!(world.query::<&RequestModeCharacter>().iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn replacement_a_kilometre_from_center_stays_after_attach_update() {
		let pose = Vec3::new(1_000.0, 16.0, 0.0);
		let mut app = App::new();
		app.insert_resource(PlaygroundMode::Character)
			.insert_resource(PlayerPhysicsEnabled::default())
			.insert_resource(TerrainCellLayout::default())
			.init_resource::<HcsgStorage>()
			.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(&TerrainConfig::new(
				42,
			))))
			.add_systems(
				Update,
				(
					emit_attach_mode_request,
					apply_mode_commands.after(emit_attach_mode_request),
					snap_player_to_composed_surface.after(apply_mode_commands),
				),
			);
		let player = app
			.world_mut()
			.spawn((
				Player,
				Transform::from_translation(pose),
				LinearVelocity(Vec3::ZERO),
				GravityScale(0.0),
			))
			.id();
		app.update();

		assert_eq!(
			app.world().get::<Transform>(player).map(|transform| transform.translation),
			Some(pose)
		);
		assert!(app.world().get::<AwaitingTerrainSurface>(player).is_none());
	}

	#[test]
	fn mode_character_from_free_camera_resets_to_the_layout_spawn() -> anyhow::Result<()> {
		let pose = Vec3::new(640.0, 9.0, 120.0);
		let (mut world, player) = mode_world(PlaygroundMode::Free, pose);
		world.spawn(RequestModeCharacter);

		world
			.run_system_once(apply_mode_commands)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(*world.resource::<PlaygroundMode>(), PlaygroundMode::Character);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_some());
		assert_eq!(world.query::<&RequestModeCharacter>().iter(&world).count(), 0);
		Ok(())
	}

	#[test]
	fn mode_character_while_already_placed_leaves_the_body() -> anyhow::Result<()> {
		let pose = Vec3::new(1_200.0, 11.0, 40.0);
		let (mut world, player) = mode_world(PlaygroundMode::Character, pose);
		world.spawn(RequestModeCharacter);

		world
			.run_system_once(apply_mode_commands)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(
			world.get::<Transform>(player).map(|transform| transform.translation),
			Some(pose)
		);
		assert!(world.get::<AwaitingTerrainSurface>(player).is_none());
		Ok(())
	}

	fn emit_attach_mode_request(mut commands: Commands, mode: Res<PlaygroundMode>) {
		character::request_character_mode_if_needed(&mut commands, *mode);
	}

	fn mode_world(mode: PlaygroundMode, translation: Vec3) -> (World, Entity) {
		let mut world = World::new();
		world.insert_resource(mode);
		world.insert_resource(PlayerPhysicsEnabled::default());
		world.insert_resource(TerrainCellLayout::default());
		world.init_resource::<HcsgStorage>();
		world.insert_resource(WorldBaseTerrain(BaseTerrainNoise::from_config(
			&TerrainConfig::new(42),
		)));
		let player = world
			.spawn((
				Player,
				Transform::from_translation(translation),
				LinearVelocity(Vec3::ZERO),
				GravityScale(0.0),
			))
			.id();
		(world, player)
	}
}
