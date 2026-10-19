//! Starter Kit: the shared inventory layout in read-only mode.

use bevy::prelude::*;
use character_creation_menus::{
	inventory_paint_state, InventoryLayout, InventoryPaint, INVENTORY_CAPTURE_TIMEOUT,
};
use character_items::{self_lubikan_catalog_key, Inventory, InventoryItem};
use item_icons::{
	request_inventory_icons, CaptureClient, ItemCaptureSubjects, ItemIconCache, ItemIconDiscovery,
	ItemIconPlugin, PoseCaptureSubjects,
};
use maybraid_character_ui_menu_renderer::{
	fill_inventory_layout, fill_inventory_loading, spawn_starter_kit_shell, MenuThumbnailContext,
};
use maybraid_input::{MenuNav, MenuNavImpulse};
use maybraid_menu_controller::MenuController;
use menu_components::{
	spawn_hud_action, spawn_hud_text, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu,
	KeyboardMenuNav, MenuComponentsPlugin, ScreenBack, TextMenuInputLock, MENU_CLEAR, TEXT_YELLOW,
};
use std::time::Duration;
use tanz_user::request_inventory_tanz_icons;

use crate::input::add_menu_input;
use crate::show::{add_menu_show_systems, MenuScreens, MenuShowRequest};
use crate::MenuScreen;

/// Queue a Starter Kit spawn. Pair with [`SpinRevealItems`].
#[derive(Component, Debug, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowSpinReveal;

/// Granted items shown on the Starter Kit.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealItems(pub Vec<InventoryItem>);

/// Hosts bind catalog art here. The layout is the only view; do not spawn a
/// live inspect mesh or camera on this screen.
#[derive(Resource, Clone, Debug)]
pub struct SpinRevealCurrent {
	pub items: Vec<InventoryItem>,
	pub revealed: bool,
}

/// Marker on the spawned Starter Kit root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct SpinRevealScreen;

/// Scroll viewport that owns Starter Kit focus.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct StarterKitViewport;

/// Continue footer. Activate finishes the grant.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct StarterKitContinue;

/// Continue from the Starter Kit into the body editor.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum SpinRevealChoice {
	#[default]
	Advance,
}

/// Fired once the player continues from the Starter Kit.
#[derive(Message, Clone, Debug)]
pub struct SpinRevealFinished {
	pub items: Vec<InventoryItem>,
}

pub fn request_show_spin_reveal(commands: &mut Commands, items: Vec<InventoryItem>) {
	commands.insert_resource(SpinRevealItems(items));
	commands.spawn(RequestShowSpinReveal);
}

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpinRevealSystems {
	Apply,
}

pub struct SpinRevealScreenPlugin;

impl Plugin for SpinRevealScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		if !app.is_plugin_added::<ItemIconPlugin>() {
			app.add_plugins(ItemIconPlugin);
		}
		app.add_message::<SpinRevealChoice>()
			.add_message::<SpinRevealFinished>()
			.add_observer(click_starter_kit_continue)
			.add_observer(select_starter_kit_continue)
			.add_systems(
				Update,
				(
					enter_starter_kit_continue,
					finish_spin_reveal,
					drop_stale_gallery,
					reveal_starter_kit_icons,
				)
					.chain()
					.in_set(SpinRevealSystems::Apply),
			);
		add_menu_show_systems(app, apply_show_spin_reveal);
	}
}

fn apply_show_spin_reveal(
	mut commands: Commands,
	requests: Query<(), With<RequestShowSpinReveal>>,
	screens: MenuScreens,
	items: Option<Res<SpinRevealItems>>,
	assets: Res<AssetServer>,
	time: Res<Time>,
	mut discovery: ResMut<ItemIconDiscovery>,
	mut subjects: ResMut<ItemCaptureSubjects>,
	mut poses: ResMut<PoseCaptureSubjects>,
	mut client: CaptureClient,
) {
	if screens.take(&mut commands, &requests).is_none() {
		return;
	}
	let Some(items) = items else {
		return;
	};
	let bag = Inventory { items: items.0.clone(), ..Inventory::default() };
	request_inventory_icons(&bag, &mut discovery, &mut subjects, &mut client);
	request_inventory_tanz_icons(&bag, &mut discovery, &mut poses, &mut client);
	spawn_starter_kit(&mut commands, &items, &HudFonts::load(assets.as_ref()), time.elapsed());
}

