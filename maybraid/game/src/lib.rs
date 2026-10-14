//! Maybraid game executable: home shell over the world playground.

mod agent_observe;
mod flow;
mod load;
mod panic_log;
mod paths;
mod shell;

pub use flow::{GameFlow, HomeRoute, PauseMenuRoute, PlaySession, WorldPause};
pub use panic_log::{install_panic_log_hook, log_dir, panic_log_path, ENV_LOGS};
pub use paths::assets_root;

use crate::shell::{
	apply_pause_character_look, apply_shell_look, attach_preview_camera, despawn_loading_backdrop,
	detach_preview_camera, enter_characters, enter_home, enter_loading_world, enter_world,
	enter_world_menu, exit_world_menu, restore_stashed_world_camera, spawn_loading_backdrop,
	stamp_preview_render_layers, ShellRoute,
};
use bevy::prelude::*;
use bevy::render::renderer::RenderAdapterInfo;
use bevy::window::PrimaryWindow;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::{MenuNav, MenuNavPad, PadButton, VirtualPad};
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{
	host_memory_bytes, parse_device_tier_env, parse_visual_style_env, reset_first_spawn_offer,
	resume_discovery_from_saved_waypoints, AdapterKind, AgentDrive, Durham,
	InventoryEditCameraFollow, PlayerPhysicsEnabled, PlayerSpawnXz, ShadowQuality,
	TerrainStreaming, VisualStyle, VisualStyleAutoReason, VisualStyleSetting, VisualStyleSignals,
	WorldGameplayEnabled, WorldMobHudEnabled, WorldPlaySessionEnded, WorldPlayerLoadout,
	WorldPlugin, WorldSceneryVisible, WorldSurfaceReady, WorldSurfaceSet, DEVICE_TIER_ENV,
	VISUAL_STYLE_ENV,
};
use menu_components::{
	consume_screen_back, ActiveOverlayKey, MenuBackConsumed, ScreenBackPressed, ShortTextModal,
	TextMenuSystems, MENU_CLEAR,
};
use menu_playground::{
	ActiveCharacter, CharacterEditBaseline, CharacterEditorReturn, CharacterMenuState,
	CharacterPreviewPlugin, CharacterScreen, CharacterScreenPlugin, CharacterSessionPlugin,
	EditingCharacter, RequestEditCharacter,
};
use menu_screens::{
	apply_unsaved_visual_style_default, cancel_pending_create, request_show_gallery,
	request_show_home, request_show_in_game, request_show_in_game_settings, CreateCharacterPlugin,
	GalleryScreen, GameMode, HomeMenuChoice, HomeScreenPlugin, InGameMenuChoice, InGameScreen,
	InGameScreenPlugin, InGameSettings, InGameSettingsScreen, InGameSettingsSystems,
	InGameShadowQuality, InGameVisualStyle, LoadingScreenPlugin, LoadingScreenSystems, MenuScreen,
	MenuScreens, RenderScaleReadout, SpinRevealScreen, VisualStyleAutoPick, VisualStyleSaved,
};

pub struct GamePlugin;

