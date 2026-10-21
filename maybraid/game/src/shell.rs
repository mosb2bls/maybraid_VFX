//! Cameras, clear color, preview look, and world pause — derived from [`GameFlow`].
//!
//! Home / Characters use the world `Camera3d` as [`IsDefaultUiCamera`] with
//! preview-only [`RenderLayers`], matching [PR #729](https://github.com/ramate-io/maybraid/pull/729).
//! Isolation is camera layers, not `Visibility` on LOD / trimesh hosts.
//!
//! Async load-in follows [`efa73ad`](https://github.com/ramate-io/maybraid/commit/efa73adf):
//! a `Camera2d` exists only during [`GameFlow::LoadingWorld`]. A persistent
//! second camera would steal UI. Terrain streaming stays off on menu shells.
//! Discovery streams the playable world.

use bevy::camera::visibility::RenderLayers;
use bevy::camera::ClearColorConfig;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use characters_playground::CameraController as PreviewCameraController;
use layer_stack::ActiveGenerationMode;
use maybraid_game_mode_discover::{streams_terrain, Discovery};
use maybraid_world::{
	Durham, InputRouter, InventoryEditCameraFollow, PlayerPhysicsEnabled, SessionCapabilities,
	TerrainStreaming, WorldGameplayEnabled, WorldMenuShell, WorldSceneryVisible, SKY_CLEAR,
};
use menu_components::MENU_CLEAR;
use menu_playground::{
	end_character_editor, CharacterEditorReturn, CharacterPreviewLight, CharacterPreviewRoot,
	CharacterScreen,
};
use menu_screens::{
	request_show_gallery, request_show_home, request_show_in_game, request_show_loading,
	MenuScreens,
};

/// World camera pose stashed while the pause character editor uses the preview eye.
#[derive(Resource, Clone, Copy, Debug)]
pub(crate) struct StashedWorldCamera {
	transform: Transform,
}

use crate::flow::{GameFlow, PlaySession, WorldPause};

/// Durham / vegetation sky wash while the 3D camera is live.
const PREVIEW_EYE: Vec3 = Vec3::new(0.0, 1.6, 3.5);
const PREVIEW_LOOK: Vec3 = Vec3::new(0.0, 1.0, 0.0);
const WORLD_RENDER_LAYER: usize = 0;
const PREVIEW_RENDER_LAYER: usize = 1;

#[derive(Component)]
pub(crate) struct LoadingBackdropCamera;

pub(crate) fn spawn_loading_backdrop(mut commands: Commands) {
	commands.spawn((
		Camera2d,
		LoadingBackdropCamera,
		IsDefaultUiCamera,
		Camera { order: 1, clear_color: ClearColorConfig::Custom(MENU_CLEAR), ..default() },
	));
}

pub(crate) fn despawn_loading_backdrop(
	mut commands: Commands,
	cameras: Query<Entity, With<LoadingBackdropCamera>>,
) {
	for entity in &cameras {
		commands.entity(entity).try_despawn();
	}
}

pub(crate) fn enter_home(mut commands: Commands) {
	request_show_home(&mut commands);
}

pub(crate) fn enter_characters(mut commands: Commands) {
	request_show_gallery(&mut commands);
}

pub(crate) fn enter_loading_world(mut commands: Commands, screens: MenuScreens) {
	screens.clear(&mut commands);
	request_show_loading(&mut commands);
}

pub(crate) fn enter_world(mut commands: Commands, screens: MenuScreens) {
	screens.clear(&mut commands);
}

pub(crate) fn enter_world_menu(mut commands: Commands) {
	request_show_in_game(&mut commands);
}

/// Leaving pause drops every pause screen and any show request still queued,
/// so nothing from the pause stack spawns over live play.
pub(crate) fn exit_world_menu(
	mut commands: Commands,
	screens: MenuScreens,
	editing: Option<Res<CharacterEditorReturn>>,
) {
	screens.clear(&mut commands);
	if editing.is_some() {
		end_character_editor(&mut commands);
	}
}