/// If another screen replaced the Starter Kit, drop the current-item binding so
/// hosts do not keep painting catalog art over the new screen.
fn drop_stale_gallery(
	mut commands: Commands,
	current: Option<Res<SpinRevealCurrent>>,
	screens: Query<Entity, With<SpinRevealScreen>>,
) {
	if current.is_none() || !screens.is_empty() {
		return;
	}
	commands.remove_resource::<SpinRevealCurrent>();
	commands.remove_resource::<StarterKitIconWait>();
}

fn spawn_starter_kit(
	commands: &mut Commands,
	items: &SpinRevealItems,
	fonts: &HudFonts,
	started: Duration,
) {
	commands.insert_resource(SpinRevealCurrent { items: items.0.clone(), revealed: false });
	commands.insert_resource(StarterKitIconWait { started, revealed: false });
	let layout = InventoryLayout::starter_kit(items.0.clone());
	let mut viewport = Entity::PLACEHOLDER;
	let mut continue_parent = Entity::PLACEHOLDER;
	commands
		.spawn((
			SpinRevealScreen,
			MenuScreen,
			MenuController::default(),
			BackgroundColor(MENU_CLEAR),
			Node {
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				flex_direction: FlexDirection::Column,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|root| {
			viewport =
				spawn_starter_kit_shell(root, fonts, layout.title, TEXT_YELLOW, StarterKitViewport);
			continue_parent = root
				.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::Center,
						padding: UiRect::bottom(Val::Px(24.0)),
						..default()
					},
					Pickable::IGNORE,
				))
				.id();
			root.spawn((
				Button,
				ScreenBack,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(16.0),
					bottom: Val::Px(16.0),
					..default()
				},
			))
			.with_children(|back| {
				spawn_hud_text(
					back,
					fonts.item(22.0),
					"Back",
					TEXT_YELLOW,
					bevy::text::Justify::Left,
				);
			});
		});
	fill_inventory_loading(commands, viewport, fonts);
	let count = 0;
	if let Ok(mut footer) = commands.get_entity(continue_parent) {
		footer.with_children(|footer| {
			spawn_hud_action(
				footer,
				fonts,
				"Continue",
				JustifyContent::Center,
				(StarterKitContinue, HudMenuItem { index: count, menu: viewport }),
			);
		});
	}
	if let Ok(mut entity) = commands.get_entity(viewport) {
		entity.insert((HudMenu::retain(count + 1, None), HudOverlayMenu));
	}
}

#[derive(Resource, Debug)]
struct StarterKitIconWait {
	started: Duration,
	revealed: bool,
}

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
		self.icons?.catalog_for_key(key)
	}

	fn tint_for_key(&mut self, key: u64) -> Color {
		if key == self_lubikan_catalog_key() {
			TEXT_YELLOW
		} else {
			Color::WHITE
		}
	}
}

fn reveal_starter_kit_icons(
	mut commands: Commands,
	time: Res<Time>,
	assets: Res<AssetServer>,
	cache: Option<Res<ItemIconCache>>,
	current: Option<Res<SpinRevealCurrent>>,
	wait: Option<ResMut<StarterKitIconWait>>,
	viewports: Query<Entity, With<StarterKitViewport>>,
	continues: Query<Entity, With<StarterKitContinue>>,
) {
	let Some(mut wait) = wait else {
		return;
	};
	if wait.revealed {
		return;
	}
	let Some(current) = current else {
		return;
	};
	let Ok(viewport) = viewports.single() else {
		return;
	};
	let layout = InventoryLayout::starter_kit(current.items.clone());
	let icons = cache.as_deref();
	let reports = layout
		.captures
		.iter()
		.map(|need| icons.is_some_and(|cache| cache.filled_or_failed(need.key, need.silhouette)));
	let waited = time.elapsed().saturating_sub(wait.started);
	if inventory_paint_state(reports, waited, INVENTORY_CAPTURE_TIMEOUT) != InventoryPaint::Ready {
		return;
	}
	if waited >= INVENTORY_CAPTURE_TIMEOUT
		&& !layout.captures.iter().all(|need| {
			icons.is_some_and(|cache| cache.filled_or_failed(need.key, need.silhouette))
		}) {
		warn!(
			waited_secs = waited.as_secs_f32(),
			"starter kit icons timed out; showing the layout without every capture"
		);
	}
	let fonts = HudFonts::load(assets.as_ref());
	let mut thumbs = CatalogThumbs { icons };
	let count = fill_inventory_layout(
		&mut commands,
		viewport,
		&fonts,
		&layout.node,
		false,
		None,
		&mut thumbs,
	);
	wait.revealed = true;
	commands.insert_resource(SpinRevealCurrent { items: current.items.clone(), revealed: true });
	for entity in &continues {
		commands.entity(entity).try_insert(HudMenuItem { index: count, menu: viewport });
	}
	if let Ok(mut entity) = commands.get_entity(viewport) {
		entity.insert((HudMenu::retain(count + 1, None), HudOverlayMenu));
	}
}