impl Plugin for GamePlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(WorldPlugin::game())
			.insert_resource(WorldGameplayEnabled(false))
			.insert_resource(InventoryEditCameraFollow(false))
			.insert_resource(PlayerPhysicsEnabled(false))
			.insert_resource(TerrainStreaming::<Durham>::new(false))
			.insert_resource(WorldSceneryVisible(false))
			.insert_resource(ClearColor(MENU_CLEAR))
			.init_resource::<PlaySession>()
			.init_state::<GameFlow>()
			.add_sub_state::<WorldPause>()
			.add_plugins((
				maybraid_game_mode_reliquary::ReliquaryPlugin,
				HomeScreenPlugin,
				InGameScreenPlugin,
				LoadingScreenPlugin,
				CreateCharacterPlugin,
				CharacterSessionPlugin,
				CharacterScreenPlugin,
				CharacterPreviewPlugin,
				MenuControllerPlugin,
			))
			.add_systems(
				OnEnter(GameFlow::Home),
				(enter_home, clear_play_session, apply_shell_look, attach_preview_camera).chain(),
			)
			.add_systems(
				OnEnter(GameFlow::Characters),
				(enter_characters, apply_shell_look, attach_preview_camera),
			)
			.add_systems(OnExit(GameFlow::Characters), detach_preview_camera)
			.add_systems(
				OnEnter(GameFlow::LoadingWorld),
				(
					enter_loading_world,
					spawn_loading_backdrop,
					reset_surface_ready,
					apply_shell_look,
					detach_preview_camera,
					crate::load::arm_first_load,
					load_active_player_loadout.before(resume_discovery_from_saved_waypoints),
					reset_first_spawn_offer,
					resume_discovery_from_saved_waypoints,
				),
			)
			.add_systems(
				OnExit(GameFlow::LoadingWorld),
				(despawn_loading_backdrop, crate::load::disarm_first_load),
			)
			.add_systems(
				OnEnter(GameFlow::World),
				(load_active_player_loadout, enter_world, apply_shell_look, detach_preview_camera)
					.chain(),
			)
			.add_systems(OnEnter(WorldPause::Playing), apply_shell_look)
			.add_systems(OnEnter(WorldPause::Menu), (enter_world_menu, apply_shell_look))
			.add_systems(OnExit(WorldPause::Menu), (exit_world_menu, restore_stashed_world_camera))
			.add_systems(
				PostStartup,
				(
					(
						boot_shell,
						apply_shell_look,
						attach_preview_camera.run_if(not(starting_discovery_at_override)),
					)
						.chain(),
					sync_world_shadows,
					(
						apply_unsaved_visual_style_from_host,
						sync_world_render_scale,
						sync_render_scale_readout,
					)
						.chain(),
				),
			)
			.add_systems(PreUpdate, sync_inventory_edit_follow.run_if(in_state(WorldPause::Menu)));
		if app.world().contains_resource::<AgentDrive>() {
			app.init_resource::<crate::agent_observe::AgentShowRequestMemory>().add_systems(
				Update,
				(
					crate::agent_observe::publish_agent_menu_shell,
					crate::agent_observe::publish_agent_menu_focus,
				)
					.chain(),
			);
		}
		app.add_systems(
			Update,
			(
				stamp_preview_render_layers,
				crate::load::finish_world_loading
					.run_if(in_state(GameFlow::LoadingWorld))
					.after(WorldSurfaceSet)
					.before(LoadingScreenSystems::Apply),
				route_home_choice.run_if(in_state(GameFlow::Home)),
				home_settings_back
					.after(TextMenuSystems::Navigate)
					.run_if(in_state(GameFlow::Home)),
				route_in_game_choice.run_if(in_state(WorldPause::Menu)),
				apply_pause_character_look.run_if(in_state(WorldPause::Menu)),
				// Discard restores the baseline from a Navigate observer; write it
				// to the live player before the swap drops the editor return.
				sync_world_loadout_from_editor
					.after(TextMenuSystems::Navigate)
					.run_if(in_state(WorldPause::Menu)),
				persist_changed_player_inventory,
				sync_world_mob_hud,
				sync_world_shadows,
				(
					apply_unsaved_visual_style_from_host,
					sync_world_render_scale,
					sync_render_scale_readout,
				)
					.chain()
					.after(InGameSettingsSystems::ApplyChoice)
					.before(InGameSettingsSystems::Show),
				pause_menu_back
					.after(TextMenuSystems::Navigate)
					.run_if(in_state(WorldPause::Menu)),
				character_back
					.after(TextMenuSystems::Navigate)
					.run_if(in_state(GameFlow::Characters)),
				toggle_world_pause
					.after(CharacterControlSystems)
					.before(TextMenuSystems::InputLock)
					.run_if(in_state(GameFlow::World)),
			),
		);
	}
}

fn starting_discovery_at_override(spawn: Res<PlayerSpawnXz>) -> bool {
	spawn.0.is_some()
}

fn boot_shell(
	spawn: Res<PlayerSpawnXz>,
	mut route: ShellRoute,
	mut mode: ResMut<GameMode>,
	mut commands: Commands,
	screens: MenuScreens,
) {
	if spawn.0.is_some() {
		mode.label = String::from("Discovery");
		commands.insert_resource(PlaySession::Discovery);
		enter_loading_world(commands, screens);
		route.enter(GameFlow::LoadingWorld);
		return;
	}
	enter_home(commands);
}

fn reset_surface_ready(mut ready: ResMut<WorldSurfaceReady>) {
	ready.0 = false;
}

/// Leave / Home: drop the session.
fn clear_play_session(
	mut session: ResMut<PlaySession>,
	mut mode: ResMut<GameMode>,
	mut ready: ResMut<WorldSurfaceReady>,
	mut ended: MessageWriter<WorldPlaySessionEnded>,
) {
	ready.0 = false;
	*session = PlaySession::None;
	mode.label = String::from(PlaySession::Discovery.label());
	ended.write(WorldPlaySessionEnded);
}