pub(crate) fn restore_stashed_world_camera(
	mut commands: Commands,
	stashed: Option<Res<StashedWorldCamera>>,
	mut cameras: Query<(Entity, &mut Transform), (With<Camera3d>, Without<LoadingBackdropCamera>)>,
) {
	let Some(stashed) = stashed else {
		return;
	};
	for (entity, mut transform) in &mut cameras {
		*transform = stashed.transform;
		commands.entity(entity).remove::<PreviewCameraController>();
	}
	commands.remove_resource::<StashedWorldCamera>();
}

/// World follow while the pause menu edits the live character; gallery preview otherwise.
pub(crate) fn apply_pause_character_look(
	mut commands: Commands,
	character: Query<(), With<CharacterScreen>>,
	return_to: Option<Res<CharacterEditorReturn>>,
	stashed: Option<Res<StashedWorldCamera>>,
	mut cameras: Query<
		(Entity, &mut Transform, Has<PreviewCameraController>),
		(With<Camera3d>, Without<LoadingBackdropCamera>),
	>,
) {
	let editing = !character.is_empty();
	let in_game_edit =
		editing && return_to.is_some_and(|return_to| return_to.uses_live_world_player());
	let preview_edit = editing && !in_game_edit;
	let layers = if preview_edit {
		RenderLayers::layer(PREVIEW_RENDER_LAYER)
	} else {
		RenderLayers::layer(WORLD_RENDER_LAYER)
	};
	if preview_edit && stashed.is_none() {
		if let Some((_, transform, _)) = cameras.iter().next() {
			commands.insert_resource(StashedWorldCamera { transform: *transform });
		}
	}
	for (entity, mut transform, has_preview) in &mut cameras {
		commands.entity(entity).insert(layers.clone());
		if preview_edit && !has_preview {
			*transform = Transform::from_translation(PREVIEW_EYE).looking_at(PREVIEW_LOOK, Vec3::Y);
			commands.entity(entity).insert(PreviewCameraController {
				speed: 6.0,
				sensitivity: 0.005,
				yaw: 0.0,
				pitch: 0.0,
			});
		} else if !preview_edit && has_preview {
			if let Some(stashed) = stashed.as_ref() {
				*transform = stashed.transform;
			}
			commands.entity(entity).remove::<PreviewCameraController>();
		}
	}
	if !preview_edit && stashed.is_some() {
		commands.remove_resource::<StashedWorldCamera>();
	}
}

pub(crate) fn terrain_streaming_for_shell(flow: GameFlow) -> bool {
	matches!(flow, GameFlow::LoadingWorld | GameFlow::World)
}

/// Requests a shell flow together with the [`Discovery`] [`ActiveGenerationMode`].
///
/// The mode uses `set_if_neq`: Bevy 0.19 runs `OnExit` and `OnEnter` when
/// `set` repeats the current state, which would tear down the live world on
/// Loading → World.
#[derive(SystemParam)]
pub(crate) struct ShellRoute<'w> {
	flow: ResMut<'w, NextState<GameFlow>>,
	mode: ResMut<'w, NextState<ActiveGenerationMode>>,
}

impl ShellRoute<'_> {
	pub(crate) fn enter(&mut self, flow: GameFlow) {
		self.flow.set(flow);
		NextState::set_if_neq(&mut self.mode, ActiveGenerationMode::of::<Discovery>());
	}
}

fn world_session_playing(flow: GameFlow, session: PlaySession, pause: Option<WorldPause>) -> bool {
	flow == GameFlow::World
		&& session == PlaySession::Discovery
		&& pause == Some(WorldPause::Playing)
}

fn pending_or_current_pause(
	pause: Option<&State<WorldPause>>,
	next: Option<&NextState<WorldPause>>,
) -> Option<WorldPause> {
	match next {
		Some(NextState::Pending(value)) => Some(*value),
		_ => pause.map(|state| *state.get()),
	}
}

fn discovery_streams(session: PlaySession, flow: GameFlow) -> bool {
	streams_terrain(session == PlaySession::Discovery, terrain_streaming_for_shell(flow))
}