fn click_starter_kit_continue(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	continues: Query<(), With<StarterKitContinue>>,
	mut choices: MessageWriter<SpinRevealChoice>,
) {
	if lock.0 || continues.get(click.entity).is_err() {
		return;
	}
	choices.write(SpinRevealChoice::Advance);
}

fn select_starter_kit_continue(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	menus: Query<&HudMenu, With<StarterKitViewport>>,
	items: Query<(&HudMenuItem, Option<&StarterKitContinue>)>,
	mut choices: MessageWriter<SpinRevealChoice>,
) {
	if lock.0 || impulse.event().nav != MenuNav::Select {
		return;
	}
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	if items.iter().any(|(item, cont)| {
		item.menu == impulse.entity && item.index == menu.selected && cont.is_some()
	}) {
		choices.write(SpinRevealChoice::Advance);
	}
}

fn enter_starter_kit_continue(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	menus: Query<(Entity, &HudMenu), With<StarterKitViewport>>,
	items: Query<(&HudMenuItem, Option<&StarterKitContinue>)>,
	mut choices: MessageWriter<SpinRevealChoice>,
) {
	if !keyboard_nav.is_enabled() || lock.0 || !keyboard.just_pressed(KeyCode::Enter) {
		return;
	}
	let Some((menu_entity, menu)) = menus.iter().next() else {
		return;
	};
	if items.iter().any(|(item, cont)| {
		item.menu == menu_entity && item.index == menu.selected && cont.is_some()
	}) {
		choices.write(SpinRevealChoice::Advance);
	}
}

fn finish_spin_reveal(
	mut commands: Commands,
	mut choices: MessageReader<SpinRevealChoice>,
	mut finished: MessageWriter<SpinRevealFinished>,
	items: Option<Res<SpinRevealItems>>,
) {
	if !choices.read().any(|choice| *choice == SpinRevealChoice::Advance) {
		return;
	}
	let Some(items) = items else {
		return;
	};
	finished.write(SpinRevealFinished { items: items.0.clone() });
	commands.remove_resource::<SpinRevealCurrent>();
	commands.remove_resource::<StarterKitIconWait>();
}

impl SpinRevealScreen {
	pub fn layout(items: &[InventoryItem]) -> InventoryLayout {
		InventoryLayout::starter_kit(items.to_vec())
	}
}

#[cfg(test)]
mod tests {
	use super::{
		drop_stale_gallery, enter_starter_kit_continue, finish_spin_reveal, SpinRevealChoice,
		SpinRevealCurrent, SpinRevealFinished, SpinRevealItems, SpinRevealScreen,
		StarterKitContinue, StarterKitViewport,
	};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_creation_menus::{InventoryLayoutMode, STARTER_KIT_LABEL};
	use character_items::{ClothingMaterial, ClothingMesh, InventoryItem, ItemColor};

	fn pants() -> InventoryItem {
		InventoryItem::clothing(ClothingMesh::Pants, ClothingMaterial::Cloth, ItemColor::Red)
	}

	#[test]
	fn a_replaced_gallery_drops_the_current_binding() -> Result<(), Box<dyn std::error::Error>> {
		let mut world = World::new();
		world.insert_resource(SpinRevealCurrent { items: vec![pants()], revealed: true });
		world
			.run_system_once(drop_stale_gallery)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		if world.get_resource::<SpinRevealCurrent>().is_some() {
			return Err("the current binding should end once the gallery is gone".into());
		}
		Ok(())
	}