fn load_active_player_loadout(
	mut commands: Commands,
	active: Option<Res<ActiveCharacter>>,
	save_root: Res<character_persist::SaveRoot>,
) {
	commands.remove_resource::<WorldPlayerLoadout>();
	let Some(active) = active else {
		warn!("entering world without an active character; using the default world loadout");
		return;
	};
	let loadout = match read_player_loadout(save_root.as_ref(), active.id) {
		Ok(loadout) => loadout,
		Err(error) => {
			warn!("failed to load active player {} for world: {error}", active.id.to_hex());
			return;
		}
	};
	commands.insert_resource(loadout);
}

fn read_player_loadout(
	save_root: &character_persist::SaveRoot,
	id: character_persist::CharacterId,
) -> Result<WorldPlayerLoadout, character_persist::PersistError> {
	let model = character_model_user::load(save_root, id)?;
	let inventory = character_inventory_user::load(save_root, id)?;
	Ok(WorldPlayerLoadout::new(id.to_hex(), model.appearance, inventory).with_name(model.name))
}

fn route_home_choice(
	mut choices: MessageReader<HomeMenuChoice>,
	mut route: ShellRoute,
	mut mode: ResMut<GameMode>,
	mut commands: Commands,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match HomeRoute::from_choice(choice) {
		HomeRoute::World { session } => {
			commands.insert_resource(session);
			mode.label = String::from(session.label());
			route.enter(GameFlow::LoadingWorld);
		}
		HomeRoute::Characters => route.enter(GameFlow::Characters),
		HomeRoute::Settings => request_show_in_game_settings(&mut commands),
		HomeRoute::Unimplemented => {}
	}
}

fn route_in_game_choice(
	mut choices: MessageReader<InGameMenuChoice>,
	mut route: ShellRoute,
	mut commands: Commands,
	mut edits: MessageWriter<RequestEditCharacter>,
	active: Option<Res<ActiveCharacter>>,
	loadout: Option<Res<WorldPlayerLoadout>>,
) {
	let Some(choice) = choices.read().last().copied() else {
		return;
	};
	match PauseMenuRoute::from_choice(choice) {
		PauseMenuRoute::Leave => route.enter(GameFlow::Home),
		PauseMenuRoute::Settings => request_show_in_game_settings(&mut commands),
		PauseMenuRoute::Character => {
			let Some(active) = active else {
				warn!("pause character: no active character");
				return;
			};
			if !plays_saved_character(loadout.as_deref(), active.as_ref()) {
				warn!("pause character: the world loadout is not the active character's");
				return;
			}
			edits.write(RequestEditCharacter {
				id: active.id,
				return_to: CharacterEditorReturn::InGame,
				inventory: loadout.map(|loadout| loadout.inventory.clone()),
			});
		}
		PauseMenuRoute::Stay => {}
	}
}

fn persist_changed_player_inventory(
	loadout: Option<Res<WorldPlayerLoadout>>,
	active: Option<Res<ActiveCharacter>>,
	save_root: Res<character_persist::SaveRoot>,
) {
	let Some(loadout) = loadout else {
		return;
	};
	if !loadout.is_changed() {
		return;
	}
	let Some(active) = active else {
		return;
	};
	if !plays_saved_character(Some(&loadout), &active) {
		return;
	}
	if let Err(error) = character_inventory_user::save(&save_root, active.id, &loadout.inventory) {
		warn!("failed to persist inventory {}: {error}", active.id.to_hex());
	}
}

/// A world loadout under another key must not reach the active character's files,
/// neither through its bag nor through the editor.
fn plays_saved_character(loadout: Option<&WorldPlayerLoadout>, active: &ActiveCharacter) -> bool {
	loadout.is_none_or(|loadout| loadout.key == active.id.to_hex())
}

fn sync_inventory_edit_follow(
	character: Query<(), With<CharacterScreen>>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut follow: ResMut<InventoryEditCameraFollow>,
) {
	follow.0 = !character.is_empty()
		&& return_to.is_some_and(|return_to| return_to.uses_live_world_player());
}