/// Lifecycle install from a shell enter. Not a per-frame truth table.
/// Pause writes an owned suspension token; it does not uninstall play.
pub(crate) fn install_shell_capabilities(
	flow: GameFlow,
	session: PlaySession,
	pause: Option<WorldPause>,
	capabilities: &mut SessionCapabilities,
	router: &mut InputRouter,
) {
	match flow {
		GameFlow::Home | GameFlow::Characters => {
			capabilities.install_menu_shell();
			router.set_paused(false);
		}
		GameFlow::LoadingWorld => {
			capabilities.install_loading(discovery_streams(session, flow));
			router.set_paused(false);
		}
		GameFlow::World => {
			capabilities.install_play(discovery_streams(session, flow));
			router.set_paused(!world_session_playing(flow, session, pause));
		}
	}
}

pub(crate) fn apply_session_projections(
	capabilities: SessionCapabilities,
	router: &InputRouter,
	gameplay: &mut WorldGameplayEnabled,
	physics: &mut PlayerPhysicsEnabled,
	streaming: &mut TerrainStreaming<Durham>,
	menu_shell: &mut WorldMenuShell,
) {
	*gameplay = WorldGameplayEnabled::from_session(capabilities, router);
	physics.0 = capabilities.physics_installed();
	streaming.enabled = capabilities.streaming_enabled();
	menu_shell.0 = capabilities.menu_shell_active();
}

/// One writer for scenery and live-inventory camera follow.
pub(crate) fn sync_scenery_and_editor(
	capabilities: Res<SessionCapabilities>,
	character: Query<(), With<CharacterScreen>>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut scenery: ResMut<WorldSceneryVisible>,
	mut inventory_follow: ResMut<InventoryEditCameraFollow>,
) {
	let editing = !character.is_empty();
	let in_game_edit =
		editing && return_to.is_some_and(|return_to| return_to.uses_live_world_player());
	let preview_edit = editing && !in_game_edit;
	inventory_follow.0 = in_game_edit;
	scenery.0 = capabilities.scenery_active() && !preview_edit;
}

/// Honor a pending pause the same frame Start is taken. Does not rebuild
/// capabilities from shell state.
pub(crate) fn resolve_session_capabilities(
	flow: Res<State<GameFlow>>,
	pause: Option<Res<State<WorldPause>>>,
	next_pause: Option<Res<NextState<WorldPause>>>,
	session: Res<PlaySession>,
	capabilities: Res<SessionCapabilities>,
	mut router: ResMut<InputRouter>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	mut physics: ResMut<PlayerPhysicsEnabled>,
	mut streaming: ResMut<TerrainStreaming<Durham>>,
	mut menu_shell: ResMut<WorldMenuShell>,
) {
	let pause = pending_or_current_pause(pause.as_deref(), next_pause.as_deref());
	if *flow.get() == GameFlow::World && *session == PlaySession::Discovery {
		match pause {
			Some(WorldPause::Menu) => router.set_paused(true),
			Some(WorldPause::Playing) => router.set_paused(false),
			None => {}
		}
	}
	apply_session_projections(
		*capabilities,
		&router,
		&mut gameplay,
		&mut physics,
		&mut streaming,
		&mut menu_shell,
	);
}

