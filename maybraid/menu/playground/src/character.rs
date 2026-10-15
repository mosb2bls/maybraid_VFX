//! Right-justified character-creator panel over the playground camera.

use bevy::prelude::*;
use character_creation_menus::{
	inventory_paint_state, CharacterMenu, InventoryLayout, InventoryPaint, MenuEvent,
	INVENTORY_CAPTURE_TIMEOUT, INVENTORY_LABEL,
};
use character_items::{self_lubikan_catalog_key, Inventory};
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent};
use item_icons::{
	request_inventory_icons, request_self_lubikan_icon, CaptureClient, ItemCaptureSubjects,
	ItemIconCache, ItemIconDiscovery, ItemIconPlugin, ItemIconSystems,
};
use maybraid_character_ui_menu_renderer::{
	fill_inventory_layout, fill_inventory_loading, find_overlay_node, overlay_closes_on_pick,
	spawn_overlay_shell, CharacterHudSystems, CharacterMenuEvent,
	MaybraidCharacterMenuRendererPlugin, MaybraidMenuSink, MenuButton, MenuJustify, MenuSink,
	MenuThumbnailContext, OverlayClose, OverlayOpen, OverlaySelectRoot, OverlaySelectViewport,
	RenderContext,
};
use maybraid_menu_controller::MenuController;
use maybraid_skill_map::{SkillMapSpinRevealHud, SkillMapTileMaterialPlugin};
use menu_components::theme::{
	character_screen_grid_node, menu_region_wash, CORNER_BOTTOM, CORNER_INSET, MENU_LIST_PAD,
	MENU_LIST_RADIUS,
};
use menu_components::{
	spawn_corner_action, spawn_scroll_pane, ActiveOverlayKey, HudFonts, HudMenu, HudMenuItem,
	HudOverlayMenu, InputPromptDevice, MenuActivate, MenuFocus, MenuModalId, MenuModalLayer,
	MenuModalStack, ScreenBack, ShortTextChange, CATALOG_THUMB_PX, PANEL_ROW_GAP,
	PANEL_TILE_COLUMNS, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};
use menu_screens::{
	add_menu_show_systems, MenuScreen, MenuScreens, MenuShowRequest, SpinRevealItems,
};
use std::time::Duration;

use crate::drop_prompt::CharacterDropPromptPlugin;
use crate::leave_prompt::CharacterLeavePromptPlugin;

/// Queue a character-panel spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowCharacter;

/// Marker on the spawned character-panel root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CharacterScreen;

#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CharacterMenuState(pub CharacterMenu);

impl Default for CharacterMenuState {
	fn default() -> Self {
		Self(CharacterMenu::default())
	}
}

impl CharacterMenuState {
	pub fn for_create(items: Vec<character_items::InventoryItem>) -> Self {
		Self(CharacterMenu::for_create(items))
	}
}

/// Snapshot of a saved character when the editor opened (or last saved).
#[derive(Resource, Clone, Debug, PartialEq)]
pub struct CharacterEditBaseline {
	pub name: String,
	pub inventory: Inventory,
}

impl CharacterEditBaseline {
	pub fn capture(menu: &CharacterMenu) -> Self {
		Self { name: menu.saved_name(), inventory: menu.inventory.clone().unwrap_or_default() }
	}

	pub fn is_dirty(&self, menu: &CharacterMenu) -> bool {
		if menu.saved_name() != self.name {
			return true;
		}
		match menu.inventory.as_ref() {
			Some(inventory) => !self.inventory.same_catalog_identity(inventory),
			None => self.inventory != Inventory::default(),
		}
	}

	/// Put the editor back to the snapshot taken when it opened or last saved.
	pub fn restore_into(&self, menu: &mut CharacterMenu) {
		menu.name.clone_from(&self.name);
		menu.inventory = Some(self.inventory.clone());
	}
}

/// Create always offers Save Character. Saved characters offer Save Changes once dirty.
pub fn save_chrome(
	menu: &CharacterMenu,
	baseline: Option<&CharacterEditBaseline>,
) -> Option<&'static str> {
	if menu.is_create() {
		Some("Save Character")
	} else if baseline.is_some_and(|baseline| baseline.is_dirty(menu)) {
		Some("Save Changes")
	} else {
		None
	}
}