fn sync_world_loadout_from_editor(
	baseline: Option<Res<CharacterEditBaseline>>,
	editing: Option<Res<EditingCharacter>>,
	menu: Res<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut commands: Commands,
) {
	if return_to.is_none_or(|return_to| !return_to.uses_live_world_player()) {
		return;
	}
	let Some(editing) = editing else {
		return;
	};
	if !menu.is_changed() && baseline.as_ref().is_none_or(|baseline| !baseline.is_changed()) {
		return;
	}
	commands.insert_resource(
		WorldPlayerLoadout::new(
			editing.id.to_hex(),
			menu.0.appearance(),
			menu.0.inventory.clone().unwrap_or_default(),
		)
		.with_name(menu.0.saved_name()),
	);
}

fn sync_world_mob_hud(settings: Res<InGameSettings>, mut hud: ResMut<WorldMobHudEnabled>) {
	if hud.0 != settings.mob_hud {
		hud.0 = settings.mob_hud;
	}
}

fn sync_world_shadows(settings: Res<InGameSettings>, mut quality: ResMut<ShadowQuality>) {
	let wanted = match settings.shadows {
		InGameShadowQuality::High => ShadowQuality::High,
		InGameShadowQuality::Low => ShadowQuality::Low,
		InGameShadowQuality::Off => ShadowQuality::Off,
	};
	if *quality != wanted {
		*quality = wanted;
	}
}

fn apply_unsaved_visual_style_from_host(
	saved: Res<VisualStyleSaved>,
	mut auto: ResMut<VisualStyleAutoPick>,
	mut settings: ResMut<InGameSettings>,
	adapter: Option<Res<RenderAdapterInfo>>,
	windows: Query<&Window, With<PrimaryWindow>>,
) {
	if saved.0 || auto.reason.is_some() {
		return;
	}
	let Some(adapter) = adapter else {
		return;
	};
	let window = match windows.single() {
		Ok(window) => UVec2::new(window.physical_width(), window.physical_height()),
		Err(_) => UVec2::ZERO,
	};
	let signals = VisualStyleSignals {
		adapter_kind: AdapterKind::from_wgpu_device_type_name(&format!(
			"{:?}",
			adapter.device_type
		)),
		name: adapter.name.clone(),
		driver: adapter.driver.clone(),
		driver_info: adapter.driver_info.clone(),
		window,
		cpu_cores: std::thread::available_parallelism().ok().map(|n| n.get() as u32),
		memory_bytes: host_memory_bytes(),
		device_tier: parse_device_tier_env(std::env::var(DEVICE_TIER_ENV).ok().as_deref()),
	};
	let Some(pick) = apply_unsaved_visual_style_default(saved.0, &mut settings, &signals) else {
		return;
	};
	info!("visual style default {:?} ({})", pick.style, pick.reason.as_str());
	auto.reason = Some(pick.reason);
}

fn sync_world_render_scale(
	settings: Res<InGameSettings>,
	mut visual_style: ResMut<VisualStyleSetting>,
) {
	let env = parse_visual_style_env(std::env::var(VISUAL_STYLE_ENV).ok().as_deref());
	if env.is_some() || settings.is_changed() || visual_style.requested().is_none() {
		apply_settings_to_visual_style(&mut visual_style, settings.visual_style, env);
	}
}

fn apply_settings_to_visual_style(
	style: &mut VisualStyleSetting,
	preference: InGameVisualStyle,
	env: Option<VisualStyle>,
) {
	let wanted = env.unwrap_or_else(|| preference.to_visual_style());
	if style.requested() == Some(wanted) {
		return;
	}
	style.set(wanted);
}

fn sync_render_scale_readout(
	style: Res<VisualStyleSetting>,
	windows: Query<&Window, With<PrimaryWindow>>,
	auto: Res<VisualStyleAutoPick>,
	mut readout: ResMut<RenderScaleReadout>,
) {
	let env = parse_visual_style_env(std::env::var(VISUAL_STYLE_ENV).ok().as_deref());
	let (physical, factor) = match windows.single() {
		Ok(window) => {
			(UVec2::new(window.physical_width(), window.physical_height()), window.scale_factor())
		}
		Err(_) => (UVec2::ZERO, 1.0),
	};
	let next = visual_style_readout(&style, physical, factor, env, auto.reason);
	if *readout != next {
		*readout = next;
	}
}

fn visual_style_readout(
	style: &VisualStyleSetting,
	window_physical: UVec2,
	window_scale_factor: f32,
	env_override: Option<VisualStyle>,
	auto_reason: Option<VisualStyleAutoReason>,
) -> RenderScaleReadout {
	let plan = style.plan(window_physical, window_scale_factor);
	let effective_scale = if window_physical.y == 0 {
		1.0
	} else {
		plan.target_size.y as f32 / window_physical.y as f32
	};
	RenderScaleReadout {
		style: plan.style,
		effective_scale,
		window_physical,
		target_physical: plan.target_size,
		env_override,
		auto_reason,
	}
}