pub(crate) fn apply_shell_look(
	mut commands: Commands,
	flow: Res<State<GameFlow>>,
	pause: Option<Res<State<WorldPause>>>,
	session: Res<PlaySession>,
	mut clear: ResMut<ClearColor>,
	mut world_cameras: Query<
		(Entity, &mut Camera),
		(With<Camera3d>, Without<LoadingBackdropCamera>),
	>,
	mut loading_cameras: Query<&mut Camera, (With<LoadingBackdropCamera>, Without<Camera3d>)>,
	mut capabilities: ResMut<SessionCapabilities>,
	mut router: ResMut<InputRouter>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	mut physics: ResMut<PlayerPhysicsEnabled>,
	mut streaming: ResMut<TerrainStreaming<Durham>>,
	mut menu_shell: ResMut<WorldMenuShell>,
) {
	let flow = *flow.get();
	let loading = flow == GameFlow::LoadingWorld;
	let menu = matches!(flow, GameFlow::Home | GameFlow::Characters | GameFlow::LoadingWorld);
	clear.0 = if menu { MENU_CLEAR } else { SKY_CLEAR };
	let layers = camera_render_layers(flow);
	for (entity, mut camera) in &mut world_cameras {
		camera.is_active = !loading;
		commands.entity(entity).insert(layers.clone());
		if loading {
			commands.entity(entity).remove::<IsDefaultUiCamera>();
		} else {
			commands.entity(entity).insert(IsDefaultUiCamera);
		}
	}
	for mut camera in &mut loading_cameras {
		camera.is_active = loading;
	}
	// Menus keep Durham off. Discovery streams the playable rings. Gameplay and
	// the world motor stay on while the session is playing. Pause writes an
	// owned suspension token; it does not uninstall motors.
	install_shell_capabilities(
		flow,
		*session,
		pause.as_deref().map(|pause| *pause.get()),
		&mut capabilities,
		&mut router,
	);
	apply_session_projections(
		*capabilities,
		&router,
		&mut gameplay,
		&mut physics,
		&mut streaming,
		&mut menu_shell,
	);
}

fn camera_render_layers(flow: GameFlow) -> RenderLayers {
	match flow {
		GameFlow::Home | GameFlow::Characters => RenderLayers::layer(PREVIEW_RENDER_LAYER),
		GameFlow::LoadingWorld | GameFlow::World => RenderLayers::layer(WORLD_RENDER_LAYER),
	}
}

/// Match the eye used by the standalone menu playground.
pub(crate) fn attach_preview_camera(
	mut commands: Commands,
	mut cameras: Query<(Entity, &mut Transform), With<Camera3d>>,
) {
	for (entity, mut transform) in &mut cameras {
		*transform = Transform::from_translation(PREVIEW_EYE).looking_at(PREVIEW_LOOK, Vec3::Y);
		commands.entity(entity).insert(PreviewCameraController {
			speed: 6.0,
			sensitivity: 0.005,
			yaw: 0.0,
			pitch: 0.0,
		});
	}
}

pub(crate) fn detach_preview_camera(
	mut commands: Commands,
	preview: Query<Entity, (With<Camera3d>, With<PreviewCameraController>)>,
) {
	for entity in &preview {
		commands.entity(entity).remove::<PreviewCameraController>();
	}
}