#[derive(Resource, Default)]
pub(crate) struct CharacterUiSyncState {
	pub(crate) menu_dirty: bool,
	save_label: Option<&'static str>,
	icon_generation: u64,
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
struct OverlaySelectState {
	open: Option<&'static str>,
}

#[derive(Resource, Default)]
pub(crate) struct OverlayUiSyncState {
	open: Option<&'static str>,
	pub(crate) menu_dirty: bool,
	icon_generation: u64,
	wait_started: Option<Duration>,
	showing_loading: bool,
}

#[derive(Component)]
struct CharacterPanelViewport;

/// Left grid cell. Overlay pickers spawn here so they cannot cover the HUD.
#[derive(Component)]
struct CharacterOverlaySlot;

#[derive(Component)]
struct CharacterSaveCorner;

#[derive(Component)]
struct CharacterBackCorner;

struct CatalogThumbs<'a> {
	icons: Option<&'a ItemIconCache>,
}

impl MenuThumbnailContext for CatalogThumbs<'_> {
	fn image_for_asset(
		&mut self,
		_label: &'static str,
		_asset_path: &'static str,
		_color: Color,
		_camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>> {
		None
	}

	fn image_for_key(&mut self, key: u64) -> Option<Handle<Image>> {
		let cache = self.icons?;
		if key == self_lubikan_catalog_key() {
			cache.silhouette_for_key(key)
		} else {
			cache.shaded_for_key(key)
		}
	}

	fn tint_for_key(&mut self, key: u64) -> Color {
		if key == self_lubikan_catalog_key() {
			TEXT_YELLOW
		} else {
			Color::WHITE
		}
	}

	fn viewport_for_key(&mut self, _key: u64) -> Option<Entity> {
		None
	}
}

pub fn request_show_character(commands: &mut Commands) {
	commands.spawn(RequestShowCharacter);
}

pub struct CharacterScreenPlugin;

impl Plugin for CharacterScreenPlugin {
	fn build(&self, app: &mut App) {
		app.add_plugins(MaybraidCharacterMenuRendererPlugin::<MenuEvent>::default());
		if !app.is_plugin_added::<SkillMapTileMaterialPlugin>() {
			app.add_plugins(SkillMapTileMaterialPlugin);
		}
		if !app.is_plugin_added::<CharacterLeavePromptPlugin>() {
			app.add_plugins(CharacterLeavePromptPlugin);
		}
		if !app.is_plugin_added::<CharacterDropPromptPlugin>() {
			app.add_plugins(CharacterDropPromptPlugin);
		}
		if !app.is_plugin_added::<ItemIconPlugin>() {
			app.add_plugins(ItemIconPlugin);
		}
		app.init_resource::<CharacterMenuState>()
			.init_resource::<CharacterUiSyncState>()
			.init_resource::<OverlaySelectState>()
			.init_resource::<OverlayUiSyncState>()
			.add_observer(on_overlay_open)
			.add_observer(on_overlay_close)
			.add_observer(on_menu_activate)
			.add_observer(on_menu_focus)
			.add_observer(on_short_text_change)
			.add_systems(Update, discover_menu_item_icons.in_set(ItemIconSystems::Discover))
			.add_systems(
				Update,
				(sync_character_ui, sync_overlay_select, sync_spin_reveal_item_icon)
					.chain()
					.in_set(CharacterHudSystems::Sync),
			);
		add_menu_show_systems(app, apply_show_character);
	}
}