fn home_settings_back(
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	consumed: Res<MenuBackConsumed>,
	mut backs: MessageReader<ScreenBackPressed>,
	screens: Query<(), With<InGameSettingsScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	if !consume_screen_back(nav.as_ref(), &overlay, modal.is_open(), &consumed, &mut backs) {
		return;
	}
	request_show_home(&mut commands);
}

/// Settings Back returns to the pause root. The character editor's leave
/// prompt owns Back on that screen.
fn pause_menu_back(
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	consumed: Res<MenuBackConsumed>,
	mut backs: MessageReader<ScreenBackPressed>,
	settings: Query<(), With<InGameSettingsScreen>>,
	character: Query<(), With<CharacterScreen>>,
) {
	if !consume_screen_back(
		nav.as_ref(),
		&overlay,
		modal.is_open() || !character.is_empty(),
		&consumed,
		&mut backs,
	) {
		return;
	}
	if !settings.is_empty() {
		request_show_in_game(&mut commands);
	}
}

/// Where Start sends the pause state, if anywhere. Play opens pause and the
/// pause root closes it; on a pause sub-screen (character editor, settings)
/// Start is that screen's Select.
fn start_pause_target(pause: WorldPause, sub_screen: bool) -> Option<WorldPause> {
	match pause {
		WorldPause::Playing => Some(WorldPause::Menu),
		WorldPause::Menu if sub_screen => None,
		WorldPause::Menu => Some(WorldPause::Playing),
	}
}

/// Start is both a pause intent and a menu Select. When pause takes it, the
/// Select is dropped before menus navigate so one press never confirms a row
/// and unpauses in the same frame.
fn toggle_world_pause(
	pause: Res<State<WorldPause>>,
	mut next: ResMut<NextState<WorldPause>>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	mut intents: MessageReader<CharacterIntent>,
	mut nav: ResMut<MenuNavPad>,
	pad: Option<Res<VirtualPad>>,
	sub_screens: Query<(), (With<MenuScreen>, Without<InGameScreen>)>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::InGameMenu)) {
		return;
	}
	let Some(target) = start_pause_target(*pause.get(), !sub_screens.is_empty()) else {
		return;
	};
	if !pad.is_some_and(|pad| pad.just_pressed(PadButton::A)) {
		nav.events.retain(|event| *event != MenuNav::Select);
	}
	if target == WorldPause::Menu {
		gameplay.0 = false;
	}
	next.set(target);
}