	#[test]
	fn a_live_gallery_keeps_the_current_binding() -> Result<(), Box<dyn std::error::Error>> {
		let mut world = World::new();
		world.spawn(SpinRevealScreen);
		world.insert_resource(SpinRevealCurrent { items: vec![pants()], revealed: true });
		world
			.run_system_once(drop_stale_gallery)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		if world.get_resource::<SpinRevealCurrent>().is_none() {
			return Err("a live gallery must keep its current binding".into());
		}
		Ok(())
	}

	#[test]
	fn continue_finishes_every_granted_item() -> Result<(), Box<dyn std::error::Error>> {
		let items = vec![
			pants(),
			InventoryItem::clothing(
				ClothingMesh::TankTop,
				ClothingMaterial::Hawaiian,
				ItemColor::Blue,
			),
		];
		let mut world = World::new();
		world.init_resource::<Messages<SpinRevealChoice>>();
		world.init_resource::<Messages<SpinRevealFinished>>();
		world.insert_resource(SpinRevealItems(items.clone()));
		world
			.run_system_once(|mut writer: MessageWriter<SpinRevealChoice>| {
				writer.write(SpinRevealChoice::Advance);
			})
			.map_err(|error| format!("{error:?}"))?;
		world
			.run_system_once(finish_spin_reveal)
			.map_err(|error| format!("{error:?}"))?;
		world.flush();
		let got = world
			.run_system_once(|mut reader: MessageReader<SpinRevealFinished>| {
				reader.read().next().map(|event| event.items.clone())
			})
			.map_err(|error| format!("{error:?}"))?;
		if got.as_ref() != Some(&items) {
			return Err("finished items must be the whole grant".into());
		}
		Ok(())
	}

	fn continue_world(lock: bool, keyboard_nav: bool) -> World {
		use menu_components::{HudMenu, HudMenuItem, KeyboardMenuNav, TextMenuInputLock};

		let mut world = World::new();
		world.insert_resource(TextMenuInputLock(lock));
		world.insert_resource(KeyboardMenuNav(keyboard_nav));
		world.init_resource::<Messages<SpinRevealChoice>>();
		let mut keys = ButtonInput::<KeyCode>::default();
		keys.press(KeyCode::Enter);
		world.insert_resource(keys);
		let menu = world.spawn((HudMenu::retain(1, None), StarterKitViewport)).id();
		world.spawn((HudMenuItem { index: 0, menu }, StarterKitContinue));
		world
	}

	fn continue_choices(world: &mut World) -> usize {
		world.resource_mut::<Messages<SpinRevealChoice>>().drain().count()
	}

	#[test]
	fn locked_text_input_does_not_advance_on_enter() -> Result<(), Box<dyn std::error::Error>> {
		let mut world = continue_world(true, true);
		world
			.run_system_once(enter_starter_kit_continue)
			.map_err(|error| format!("{error:?}"))?;
		if continue_choices(&mut world) != 0 {
			return Err("console / text lock must not activate Continue".into());
		}
		Ok(())
	}

	#[test]
	fn disabled_keyboard_nav_does_not_advance_on_enter() -> Result<(), Box<dyn std::error::Error>> {
		let mut world = continue_world(false, false);
		world
			.run_system_once(enter_starter_kit_continue)
			.map_err(|error| format!("{error:?}"))?;
		if continue_choices(&mut world) != 0 {
			return Err("controller-owned Enter must not be handled twice".into());
		}
		Ok(())
	}

	#[test]
	fn unlocked_enter_advances_when_continue_is_focused() -> Result<(), Box<dyn std::error::Error>>
	{
		let mut world = continue_world(false, true);
		world
			.run_system_once(enter_starter_kit_continue)
			.map_err(|error| format!("{error:?}"))?;
		if continue_choices(&mut world) != 1 {
			return Err("unlocked keyboard Enter should activate Continue".into());
		}
		Ok(())
	}

	#[test]
	fn grant_uses_the_read_only_starter_kit_layout() {
		let layout = SpinRevealScreen::layout(&[pants()]);
		assert_eq!(layout.title, STARTER_KIT_LABEL);
		assert_eq!(layout.mode, InventoryLayoutMode::ReadOnly);
		assert!(!layout.allows(character_creation_menus::MenuEvent::ToggleInventory(0)));
		assert!(!layout.allows(character_creation_menus::MenuEvent::DropInventory(0)));
		assert_eq!(
			layout.section_labels(),
			["Clothing", "Tools", "Skill Maps", "Tanzes", "Lubikan"]
		);
	}
}