fn apply_show_character(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	requests: Query<(), With<RequestShowCharacter>>,
	screens: MenuScreens,
	menu_state: Res<CharacterMenuState>,
	baseline: Option<Res<CharacterEditBaseline>>,
	mut sync_state: ResMut<CharacterUiSyncState>,
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	mut stack: ResMut<MenuModalStack>,
	icons: Option<Res<ItemIconCache>>,
) {
	if screens.take(&mut commands, &requests).is_none() {
		return;
	}
	overlay.open = None;
	overlay_sync.open = None;
	overlay_sync.menu_dirty = false;
	active_overlay.0 = None;
	stack.close(MenuModalId::OVERLAY_SELECT);
	sync_state.menu_dirty = false;
	sync_state.save_label = save_chrome(&menu_state.0, baseline.as_deref());
	sync_state.icon_generation = icons.as_ref().map(|cache| cache.generation).unwrap_or(0);
	let (viewport, save_corner, back_corner) = spawn_character_ui_shell(&mut commands);
	rebuild_character_ui_panel(
		&mut commands,
		&HudFonts::load(asset_server.as_ref()),
		menu_state.as_ref(),
		baseline.as_deref(),
		[viewport],
		Some(save_corner),
		Some(back_corner),
		&[],
		icons.as_deref(),
	);
}

fn sync_character_ui(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	menu_state: Res<CharacterMenuState>,
	baseline: Option<Res<CharacterEditBaseline>>,
	mut sync_state: ResMut<CharacterUiSyncState>,
	viewports: Query<(Entity, Option<&HudMenu>), With<CharacterPanelViewport>>,
	save_corners: Query<Entity, With<CharacterSaveCorner>>,
	back_corners: Query<Entity, With<CharacterBackCorner>>,
	icons: Option<Res<ItemIconCache>>,
) {
	if screens.is_empty() {
		return;
	}
	let save_label = save_chrome(&menu_state.0, baseline.as_deref());
	let icon_generation = icons.as_ref().map(|cache| cache.generation).unwrap_or(0);
	if !sync_state.menu_dirty
		&& !menu_state.is_changed()
		&& sync_state.save_label == save_label
		&& sync_state.icon_generation == icon_generation
	{
		return;
	}
	sync_state.menu_dirty = false;
	sync_state.save_label = save_label;
	sync_state.icon_generation = icon_generation;
	let previous: Vec<(Entity, Option<HudMenu>)> =
		viewports.iter().map(|(entity, menu)| (entity, menu.copied())).collect();
	let save_corner = save_corners.iter().next();
	let back_corner = back_corners.iter().next();
	rebuild_character_ui_panel(
		&mut commands,
		&HudFonts::load(asset_server.as_ref()),
		menu_state.as_ref(),
		baseline.as_deref(),
		previous.iter().map(|(entity, _)| *entity),
		save_corner,
		back_corner,
		&previous,
		icons.as_deref(),
	);
}

fn spawn_character_ui_shell(commands: &mut Commands) -> (Entity, Entity, Entity) {
	let mut viewport = Entity::PLACEHOLDER;
	let mut save_corner = Entity::PLACEHOLDER;
	let mut back_corner = Entity::PLACEHOLDER;
	commands
		.spawn((
			CharacterScreen,
			MenuScreen,
			MenuController::default(),
			Node { width: Val::Percent(100.0), height: Val::Percent(100.0), ..default() },
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((character_screen_grid_node(), Pickable::IGNORE))
				.with_children(|grid| {
					grid.spawn((
						CharacterOverlaySlot,
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							min_width: Val::Px(0.0),
							min_height: Val::Px(0.0),
							..default()
						},
						Pickable::IGNORE,
					));
					grid.spawn((
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							min_width: Val::Px(0.0),
							padding: UiRect::all(Val::Px(MENU_LIST_PAD)),
							flex_direction: FlexDirection::Column,
							overflow: Overflow::clip(),
							border_radius: BorderRadius::all(Val::Px(MENU_LIST_RADIUS)),
							..default()
						},
						BackgroundColor(menu_region_wash()),
						Pickable::IGNORE,
					))
					.with_children(|shell| {
						viewport = spawn_scroll_pane(
							shell,
							CharacterPanelViewport,
							AlignItems::FlexEnd,
							PANEL_ROW_GAP,
						);
					});
				});
			save_corner = root
				.spawn((
					CharacterSaveCorner,
					Node {
						position_type: PositionType::Absolute,
						right: Val::Px(CORNER_INSET),
						bottom: Val::Px(CORNER_BOTTOM),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::FlexEnd,
						..default()
					},
				))
				.id();
			back_corner = root
				.spawn((
					CharacterBackCorner,
					Node {
						position_type: PositionType::Absolute,
						left: Val::Px(CORNER_INSET),
						bottom: Val::Px(CORNER_BOTTOM),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::FlexStart,
						..default()
					},
				))
				.id();
		});
	(viewport, save_corner, back_corner)
}