/// Gallery / spin-reveal Back. The character editor's leave prompt owns Back
/// on that screen.
fn character_back(
	mut route: ShellRoute,
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	consumed: Res<MenuBackConsumed>,
	mut backs: MessageReader<ScreenBackPressed>,
	character: Query<(), With<CharacterScreen>>,
	spin: Query<(), With<SpinRevealScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
) {
	if !consume_screen_back(
		nav.as_ref(),
		&overlay,
		modal.is_open() || !character.is_empty(),
		&consumed,
		&mut backs,
	) {
		return;
	}
	if !spin.is_empty() {
		cancel_pending_create(&mut commands);
		request_show_gallery(&mut commands);
		return;
	}
	if !gallery.is_empty() {
		route.enter(GameFlow::Home);
	}
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryItem, ItemColor,
	};
	use character_model_user::CharacterModel;
	use character_persist::{CharacterId, SaveRoot};
	use characters::CharacterAppearance;
	use menu_playground::ActiveCharacter;

	use crate::{
		apply_settings_to_visual_style, assets_root, clear_play_session,
		load_active_player_loadout, persist_changed_player_inventory, read_player_loadout,
		route_home_choice, sync_world_loadout_from_editor, sync_world_shadows,
		visual_style_readout, GameFlow, PlaySession,
	};
	use maybraid_world::{
		ShadowQuality, VisualStyle, VisualStyleSetting, WorldPlaySessionEnded, WorldPlayerLoadout,
		WorldSurfaceReady,
	};
	use menu_playground::{
		CharacterEditBaseline, CharacterEditorReturn, CharacterMenuState, EditingCharacter,
	};
	use menu_screens::{
		GameMode, HomeMenuChoice, InGameSettings, InGameShadowQuality, InGameVisualStyle,
		RenderScaleReadout,
	};

	#[test]
	fn crate_assets_contain_barlow() {
		let font = assets_root().join("fonts/barlow/BarlowSemiCondensed-Regular.ttf");
		assert!(font.is_file(), "expected Barlow at {}", font.display());
	}

	#[test]
	fn world_loadout_uses_the_active_character() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(11);
		let model = CharacterModel::new(id, "Roamer", CharacterAppearance::default());
		let inventory = Inventory::default();
		character_model_user::save(&root, &model)?;
		character_inventory_user::save(&root, id, &inventory)?;

		let mut world = World::new();
		world.insert_resource(root);
		world.insert_resource(ActiveCharacter { id });
		world
			.run_system_once(load_active_player_loadout)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("world enter did not insert a loadout"))?;
		assert_eq!(loadout.key, id.to_hex());
		assert_eq!(loadout.inventory, inventory);
		Ok(())
	}

	#[test]
	fn the_removed_training_ground_route_stays_home() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<HomeMenuChoice>>();
		world.write_message(HomeMenuChoice::TrainingGround);
		world.insert_resource(NextState::<GameFlow>::Unchanged);
		world.insert_resource(NextState::<layer_stack::ActiveGenerationMode>::Unchanged);
		world.insert_resource(GameMode::default());
		world.insert_resource(PlaySession::None);
		world
			.run_system_once(route_home_choice)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::None);
		assert!(matches!(world.resource::<NextState<GameFlow>>(), NextState::Unchanged));
		Ok(())
	}

	#[test]
	fn leave_drops_the_session() -> anyhow::Result<()> {
		let mut world = World::new();
		world.init_resource::<Messages<WorldPlaySessionEnded>>();
		world.insert_resource(PlaySession::Discovery);
		world.insert_resource(GameMode::new("Elsewhere"));
		world.insert_resource(WorldSurfaceReady(true));
		world
			.run_system_once(clear_play_session)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<PlaySession>(), PlaySession::None);
		assert_eq!(world.resource::<GameMode>().label, "Discovery");
		assert!(!world.resource::<WorldSurfaceReady>().0);
		let ended = world.resource_mut::<Messages<WorldPlaySessionEnded>>().drain().count();
		assert_eq!(ended, 1, "leave should emit WorldPlaySessionEnded for Lubikan teardown");
		Ok(())
	}

	#[test]
	fn a_foreign_bag_is_never_saved_to_the_active_character() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		character_inventory_user::save(&root, id, &Inventory::default())?;

		let foreign = WorldPlayerLoadout::new(
			"foreign",
			CharacterAppearance::default(),
			Inventory::with_starter_outfit(vec![InventoryItem::firearm(FirearmMesh::Bullpup)]),
		);
		assert!(!foreign.inventory.items.is_empty());
		let mut world = World::new();
		world.insert_resource(root.clone());
		world.insert_resource(ActiveCharacter { id });
		world.insert_resource(foreign);
		world
			.run_system_once(persist_changed_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		assert_eq!(character_inventory_user::load(&root, id)?, Inventory::default());
		Ok(())
	}

	#[test]
	fn discovery_reads_the_active_character_files() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(42);
		let model = CharacterModel::new(id, "Active", CharacterAppearance::default());
		let inventory = Inventory::default();
		character_model_user::save(&root, &model)?;
		character_inventory_user::save(&root, id, &inventory)?;

		let loadout = read_player_loadout(&root, id)?;
		assert_eq!(loadout.key, id.to_hex());
		assert_eq!(loadout.name, "Active");
		assert_eq!(loadout.inventory, inventory);
		assert_eq!(loadout.appearance.species_id(), model.appearance.species_id());
		Ok(())
	}

	#[test]
	fn persist_writes_the_live_bag() -> anyhow::Result<()> {
		let dir = tempfile::tempdir()?;
		let root = SaveRoot::at(dir.path());
		let id = CharacterId(7);
		let model = CharacterModel::new(id, "Live", CharacterAppearance::default());
		character_model_user::save(&root, &model)?;
		character_inventory_user::save(&root, id, &Inventory::default())?;

		let bag = Inventory::with_starter_outfit(vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::firearm(FirearmMesh::Bullpup),
		]);
		let mut world = World::new();
		world.insert_resource(root.clone());
		world.insert_resource(ActiveCharacter { id });
		world.insert_resource(WorldPlayerLoadout::new(
			id.to_hex(),
			CharacterAppearance::default(),
			bag.clone(),
		));
		world
			.run_system_once(persist_changed_player_inventory)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let loaded = character_inventory_user::load(&root, id)?;
		assert_eq!(loaded.items.len(), bag.items.len());
		assert_eq!(loaded.clothing, bag.clothing);
		assert_eq!(loaded.weapons, bag.weapons);
		assert_eq!(loaded.skills, bag.skills);
		Ok(())
	}

	#[test]
	fn in_game_editor_writes_live_loadout() -> anyhow::Result<()> {
		let bag = Inventory::with_starter_outfit(vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::firearm(FirearmMesh::Bullpup),
		]);
		let mut menu = CharacterMenuState::default();
		menu.0.inventory = Some(bag.clone());
		let mut world = World::new();
		world.insert_resource(CharacterEditorReturn::InGame);
		world.insert_resource(EditingCharacter { id: CharacterId(11) });
		world.insert_resource(CharacterEditBaseline::capture(&menu.0));
		world.insert_resource(menu);
		world
			.run_system_once(sync_world_loadout_from_editor)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;

		let loadout = world
			.get_resource::<WorldPlayerLoadout>()
			.ok_or_else(|| anyhow::anyhow!("loadout"))?;
		assert_eq!(
			loadout.inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Bullpup)
		);
		Ok(())
	}

	#[test]
	fn pause_settings_copy_onto_the_sky_sun() -> anyhow::Result<()> {
		let mut world = World::new();
		world.insert_resource(InGameSettings {
			mob_hud: false,
			shadows: InGameShadowQuality::Low,
			visual_style: InGameVisualStyle::Impressionist,
		});
		world.insert_resource(ShadowQuality::High);
		world
			.run_system_once(sync_world_shadows)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(*world.resource::<ShadowQuality>(), ShadowQuality::Low);
		Ok(())
	}

	#[test]
	fn settings_visual_style_applies_until_env_wins() -> anyhow::Result<()> {
		let mut style = VisualStyleSetting::default();
		apply_settings_to_visual_style(&mut style, InGameVisualStyle::PrePostModern, None);
		assert_eq!(style.requested(), Some(VisualStyle::PrePostModern));
		apply_settings_to_visual_style(&mut style, InGameVisualStyle::Impressionist, None);
		assert_eq!(style.requested(), Some(VisualStyle::Impressionist));
		apply_settings_to_visual_style(
			&mut style,
			InGameVisualStyle::Retro,
			Some(VisualStyle::Native),
		);
		assert_eq!(style.requested(), Some(VisualStyle::Native));
		Ok(())
	}

	#[test]
	fn visual_style_readout_reports_resolution_and_env() -> anyhow::Result<()> {
		let mut style = VisualStyleSetting::default();
		style.set(VisualStyle::Impressionist);
		let readout = visual_style_readout(
			&style,
			UVec2::new(1920, 1080),
			1.0,
			Some(VisualStyle::Impressionist),
			None,
		);
		assert_eq!(
			readout,
			RenderScaleReadout {
				style: VisualStyle::Impressionist,
				effective_scale: 394.0 / 1080.0,
				window_physical: UVec2::new(1920, 1080),
				target_physical: UVec2::new(700, 394),
				env_override: Some(VisualStyle::Impressionist),
				auto_reason: None,
			}
		);
		Ok(())
	}

	#[test]
	fn unsaved_host_picks_sharp_on_low_unified_memory() -> anyhow::Result<()> {
		use maybraid_world::{AdapterKind, VisualStyleAutoReason, VisualStyleSignals};
		use menu_screens::apply_unsaved_visual_style_default;

		let mut settings = InGameSettings {
			mob_hud: false,
			shadows: InGameShadowQuality::Off,
			visual_style: InGameVisualStyle::Native,
		};
		let signals = VisualStyleSignals {
			adapter_kind: AdapterKind::IntegratedGpu,
			name: "Apple M1".into(),
			window: UVec2::new(2560, 1600),
			cpu_cores: Some(8),
			memory_bytes: Some(16 * 1024 * 1024 * 1024),
			..VisualStyleSignals::default()
		};
		let pick = apply_unsaved_visual_style_default(false, &mut settings, &signals)
			.expect("unsaved pick");
		assert_eq!(pick.style, VisualStyle::Sharp);
		assert_eq!(pick.reason, VisualStyleAutoReason::UnifiedMemoryBelow24GiB);
		assert_eq!(settings.visual_style, InGameVisualStyle::Sharp);

		settings.visual_style = InGameVisualStyle::Impressionist;
		assert!(apply_unsaved_visual_style_default(true, &mut settings, &signals).is_none());
		assert_eq!(settings.visual_style, InGameVisualStyle::Impressionist);
		Ok(())
	}

	#[test]
	fn start_closes_pause_only_from_the_pause_root() {
		use crate::{start_pause_target, WorldPause};

		assert_eq!(start_pause_target(WorldPause::Playing, false), Some(WorldPause::Menu));
		assert_eq!(start_pause_target(WorldPause::Playing, true), Some(WorldPause::Menu));
		assert_eq!(start_pause_target(WorldPause::Menu, false), Some(WorldPause::Playing));
		assert_eq!(start_pause_target(WorldPause::Menu, true), None);
	}

	fn start_pressed_world(pause: crate::WorldPause) -> World {
		use maybraid_character_controller::CharacterIntent;
		use maybraid_input::{MenuNav, MenuNavPad};
		use maybraid_world::WorldGameplayEnabled;

		let mut world = World::new();
		world.insert_resource(State::new(pause));
		world.insert_resource(NextState::<crate::WorldPause>::Unchanged);
		world.insert_resource(WorldGameplayEnabled(pause == crate::WorldPause::Playing));
		world.init_resource::<Messages<CharacterIntent>>();
		world.write_message(CharacterIntent::InGameMenu);
		let mut nav = MenuNavPad::default();
		nav.events.push(MenuNav::Select);
		world.insert_resource(nav);
		world
	}

	#[test]
	fn start_on_the_pause_root_unpauses_without_selecting_a_row() -> anyhow::Result<()> {
		use crate::{toggle_world_pause, WorldPause};
		use maybraid_input::{MenuNav, MenuNavPad};
		use menu_screens::{InGameScreen, MenuScreen};

		let mut world = start_pressed_world(WorldPause::Menu);
		world.spawn((InGameScreen, MenuScreen));
		world
			.run_system_once(toggle_world_pause)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(matches!(
			world.resource::<NextState<WorldPause>>(),
			NextState::Pending(WorldPause::Playing)
		));
		anyhow::ensure!(
			!world.resource::<MenuNavPad>().just_pressed(MenuNav::Select),
			"pause took Start; the pause row must not also activate"
		);
		Ok(())
	}

	#[test]
	fn start_in_the_character_editor_stays_with_the_editor() -> anyhow::Result<()> {
		use crate::{toggle_world_pause, WorldPause};
		use maybraid_input::{MenuNav, MenuNavPad};
		use menu_playground::CharacterScreen;
		use menu_screens::MenuScreen;

		let mut world = start_pressed_world(WorldPause::Menu);
		world.spawn((CharacterScreen, MenuScreen));
		world
			.run_system_once(toggle_world_pause)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		anyhow::ensure!(
			matches!(world.resource::<NextState<WorldPause>>(), NextState::Unchanged),
			"Start under a pause sub-screen must not unpause"
		);
		anyhow::ensure!(
			world.resource::<MenuNavPad>().just_pressed(MenuNav::Select),
			"the editor (and its leave prompt) should still see Start as Select"
		);
		Ok(())
	}

	#[test]
	fn pause_back_leaves_the_editor_to_its_leave_prompt() -> anyhow::Result<()> {
		use crate::pause_menu_back;
		use maybraid_input::{MenuNav, MenuNavPad};
		use menu_components::{
			ActiveOverlayKey, MenuBackConsumed, ScreenBackPressed, ShortTextModal,
		};
		use menu_playground::CharacterScreen;
		use menu_screens::{MenuScreen, MenuShowRequest};

		let mut world = World::new();
		world.init_resource::<Messages<ScreenBackPressed>>();
		let mut nav = MenuNavPad::default();
		nav.events.push(MenuNav::Back);
		world.insert_resource(nav);
		world.init_resource::<ActiveOverlayKey>();
		world.init_resource::<ShortTextModal>();
		world.init_resource::<MenuBackConsumed>();
		world.insert_resource(CharacterEditorReturn::InGame);
		world.spawn((CharacterScreen, MenuScreen));
		world
			.run_system_once(pause_menu_back)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		world.flush();
		let requests = world.query::<&MenuShowRequest>().iter(&world).count();
		anyhow::ensure!(requests == 0, "pause back must not route out of the editor");
		Ok(())
	}
}
