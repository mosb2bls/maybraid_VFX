//! Right-justified character-creator panel over the playground camera.

use bevy::prelude::*;
use character_creation_menus::{CharacterMenu, MenuEvent};
use character_items::Inventory;
use character_ui_menu::{AssetThumbnailDisplay, MenuComponent};
use item_icons::{
	request_inventory_icons, CaptureClient, ItemCaptureSubjects, ItemIconCache, ItemIconDiscovery,
	ItemIconPlugin, ItemIconSystems,
};
use maybraid_character_ui_menu_renderer::{
	find_overlay_node, overlay_closes_on_pick, render_overlay_body, spawn_overlay_shell,
	CharacterHudSystems, CharacterMenuEvent, MaybraidCharacterMenuRendererPlugin, MaybraidMenuSink,
	MenuButton, MenuJustify, MenuSink, MenuThumbnailContext, OverlayClose, OverlayOpen,
	OverlaySelectRoot, OverlaySelectViewport, RenderContext,
};
use maybraid_menu_controller::MenuController;
use maybraid_skill_map::{
	spawn_skill_map_spin_reveal_image, SkillMapSpinRevealHud, SkillMapSpinRevealHudView,
	SkillMapTileMaterialPlugin,
};
use menu_components::theme::{CORNER_BOTTOM, CORNER_INSET};
use menu_components::{
	spawn_corner_action, spawn_scroll_pane, ActiveOverlayKey, HudFonts, HudMenu, HudMenuItem,
	HudOverlayMenu, MenuActivate, MenuFocus, MenuModalId, MenuModalLayer, MenuModalStack,
	ScreenBack, ShortTextChange, PANEL_ROW_GAP, SPIN_REVEAL_SLOT_SIZE, TEXT_YELLOW,
	TEXT_YELLOW_FAINT,
};
use menu_screens::{
	take_menu_show_request, MenuScreen, SpinRevealCurrent, SpinRevealItems, SpinRevealScreen,
};

use crate::leave_prompt::CharacterLeavePromptPlugin;

const PANEL_WIDTH: f32 = 480.0;
const PANEL_HEIGHT_PERCENT: f32 = 82.0;

/// Queue a character-panel spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
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
struct CharacterUiSyncState {
	menu_dirty: bool,
	save_label: Option<&'static str>,
	icon_generation: u64,
}

#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
struct OverlaySelectState {
	open: Option<&'static str>,
}

#[derive(Resource, Default)]
struct OverlayUiSyncState {
	open: Option<&'static str>,
	menu_dirty: bool,
	icon_generation: u64,
}

#[derive(Component)]
struct CharacterPanelViewport;

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
		self.icons.and_then(|cache| cache.shaded_for_key(key))
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
				(
					apply_show_character,
					sync_character_ui,
					sync_overlay_select,
					sync_spin_reveal_item_icon,
				)
					.chain()
					.in_set(CharacterHudSystems::Sync),
			);
	}
}