fn rebuild_character_ui_panel(
	commands: &mut Commands,
	fonts: &HudFonts,
	menu_state: &CharacterMenuState,
	baseline: Option<&CharacterEditBaseline>,
	viewports: impl IntoIterator<Item = Entity>,
	save_corner: Option<Entity>,
	back_corner: Option<Entity>,
	previous: &[(Entity, Option<HudMenu>)],
	icons: Option<&ItemIconCache>,
) {
	let mut prewarm = Vec::new();
	for viewport in viewports {
		let keep = previous
			.iter()
			.find(|(entity, _)| *entity == viewport)
			.and_then(|(_, menu)| *menu);
		let Ok(mut entity) = commands.get_entity(viewport) else {
			continue;
		};
		entity.despawn_related::<Children>();
		let mut item_count = 0;
		entity.with_children(|panel| {
			item_count = populate_character_ui_panel(
				panel,
				fonts,
				menu_state,
				viewport,
				&mut prewarm,
				icons,
			);
		});
		drop(entity);
		if let Some(corner) = back_corner {
			if let Ok(mut corner) = commands.get_entity(corner) {
				corner.despawn_related::<Children>();
				corner.with_children(|corner| {
					spawn_corner_action(corner, fonts, "Back", ScreenBack);
				});
			}
		}
		if let Some(corner) = save_corner {
			if let Ok(mut corner) = commands.get_entity(corner) {
				corner.despawn_related::<Children>();
				if let Some(label) = save_chrome(&menu_state.0, baseline) {
					corner.with_children(|corner| {
						spawn_corner_action(
							corner,
							fonts,
							label,
							(
								MenuButton(MenuEvent::Save),
								HudMenuItem { index: item_count, menu: viewport },
							),
						);
					});
					item_count += 1;
				}
			}
		}
		if let Ok(mut entity) = commands.get_entity(viewport) {
			entity.insert(HudMenu::retain(item_count, keep));
		}
	}
}

fn populate_character_ui_panel(
	panel: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	menu_state: &CharacterMenuState,
	hud_menu: Entity,
	prewarm: &mut Vec<character_ui_menu::ThumbnailRequest>,
	icons: Option<&ItemIconCache>,
) -> usize {
	let mut thumbnails = CatalogThumbs { icons };
	let mut context = RenderContext {
		fonts,
		thumbnails: &mut thumbnails,
		asset_thumbnails: AssetThumbnailDisplay::None,
		prewarm,
		hud_menu,
		hud_item_count: 0,
		interactive: true,
		lock_appearance: menu_state.0.appearance_locked(),
		catalog_columns: PANEL_TILE_COLUMNS,
		catalog_thumb_px: CATALOG_THUMB_PX,
	};
	MaybraidMenuSink::new(MenuJustify::Right).render_nodes(
		&menu_state.0.menu_nodes(),
		panel,
		&mut context,
	);
	context.hud_item_count
}

