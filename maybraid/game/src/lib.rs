//! Maybraid game executable: home shell over the world playground.

mod flow;
mod load;
mod paths;
mod shell;

pub use flow::{GameFlow, HomeRoute, PauseMenuRoute, PlaySession, WorldPause};
pub use paths::assets_root;

use crate::shell::{
	apply_pause_character_look, apply_shell_look, attach_preview_camera, despawn_loading_backdrop,
	detach_preview_camera, enter_characters, enter_home, enter_loading_world, enter_world,
	enter_world_menu, exit_world_menu, restore_stashed_world_camera, spawn_loading_backdrop,
	stamp_preview_render_layers, ShellRoute,
};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use maybraid_character_controller::{CharacterControlSystems, CharacterIntent};
use maybraid_input::MenuNavPad;
use maybraid_menu_controller::MenuControllerPlugin;
use maybraid_world::{
	parse_visual_style_env, reset_first_spawn_offer, resume_discovery_from_saved_waypoints, Durham,
	InventoryEditCameraFollow, PlayerPhysicsEnabled, PlayerSpawnXz, ShadowQuality,
	TerrainStreaming, VisualStyle, VisualStyleSetting, WorldGameplayEnabled, WorldMobHudEnabled,
	WorldPlaySessionEnded, WorldPlayerLoadout, WorldPlugin, WorldSceneryVisible, WorldSurfaceReady,
	WorldSurfaceSet, VISUAL_STYLE_ENV,
};
use menu_components::{
	consume_screen_back, ActiveOverlayKey, MenuBackConsumed, ScreenBackPressed, ShortTextModal,
	TextMenuSystems, MENU_CLEAR,
};
use menu_playground::{
	ActiveCharacter, CharacterEditBaseline, CharacterEditorReturn, CharacterLeavePrompt,
	CharacterMenuState, CharacterModalMode, CharacterPreviewPlugin, CharacterScreen,
	CharacterScreenPlugin, CharacterSessionPlugin, EditingCharacter, RequestEditCharacter,
};
use menu_screens::{
	cancel_pending_create, request_show_gallery, request_show_home, request_show_in_game,
	request_show_in_game_settings, CreateCharacterPlugin, GalleryScreen, GameMode, HomeMenuChoice,
	HomeScreenPlugin, InGameMenuChoice, InGameScreenPlugin, InGameSettings, InGameSettingsScreen,
	InGameSettingsSystems, InGameShadowQuality, InGameVisualStyle, LoadingScreenPlugin,
	LoadingScreenSystems, MenuScreen, RenderScaleReadout, SpinRevealScreen,
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
					sync_world_render_scale,
					sync_render_scale_readout,
				),
			)
			.add_systems(PreUpdate, sync_inventory_edit_follow.run_if(in_state(WorldPause::Menu)))
			.add_systems(
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
					sync_world_loadout_from_editor.run_if(in_state(WorldPause::Menu)),
					persist_changed_player_inventory,
					sync_world_mob_hud,
					sync_world_shadows,
					(sync_world_render_scale, sync_render_scale_readout)
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
	screens: Query<Entity, With<MenuScreen>>,
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
	mut readout: ResMut<RenderScaleReadout>,
) {
	let env = parse_visual_style_env(std::env::var(VISUAL_STYLE_ENV).ok().as_deref());
	let (physical, factor) = match windows.single() {
		Ok(window) => {
			(UVec2::new(window.physical_width(), window.physical_height()), window.scale_factor())
		}
		Err(_) => (UVec2::ZERO, 1.0),
	};
	let next = visual_style_readout(&style, physical, factor, env);
	if *readout != next {
		*readout = next;
	}
}

fn visual_style_readout(
	style: &VisualStyleSetting,
	window_physical: UVec2,
	window_scale_factor: f32,
	env_override: Option<VisualStyle>,
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

fn pause_menu_back(
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	prompt: Res<CharacterLeavePrompt>,
	mode: Res<CharacterModalMode>,
	consumed: Res<MenuBackConsumed>,
	mut backs: MessageReader<ScreenBackPressed>,
	settings: Query<(), With<InGameSettingsScreen>>,
	character: Query<(), With<CharacterScreen>>,
) {
	if settings.is_empty() && character.is_empty() {
		return;
	}
	if !consume_screen_back(
		nav.as_ref(),
		&overlay,
		modal.is_open() || prompt.is_open() || (!character.is_empty() && mode.blocks_leave()),
		&consumed,
		&mut backs,
	) {
		return;
	}
	commands.remove_resource::<CharacterEditorReturn>();
	request_show_in_game(&mut commands);
}

fn toggle_world_pause(
	pause: Res<State<WorldPause>>,
	mut next: ResMut<NextState<WorldPause>>,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if !intents.read().any(|intent| matches!(intent, CharacterIntent::InGameMenu)) {
		return;
	}
	match pause.get() {
		WorldPause::Playing => {
			gameplay.0 = false;
			next.set(WorldPause::Menu);
		}
		WorldPause::Menu => next.set(WorldPause::Playing),
	}
}

fn character_back(
	mut route: ShellRoute,
	mut commands: Commands,
	nav: Res<MenuNavPad>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	prompt: Res<CharacterLeavePrompt>,
	mode: Res<CharacterModalMode>,
	consumed: Res<MenuBackConsumed>,
	mut backs: MessageReader<ScreenBackPressed>,
	character: Query<(), With<CharacterScreen>>,
	spin: Query<(), With<SpinRevealScreen>>,
	gallery: Query<(), With<GalleryScreen>>,
) {
	if !consume_screen_back(
		nav.as_ref(),
		&overlay,
		modal.is_open() || prompt.is_open() || (!character.is_empty() && mode.blocks_leave()),
		&consumed,
		&mut backs,
	) {
		return;
	}
	if !character.is_empty() {
		request_show_gallery(&mut commands);
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
			Some(VisualStyle::Sharp),
		);
		assert_eq!(style.requested(), Some(VisualStyle::Sharp));
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
		);
		assert_eq!(
			readout,
			RenderScaleReadout {
				style: VisualStyle::Impressionist,
				effective_scale: 720.0 / 1080.0,
				window_physical: UVec2::new(1920, 1080),
				target_physical: UVec2::new(1280, 720),
				env_override: Some(VisualStyle::Impressionist),
			}
		);
		Ok(())
	}
}