fn apply_show_character(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	requests: Query<Entity, With<RequestShowCharacter>>,
	existing: Query<Entity, With<MenuScreen>>,
	menu_state: Res<CharacterMenuState>,
	baseline: Option<Res<CharacterEditBaseline>>,
	mut sync_state: ResMut<CharacterUiSyncState>,
	mut overlay: ResMut<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut active_overlay: ResMut<ActiveOverlayKey>,
	mut stack: ResMut<MenuModalStack>,
	icons: Option<Res<ItemIconCache>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
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
			root.spawn((
				Node {
					position_type: PositionType::Absolute,
					top: Val::Px(10.0),
					right: Val::Px(10.0),
					width: Val::Px(PANEL_WIDTH),
					height: Val::Percent(PANEL_HEIGHT_PERCENT),
					padding: UiRect::all(Val::Px(16.0)),
					flex_direction: FlexDirection::Column,
					overflow: Overflow::clip(),
					..default()
				},
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
		commands.entity(viewport).despawn_related::<Children>();
		let mut item_count = 0;
		commands.entity(viewport).with_children(|panel| {
			item_count = populate_character_ui_panel(
				panel,
				fonts,
				menu_state,
				viewport,
				&mut prewarm,
				icons,
			);
		});
		if let Some(corner) = back_corner {
			commands.entity(corner).despawn_related::<Children>();
			commands.entity(corner).with_children(|corner| {
				spawn_corner_action(corner, fonts, "Back", ScreenBack);
			});
		}
		if let Some(corner) = save_corner {
			commands.entity(corner).despawn_related::<Children>();
			if let Some(label) = save_chrome(&menu_state.0, baseline) {
				commands.entity(corner).with_children(|corner| {
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
		commands.entity(viewport).insert(HudMenu::retain(item_count, keep));
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
	screens: Query<Entity, With<CharacterScreen>>,
	overlays: Query<Entity, With<OverlaySelectRoot>>,
	viewports: Query<(Entity, Option<&HudMenu>), With<OverlaySelectViewport>>,
	menu_state: Res<CharacterMenuState>,
	overlay: Res<OverlaySelectState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	icons: Option<Res<ItemIconCache>>,
) {
	let Ok(screen) = screens.single() else {
		return;
	};
	let icon_generation = icons.as_ref().map(|cache| cache.generation).unwrap_or(0);
	let menu_changed = overlay_sync.menu_dirty
		|| menu_state.is_changed()
		|| overlay_sync.icon_generation != icon_generation;
	let open_changed = overlay_sync.open != overlay.open;
	if !open_changed && !menu_changed {
		return;
	}
	overlay_sync.open = overlay.open;
	overlay_sync.menu_dirty = false;
	overlay_sync.icon_generation = icon_generation;

	let Some(key) = overlay.open else {
		for entity in &overlays {
			commands.entity(entity).try_despawn();
		}
		return;
	};

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
		commands.entity(screen).with_children(|root| {
			viewport = spawn_overlay_shell(root, &fonts, key, title_color);
		});
		populate_overlay_viewport(
			&mut commands,
			viewport,
			&fonts,
			node,
			None,
			menu_state.0.overlay_editable(key),
			icons.as_deref(),
		);
		return;
	}

	for (viewport, menu) in &viewports {
		populate_overlay_viewport(
			&mut commands,
			viewport,
			&fonts,
			node,
			menu.copied(),
			menu_state.0.overlay_editable(key),
			icons.as_deref(),
		);
	}
}

fn populate_overlay_viewport<E: Copy + Send + Sync + 'static>(
	commands: &mut Commands,
	viewport: Entity,
	fonts: &HudFonts,
	node: &character_ui_menu::MenuNode<E>,
	previous: Option<HudMenu>,
	interactive: bool,
	icons: Option<&ItemIconCache>,
) {
	let mut prewarm = Vec::new();
	let mut thumbnails = CatalogThumbs { icons };
	commands.entity(viewport).despawn_related::<Children>();
	let mut item_count = 0;
	commands.entity(viewport).with_children(|body| {
		let mut context = RenderContext {
			fonts,
			thumbnails: &mut thumbnails,
			asset_thumbnails: AssetThumbnailDisplay::None,
			prewarm: &mut prewarm,
			hud_menu: viewport,
			hud_item_count: 0,
			interactive,
			lock_appearance: !interactive,
		};
		render_overlay_body(node, body, &mut context, MenuJustify::Left);
		item_count = context.hud_item_count;
	});
	commands
		.entity(viewport)
		.insert((HudMenu::retain(item_count, previous), HudOverlayMenu));
}

fn discover_menu_item_icons(
	menu_state: Res<CharacterMenuState>,
	spin_items: Option<Res<SpinRevealItems>>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut client: CaptureClient,
) {
	if let Some(inventory) = menu_state.0.inventory.as_ref() {
		request_inventory_icons(inventory, &mut discovery, &mut subjects, &mut client);
	}
	if let Some(spin) = spin_items.as_deref() {
		let bag = Inventory { items: spin.0.clone(), ..Inventory::default() };
		request_inventory_icons(&bag, &mut discovery, &mut subjects, &mut client);
	}
}

/// Bind the shaded catalog PNG for the current starter item. The live inspect
/// mesh is not spawned on this screen — that pair stacked two item views.
fn sync_spin_reveal_item_icon(
	mut commands: Commands,
	spin: Option<Res<SpinRevealCurrent>>,
	spin_screens: Query<(), With<SpinRevealScreen>>,
	icons: Option<Res<ItemIconCache>>,
	mut hosts: Query<&mut Visibility, With<SkillMapSpinRevealHud>>,
	mut views: Query<&mut ImageNode, With<SkillMapSpinRevealHudView>>,
) {
	let wanted = (!spin_screens.is_empty())
		.then_some(spin.as_deref())
		.flatten()
		.filter(|spin| spin.revealed)
		.and_then(|spin| icons.as_ref().and_then(|cache| cache.shaded_for_item(&spin.item)));
	let Some(image) = wanted else {
		for mut visibility in &mut hosts {
			*visibility = Visibility::Hidden;
		}
		return;
	};
	if hosts.is_empty() {
		spawn_skill_map_spin_reveal_image(&mut commands, image, SPIN_REVEAL_SLOT_SIZE);
		return;
	}
	for mut visibility in &mut hosts {
		*visibility = Visibility::Inherited;
	}
	for mut node in &mut views {
		node.image = image.clone();
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
	fn spin_reveal_shows_the_shaded_clothing_png() -> anyhow::Result<()> {
		use super::{sync_spin_reveal_item_icon, SkillMapSpinRevealHudView};
		use bevy::ecs::system::RunSystemOnce;
		use bevy::prelude::World;
		use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};
		use item_icons::{CaptureId, ItemIconCache};
		use menu_screens::{SpinRevealCurrent, SpinRevealScreen};

		let item =
			InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red);
		let label =
			item.capture_id_label().ok_or_else(|| anyhow::anyhow!("clothing capture id"))?;
		let mut cache = ItemIconCache::default();
		cache.insert_shaded(CaptureId::new(label), bevy::prelude::Handle::default());

		let mut world = World::new();
		world.spawn(SpinRevealScreen);
		world.insert_resource(SpinRevealCurrent { item, revealed: true });
		world.insert_resource(cache);
		world.run_system_once(sync_spin_reveal_item_icon).expect("sync");
		world.flush();
		let count = world.query::<&SkillMapSpinRevealHudView>().iter(&world).count();
		if count != 1 {
			anyhow::bail!("revealed clothing should spawn the catalog PNG slot, got {count}");
		}
		Ok(())
	}
}