fn sync_overlay_select(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	time: Res<Time>,
	slots: Query<Entity, With<CharacterOverlaySlot>>,
	overlays: Query<Entity, With<OverlaySelectRoot>>,
	viewports: Query<(Entity, Option<&HudMenu>), With<OverlaySelectViewport>>,
	menu_state: Res<CharacterMenuState>,
	overlay: Res<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	icons: Option<Res<ItemIconCache>>,
	prompt_device: Res<InputPromptDevice>,
) {
	let Ok(slot) = slots.single() else {
		for entity in &overlays {
			commands.entity(entity).try_despawn();
		}
		return;
	};
	let icon_generation = icons.as_ref().map(|cache| cache.generation).unwrap_or(0);
	let menu_changed = overlay_sync.menu_dirty
		|| menu_state.is_changed()
		|| overlay_sync.icon_generation != icon_generation;
	let open_changed = overlay_sync.open != overlay.open;
	let inventory_ready = inventory_overlay_ready(
		&menu_state.0,
		overlay.open,
		icons.as_deref(),
		overlay_sync.wait_started,
		time.elapsed(),
	);
	let loading_flip = overlay_sync.showing_loading && inventory_ready;
	if !open_changed && !menu_changed && !loading_flip {
		return;
	}
	overlay_sync.open = overlay.open;
	overlay_sync.menu_dirty = false;
	overlay_sync.icon_generation = icon_generation;

	let Some(key) = overlay.open else {
		for entity in &overlays {
			commands.entity(entity).try_despawn();
		}
		overlay_sync.wait_started = None;
		overlay_sync.showing_loading = false;
		return;
	};
	if open_changed && key == INVENTORY_LABEL {
		overlay_sync.wait_started = Some(time.elapsed());
	}

	let fonts = HudFonts::load(asset_server.as_ref());
	let nodes = menu_state.0.menu_nodes();
	let Some(node) = find_overlay_node(&nodes, key) else {
		for entity in &overlays {
			commands.entity(entity).try_despawn();
		}
		return;
	};

	if overlays.is_empty() || open_changed {
		for entity in &overlays {
			commands.entity(entity).try_despawn();
		}
		let mut viewport = Entity::PLACEHOLDER;
		let title_color =
			if menu_state.0.overlay_editable(key) { TEXT_YELLOW } else { TEXT_YELLOW_FAINT };
		let Ok(mut slot) = commands.get_entity(slot) else {
			return;
		};
		let prompts =
			(key == INVENTORY_LABEL).then_some((asset_server.as_ref(), *prompt_device));
		slot.with_children(|slot| {
			viewport = spawn_overlay_shell(slot, &fonts, key, title_color, prompts);
		});
		drop(slot);
		overlay_sync.showing_loading = !inventory_ready;
		populate_overlay_viewport(
			&mut commands,
			viewport,
			&fonts,
			node,
			None,
			menu_state.0.overlay_editable(key),
			icons.as_deref(),
			inventory_ready,
		);
		return;
	}

	for (viewport, menu) in &viewports {
		overlay_sync.showing_loading = !inventory_ready;
		populate_overlay_viewport(
			&mut commands,
			viewport,
			&fonts,
			node,
			menu.copied(),
			menu_state.0.overlay_editable(key),
			icons.as_deref(),
			inventory_ready,
		);
	}
}

fn inventory_overlay_ready(
	menu: &CharacterMenu,
	open: Option<&'static str>,
	icons: Option<&ItemIconCache>,
	wait_started: Option<Duration>,
	now: Duration,
) -> bool {
	if open != Some(INVENTORY_LABEL) {
		return true;
	}
	let Some(inventory) = menu.inventory.as_ref() else {
		return true;
	};
	let layout = InventoryLayout::character_menu(inventory);
	let reports = layout
		.captures
		.iter()
		.map(|need| icons.is_some_and(|cache| cache.filled_or_failed(need.key, need.silhouette)));
	let waited = wait_started
		.map(|started| now.saturating_sub(started))
		.unwrap_or(Duration::ZERO);
	if waited >= INVENTORY_CAPTURE_TIMEOUT
		&& !layout.captures.iter().all(|need| {
			icons.is_some_and(|cache| cache.filled_or_failed(need.key, need.silhouette))
		}) {
		warn!(
			waited_secs = waited.as_secs_f32(),
			"inventory icons timed out; showing the layout without every capture"
		);
	}
	inventory_paint_state(reports, waited, INVENTORY_CAPTURE_TIMEOUT) == InventoryPaint::Ready
}