/// Keep character previews and their lights isolated from the streamed world.
pub(crate) fn stamp_preview_render_layers(
	preview_roots: Query<Entity, With<CharacterPreviewRoot>>,
	preview_lights: Query<Entity, With<CharacterPreviewLight>>,
	children: Query<&Children>,
	layered: Query<(), With<RenderLayers>>,
	mut commands: Commands,
) {
	let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
	for root in preview_roots.iter().chain(preview_lights.iter()) {
		for entity in std::iter::once(root).chain(children.iter_descendants(root)) {
			if !layered.contains(entity) {
				commands.entity(entity).try_insert(preview.clone());
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use bevy::camera::visibility::RenderLayers;
	use bevy::prelude::*;

	use super::{
		apply_shell_look, camera_render_layers, resolve_session_capabilities,
		sync_scenery_and_editor, terrain_streaming_for_shell, ShellRoute, PREVIEW_RENDER_LAYER,
		WORLD_RENDER_LAYER,
	};
	use crate::flow::{GameFlow, PlaySession, WorldPause};
	use bevy::ecs::system::RunSystemOnce;
	use layer_stack::ActiveGenerationMode;
	use maybraid_game_mode_discover::Discovery;
	use maybraid_world::{
		Durham, InputRouter, InventoryEditCameraFollow, PlayerPhysicsEnabled, SessionCapabilities,
		TerrainStreaming, WorldGameplayEnabled, WorldMenuShell, WorldSceneryVisible,
	};
	use menu_components::MENU_CLEAR;

	#[test]
	fn menu_camera_sees_preview_only() {
		let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
		let world = RenderLayers::layer(WORLD_RENDER_LAYER);
		for flow in [GameFlow::Home, GameFlow::Characters] {
			assert!(camera_render_layers(flow).intersects(&preview));
			assert!(!camera_render_layers(flow).intersects(&world));
		}
	}

	fn enter_loading(mut route: ShellRoute) {
		route.enter(GameFlow::LoadingWorld);
	}

	#[test]
	fn a_route_requests_the_flow_and_the_mode_together() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world.insert_resource(NextState::<ActiveGenerationMode>::Unchanged);
		world
			.run_system_once(enter_loading)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let flow = world.resource::<NextState<GameFlow>>();
		let mode = world.resource::<NextState<ActiveGenerationMode>>();
		let flow_ok = matches!(flow, NextState::Pending(GameFlow::LoadingWorld));
		let mode_ok = matches!(mode, NextState::PendingIfNeq(mode) if mode.is::<Discovery>());
		if !flow_ok || !mode_ok {
			return Err(anyhow::anyhow!("route requested flow {flow:?} mode {mode:?}"));
		}
		Ok(())
	}

	#[test]
	fn discovery_shell_streams_in_the_world_shell() -> anyhow::Result<()> {
		for flow in [GameFlow::LoadingWorld, GameFlow::World] {
			assert!(terrain_streaming_for_shell(flow));
			let mut world = World::new();
			world.insert_resource(State::new(flow));
			world.insert_resource(PlaySession::Discovery);
			world.insert_resource(ClearColor(MENU_CLEAR));
			world.insert_resource(WorldGameplayEnabled(true));
			world.insert_resource(SessionCapabilities::default());
			world.init_resource::<InputRouter>();
			world.insert_resource(PlayerPhysicsEnabled(false));
			world.insert_resource(TerrainStreaming::<Durham>::new(false));
			world.insert_resource(WorldSceneryVisible(false));
			world.insert_resource(WorldMenuShell(false));
			world.insert_resource(InventoryEditCameraFollow(false));
			world
				.run_system_once(apply_shell_look)
				.map_err(|error| anyhow::anyhow!("{error:?}"))?;
			world
				.run_system_once(sync_scenery_and_editor)
				.map_err(|error| anyhow::anyhow!("{error:?}"))?;
			anyhow::ensure!(world.resource::<TerrainStreaming<Durham>>().enabled);
			assert!(!world.resource::<WorldGameplayEnabled>().0);
			assert_eq!(world.resource::<WorldSceneryVisible>().0, flow == GameFlow::World);
			assert!(world.resource::<SessionCapabilities>().streaming_enabled());
			assert_eq!(
				world.resource::<SessionCapabilities>().physics_installed(),
				flow == GameFlow::World,
				"World installs motors even when pause is unset; Loading does not"
			);
		}
		assert!(terrain_streaming_for_shell(GameFlow::LoadingWorld));
		assert!(terrain_streaming_for_shell(GameFlow::World));
		assert!(!terrain_streaming_for_shell(GameFlow::Home));
		Ok(())
	}

	#[test]
	fn discovery_play_enables_the_world_motor() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(State::new(GameFlow::World));
		world.insert_resource(State::new(WorldPause::Playing));
		world.insert_resource(PlaySession::Discovery);
		world.insert_resource(ClearColor(MENU_CLEAR));
		world.insert_resource(WorldGameplayEnabled(false));
		world.insert_resource(SessionCapabilities::default());
		world.init_resource::<InputRouter>();
		world.insert_resource(PlayerPhysicsEnabled(false));
		world.insert_resource(TerrainStreaming::<Durham>::new(false));
		world.insert_resource(WorldSceneryVisible(false));
		world.insert_resource(WorldMenuShell(false));
		world.insert_resource(InventoryEditCameraFollow(false));
		world
			.run_system_once(apply_shell_look)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldGameplayEnabled>().0);
		assert!(world.resource::<PlayerPhysicsEnabled>().0);
		anyhow::ensure!(world.resource::<TerrainStreaming<Durham>>().enabled);
		assert!(world.resource::<WorldSceneryVisible>().0);
		assert!(world
			.resource::<SessionCapabilities>()
			.gameplay_enabled(world.resource::<InputRouter>()));
		assert!(world.resource::<SessionCapabilities>().physics_installed());
		Ok(())
	}

	#[test]
	fn loading_and_world_cameras_see_default_layer() {
		let preview = RenderLayers::layer(PREVIEW_RENDER_LAYER);
		let world = RenderLayers::layer(WORLD_RENDER_LAYER);
		for flow in [GameFlow::LoadingWorld, GameFlow::World] {
			assert!(camera_render_layers(flow).intersects(&world));
			assert!(!camera_render_layers(flow).intersects(&preview));
		}
	}

	#[test]
	fn in_game_character_edit_keeps_the_world_camera() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use characters_playground::CameraController as PreviewCameraController;
		use maybraid_world::{InventoryEditCameraFollow, WorldSceneryVisible};
		use menu_playground::{CharacterEditorReturn, CharacterScreen};

		use super::apply_pause_character_look;

		let mut world = World::new();
		let mut capabilities = SessionCapabilities::default();
		capabilities.install_play(true);
		world.insert_resource(capabilities);
		let mut router = InputRouter::default();
		router.set_paused(true);
		world.insert_resource(router);
		world.insert_resource(WorldSceneryVisible(true));
		world.insert_resource(InventoryEditCameraFollow(false));
		world.insert_resource(CharacterEditorReturn::InGame);
		world.spawn(CharacterScreen);
		let camera = world.spawn((Camera3d::default(), Transform::default())).id();
		world
			.run_system_once(apply_pause_character_look)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(world.resource::<WorldSceneryVisible>().0);
		assert!(world.resource::<InventoryEditCameraFollow>().0);
		assert!(world.get::<PreviewCameraController>(camera).is_none());
		let layers = world.get::<RenderLayers>(camera).ok_or_else(|| anyhow::anyhow!("layers"))?;
		assert!(layers.intersects(&RenderLayers::layer(WORLD_RENDER_LAYER)));
		assert!(!layers.intersects(&RenderLayers::layer(PREVIEW_RENDER_LAYER)));
		Ok(())
	}

	#[test]
	fn unpausing_drops_queued_pause_screens() -> anyhow::Result<()> {
		use maybraid_world::InventoryEditCameraFollow;
		use menu_playground::{CharacterEditorReturn, CharacterScreen};
		use menu_screens::{request_show_in_game, MenuScreen, MenuShowRequest};

		use super::exit_world_menu;

		let mut world = World::new();
		world.insert_resource(InventoryEditCameraFollow(true));
		world.insert_resource(CharacterEditorReturn::InGame);
		world.spawn((CharacterScreen, MenuScreen));
		world
			.run_system_once(|mut commands: Commands| request_show_in_game(&mut commands))
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(exit_world_menu)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		world.insert_resource(SessionCapabilities::default());
		world.init_resource::<InputRouter>();
		world.insert_resource(WorldSceneryVisible(false));
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		anyhow::ensure!(world.query::<&MenuScreen>().iter(&world).count() == 0);
		anyhow::ensure!(
			world.query::<&MenuShowRequest>().iter(&world).count() == 0,
			"a pause screen queued before unpause must not spawn over play"
		);
		anyhow::ensure!(!world.resource::<InventoryEditCameraFollow>().0);
		Ok(())
	}

	#[test]
	fn gallery_character_edit_still_uses_preview_layers() -> anyhow::Result<()> {
		use bevy::ecs::system::RunSystemOnce;
		use characters_playground::CameraController as PreviewCameraController;
		use maybraid_world::{InventoryEditCameraFollow, WorldSceneryVisible};
		use menu_playground::{CharacterEditorReturn, CharacterScreen};

		use super::apply_pause_character_look;

		let mut world = World::new();
		let mut capabilities = SessionCapabilities::default();
		capabilities.install_play(true);
		world.insert_resource(capabilities);
		world.init_resource::<InputRouter>();
		world.insert_resource(WorldSceneryVisible(true));
		world.insert_resource(InventoryEditCameraFollow(false));
		world.insert_resource(CharacterEditorReturn::Gallery);
		world.spawn(CharacterScreen);
		let camera = world.spawn((Camera3d::default(), Transform::default())).id();
		world
			.run_system_once(apply_pause_character_look)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert!(!world.resource::<WorldSceneryVisible>().0);
		assert!(!world.resource::<InventoryEditCameraFollow>().0);
		assert!(world.get::<PreviewCameraController>(camera).is_some());
		let layers = world.get::<RenderLayers>(camera).ok_or_else(|| anyhow::anyhow!("layers"))?;
		assert!(layers.intersects(&RenderLayers::layer(PREVIEW_RENDER_LAYER)));
		Ok(())
	}

	#[test]
	fn pending_pause_suspends_gameplay_the_same_frame() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(State::new(GameFlow::World));
		world.insert_resource(State::new(WorldPause::Playing));
		world.insert_resource(NextState::<WorldPause>::Pending(WorldPause::Menu));
		world.insert_resource(PlaySession::Discovery);
		let mut capabilities = SessionCapabilities::default();
		capabilities.install_play(true);
		world.insert_resource(capabilities);
		world.init_resource::<InputRouter>();
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(PlayerPhysicsEnabled(true));
		world.insert_resource(TerrainStreaming::<Durham>::new(true));
		world.insert_resource(WorldMenuShell(false));
		world
			.run_system_once(resolve_session_capabilities)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldGameplayEnabled>().0);
		assert!(world.resource::<PlayerPhysicsEnabled>().0);
		assert!(world.resource::<SessionCapabilities>().physics_installed());
		assert!(world.resource::<SessionCapabilities>().simulation_active());
		assert!(!world
			.resource::<SessionCapabilities>()
			.gameplay_enabled(world.resource::<InputRouter>()));
		assert!(world.resource::<SessionCapabilities>().gameplay().is_installed());
		Ok(())
	}

	#[test]
	fn loading_play_pause_inventory_resume_home() -> anyhow::Result<()> {
		use maybraid_world::InventoryEditCameraFollow;
		use menu_playground::{CharacterEditorReturn, CharacterScreen};

		let mut world = World::new();
		world.insert_resource(State::new(GameFlow::LoadingWorld));
		world.insert_resource(PlaySession::Discovery);
		world.insert_resource(SessionCapabilities::default());
		world.init_resource::<InputRouter>();
		world.insert_resource(ClearColor(MENU_CLEAR));
		world.insert_resource(WorldGameplayEnabled(true));
		world.insert_resource(PlayerPhysicsEnabled(false));
		world.insert_resource(TerrainStreaming::<Durham>::new(false));
		world.insert_resource(WorldSceneryVisible(false));
		world.insert_resource(WorldMenuShell(false));
		world.insert_resource(InventoryEditCameraFollow(false));
		world
			.run_system_once(apply_shell_look)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<TerrainStreaming<Durham>>().enabled);
		assert!(!world.resource::<WorldGameplayEnabled>().0);

		world.insert_resource(State::new(GameFlow::World));
		world.insert_resource(State::new(WorldPause::Playing));
		world
			.run_system_once(apply_shell_look)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldGameplayEnabled>().0);
		assert!(world.resource::<PlayerPhysicsEnabled>().0);

		world.insert_resource(NextState::<WorldPause>::Pending(WorldPause::Menu));
		world
			.run_system_once(resolve_session_capabilities)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldGameplayEnabled>().0);
		assert!(world.resource::<PlayerPhysicsEnabled>().0);

		world.insert_resource(CharacterEditorReturn::InGame);
		world.spawn(CharacterScreen);
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<InventoryEditCameraFollow>().0);
		assert!(world.resource::<WorldSceneryVisible>().0);

		world.insert_resource(NextState::<WorldPause>::Pending(WorldPause::Playing));
		world
			.run_system_once(resolve_session_capabilities)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world.resource::<WorldGameplayEnabled>().0);

		world.insert_resource(State::new(GameFlow::Home));
		world
			.run_system_once(apply_shell_look)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world
			.run_system_once(sync_scenery_and_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(!world.resource::<WorldGameplayEnabled>().0);
		assert!(!world.resource::<SessionCapabilities>().physics_installed());
		assert!(world.resource::<SessionCapabilities>().menu_shell_active());
		Ok(())
	}
}