fn populate_overlay_viewport<E: Copy + Send + Sync + 'static>(
	commands: &mut Commands,
	viewport: Entity,
	fonts: &HudFonts,
	node: &character_ui_menu::MenuNode<E>,
	previous: Option<HudMenu>,
	interactive: bool,
	icons: Option<&ItemIconCache>,
	ready: bool,
) {
	if ready {
		let mut thumbnails = CatalogThumbs { icons };
		fill_inventory_layout(
			commands,
			viewport,
			fonts,
			node,
			interactive,
			previous,
			&mut thumbnails,
		);
	} else {
		fill_inventory_loading(commands, viewport, fonts);
	}
	if let Ok(mut entity) = commands.get_entity(viewport) {
		entity.insert(HudOverlayMenu);
	}
}

fn discover_menu_item_icons(
	menu_state: Res<CharacterMenuState>,
	spin_items: Option<Res<SpinRevealItems>>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut client: CaptureClient,
) {
	request_self_lubikan_icon(&mut discovery, &mut client);
	if let Some(inventory) = menu_state.0.inventory.as_ref() {
		request_inventory_icons(inventory, &mut discovery, &mut subjects, &mut client);
	}
	if let Some(spin) = spin_items.as_deref() {
		let bag = Inventory { items: spin.0.clone(), ..Inventory::default() };
		request_inventory_icons(&bag, &mut discovery, &mut subjects, &mut client);
	}
}

/// The grant gallery paints every item on the screen. Hide the old single-item
/// catalog PNG so it cannot stack over the grid.
fn sync_spin_reveal_item_icon(mut hosts: Query<&mut Visibility, With<SkillMapSpinRevealHud>>) {
	for mut visibility in &mut hosts {
		*visibility = Visibility::Hidden;
	}
}

fn on_overlay_open(
	open: On<OverlayOpen>,
	mut overlay: ResMut<OverlaySelectState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	mut stack: ResMut<MenuModalStack>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	overlay.open = Some(open.event().key);
	active_overlay.0 = Some(open.event().key);
	stack.open(MenuModalLayer::OVERLAY_SELECT);
}

fn on_overlay_close(
	_close: On<OverlayClose>,
	mut overlay: ResMut<OverlaySelectState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	mut stack: ResMut<MenuModalStack>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	overlay.open = None;
	active_overlay.0 = None;
	stack.close(MenuModalId::OVERLAY_SELECT);
}

fn on_menu_activate(
	activate: On<MenuActivate<MenuEvent>>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	mut stack: ResMut<MenuModalStack>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	let event = activate.event().choice;
	if !menu_state.0.apply(event) {
		return;
	}
	ui_sync.menu_dirty = true;
	overlay_sync.menu_dirty = true;
	menu_events.write(CharacterMenuEvent::Menu(event));
	if let Some(key) = overlay.open {
		let nodes = menu_state.0.menu_nodes();
		if find_overlay_node(&nodes, key).is_some_and(overlay_closes_on_pick) {
			overlay.open = None;
			active_overlay.0 = None;
			stack.close(MenuModalId::OVERLAY_SELECT);
		}
	}
}

fn on_menu_focus(
	focus: On<MenuFocus<MenuEvent>>,
	menu_state: Res<CharacterMenuState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	if let Some(camera) = menu_state.0.camera_focus_for_event(focus.event().choice) {
		menu_events.write(CharacterMenuEvent::CameraFocus(camera));
	}
}

fn on_short_text_change(
	change: On<ShortTextChange>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	baseline: Option<Res<CharacterEditBaseline>>,
	screens: Query<Entity, With<CharacterScreen>>,
) {
	if screens.is_empty() {
		return;
	}
	if change.event().key != "Name" {
		return;
	}
	let previous = save_chrome(&menu_state.0, baseline.as_deref());
	menu_state.bypass_change_detection().0.name = change.event().value.clone();
	if save_chrome(&menu_state.0, baseline.as_deref()) != previous {
		ui_sync.menu_dirty = true;
	}
}

#[cfg(test)]
mod tests {
	use super::{save_chrome, CharacterEditBaseline};
	use character_creation_menus::CharacterMenu;
	use character_items::Inventory;

	#[test]
	fn save_chrome_create_and_dirty_saved() {
		let create = CharacterMenu::for_create(Vec::new());
		assert_eq!(save_chrome(&create, None), Some("Save Character"));

		let saved = CharacterMenu::for_saved(
			String::from("Misty"),
			&create.appearance(),
			Inventory::default(),
		);
		let baseline = CharacterEditBaseline::capture(&saved);
		assert_eq!(save_chrome(&saved, Some(&baseline)), None);

		let mut dirty = saved.clone();
		dirty.name = String::from("Mist");
		assert_eq!(save_chrome(&dirty, Some(&baseline)), Some("Save Changes"));
		assert!(baseline.is_dirty(&dirty));
		assert!(!baseline.is_dirty(&saved));

		baseline.restore_into(&mut dirty);
		assert_eq!(dirty.name, baseline.name);
		assert!(!baseline.is_dirty(&dirty));
	}

	#[test]
	fn catalog_thumbs_use_shaded_item_icons_when_ready() -> anyhow::Result<()> {
		use super::CatalogThumbs;
		use character_items::{FirearmMesh, InventoryItem};
		use item_icons::{CaptureId, ItemIconCache};
		use maybraid_character_ui_menu_renderer::MenuThumbnailContext;

		let item = InventoryItem::firearm(FirearmMesh::Bullpup);
		let key = item.icon_catalog_key().ok_or_else(|| anyhow::anyhow!("firearm catalog key"))?;
		let label = item.capture_id_label().ok_or_else(|| anyhow::anyhow!("firearm capture id"))?;
		let mut cache = ItemIconCache::default();
		cache.insert_shaded(CaptureId::new(label), bevy::prelude::Handle::default());
		let mut ready = CatalogThumbs { icons: Some(&cache) };
		if ready.image_for_key(key).is_none() {
			anyhow::bail!("ready catalog should show the shaded icon");
		}
		let mut pending = CatalogThumbs { icons: None };
		if pending.image_for_key(key).is_some() {
			anyhow::bail!("pending catalog should keep the placeholder");
		}
		Ok(())
	}

	#[test]
	fn catalog_thumbs_use_the_self_lubikan_silhouette() -> anyhow::Result<()> {
		use super::CatalogThumbs;
		use character_items::{self_lubikan_catalog_key, SELF_LUBIKAN_CAPTURE_LABEL};
		use item_icons::{CaptureId, ItemIconCache};
		use maybraid_character_ui_menu_renderer::MenuThumbnailContext;
		use menu_components::theme::TEXT_YELLOW;

		let key = self_lubikan_catalog_key();
		let mut cache = ItemIconCache::default();
		cache.insert_silhouette(
			CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL),
			bevy::prelude::Handle::default(),
		);
		let mut ready = CatalogThumbs { icons: Some(&cache) };
		if ready.image_for_key(key).is_none() {
			anyhow::bail!("Self Lubikan tile should show the silhouette");
		}
		if ready.tint_for_key(key) != TEXT_YELLOW {
			anyhow::bail!("Self Lubikan silhouette should tint yellow in the inventory");
		}
		let mut shaded_only = ItemIconCache::default();
		shaded_only.insert_shaded(
			CaptureId::new(SELF_LUBIKAN_CAPTURE_LABEL),
			bevy::prelude::Handle::default(),
		);
		let mut wrong = CatalogThumbs { icons: Some(&shaded_only) };
		if wrong.image_for_key(key).is_some() {
			anyhow::bail!("Self Lubikan tile must not use the shaded handle");
		}
		Ok(())
	}

	#[test]
	fn spin_reveal_does_not_spawn_a_single_item_png() -> anyhow::Result<()> {
		use super::sync_spin_reveal_item_icon;
		use bevy::ecs::system::RunSystemOnce;
		use bevy::prelude::World;
		use maybraid_skill_map::SkillMapSpinRevealHudView;

		let mut world = World::new();
		world.run_system_once(sync_spin_reveal_item_icon).expect("sync");
		world.flush();
		let count = world.query::<&SkillMapSpinRevealHudView>().iter(&world).count();
		if count != 0 {
			anyhow::bail!("the grant gallery must not spawn the old single-item PNG, got {count}");
		}
		Ok(())
	}
}
