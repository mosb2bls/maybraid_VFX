//! Catalog drop: pad **X** / keyboard **X** or **E** on the Inventory overlay.
//!
//! In-game pause edits drop immediately — the world pile is recoverable.
//! Gallery / create still confirm because the item is deleted. Catalog tiles
//! still toggle on A. [`character_items::Inventory::try_drop`] does the
//! mutation. Section headers and the read-only Lubikan row have no bag index.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::text::Justify;
use character_creation_menus::MenuEvent;
use character_inventory_user::InventoryUser;
use character_items::{
	ActiveInventoryDropPolicy, Inventory, InventoryDropPolicy, InventoryDropRequest,
	InventoryDropped, InventoryItem,
};
use maybraid_character_ui_menu_renderer::{CharacterMenuEvent, MenuButton, OverlaySelectViewport};
use maybraid_input::{MenuNav, MenuNavImpulse, PadButton, VirtualPad};
use menu_components::theme::{HEADER_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW};
use menu_components::{
	spawn_hud_action, spawn_hud_text, ActiveOverlayKey, HudFonts, HudMenu, HudMenuItem,
	HudOverlayMenu, MenuModal, MenuModalDismiss, MenuModalId, MenuModalLayer, MenuModalStack,
	MenuModalSystems, TextMenuInputLock, TextMenuSystems,
};
use player::Player;

use crate::character::{
	CharacterEditBaseline, CharacterMenuState, CharacterScreen, CharacterUiSyncState,
	OverlayUiSyncState,
};
use crate::leave_prompt::CharacterLeavePrompt;
use crate::session::CharacterEditorReturn;

/// Open confirm card while a catalog overlay is up. Empty while idle.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct CharacterDropPrompt {
	pub index: Option<usize>,
	pub label: String,
	/// Ignore the X that opened the card so it cannot confirm.
	pub opened_this_frame: bool,
}

impl CharacterDropPrompt {
	pub fn is_open(&self) -> bool {
		self.index.is_some()
	}

	pub fn open(&mut self, index: usize, label: String, stack: &mut MenuModalStack) {
		self.index = Some(index);
		self.label = label;
		self.opened_this_frame = true;
		stack.open(MenuModalLayer::ITEM_DROP);
		stack.mark_consumed();
	}
}

#[derive(Component, Debug, Default, Clone, Copy)]
struct CharacterDropPromptRoot;

#[derive(Component, Debug, Clone, PartialEq, Eq)]
struct CharacterDropPromptIndex(usize);

#[derive(Component, Debug, Default, Clone, Copy)]
struct CharacterDropPromptMenu;

#[derive(Component, Debug, Default, Clone, Copy)]
struct DropPromptConfirm;

#[derive(Component, Debug, Default, Clone, Copy)]
struct DropPromptCancel;

pub struct CharacterDropPromptPlugin;

impl Plugin for CharacterDropPromptPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<CharacterDropPrompt>()
			.init_resource::<ActiveInventoryDropPolicy>()
			.add_message::<InventoryDropRequest>()
			.add_message::<InventoryDropped>()
			.add_observer(on_drop_prompt_click)
			.add_observer(on_drop_prompt_nav)
			.add_systems(Update, close_drop_on_modal_dismiss.in_set(MenuModalSystems::Apply))
			.add_systems(
				Update,
				intercept_catalog_drop_edges
					.in_set(TextMenuSystems::InputLock)
					.after(MenuModalSystems::Apply),
			)
			.add_systems(
				Update,
				(
					apply_accepted_world_drops,
					close_drop_prompt_without_character,
					sync_character_drop_prompt,
				)
					.chain()
					.after(TextMenuSystems::Navigate),
			);
	}
}

/// Inventory overlay + X, and no prompt already open.
pub fn catalog_drop_edge(overlay: Option<&str>, prompt_open: bool, x: bool) -> bool {
	x && !prompt_open && matches!(overlay, Some("Inventory"))
}

pub fn drop_index_from_event(event: MenuEvent) -> Option<usize> {
	match event {
		MenuEvent::ToggleInventory(index) | MenuEvent::DropInventory(index) => Some(index),
		_ => None,
	}
}

/// In-game world drops can be picked up again, so the confirm card stays closed.
pub fn drop_needs_confirm(require_confirm: bool, recoverable: bool) -> bool {
	require_confirm && !recoverable
}

#[derive(SystemParam)]
struct LiveWorldDrop<'w, 's> {
	requests: MessageWriter<'w, InventoryDropRequest>,
	return_to: Option<Res<'w, CharacterEditorReturn>>,
	owners: Query<'w, 's, Entity, (With<Player>, With<InventoryUser>)>,
}

/// How [`LiveWorldDrop::request`] resolved a catalog confirm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiveDropCommit {
	/// In-game pause edit: the world adapter owns the bag mutation.
	Requested,
	/// Gallery / create: no live world player, mutate the editor copy.
	LocalEditor,
	/// In-game, but the live owner is gone — do not fall back to the editor bag.
	OwnerUnavailable,
}

impl LiveWorldDrop<'_, '_> {
	fn is_live(&self) -> bool {
		self.return_to
			.as_ref()
			.is_some_and(|return_to| return_to.uses_live_world_player())
	}

	fn request(&mut self, index: usize, policy: InventoryDropPolicy) -> LiveDropCommit {
		if !self.is_live() {
			return LiveDropCommit::LocalEditor;
		}
		let Some(owner) = self.owners.iter().next() else {
			return LiveDropCommit::OwnerUnavailable;
		};
		self.requests.write(InventoryDropRequest::new(owner, index).with_policy(policy));
		LiveDropCommit::Requested
	}
}

#[allow(clippy::too_many_arguments)]
fn intercept_catalog_drop_edges(
	mut prompt: ResMut<CharacterDropPrompt>,
	mut stack: ResMut<MenuModalStack>,
	policy: Res<ActiveInventoryDropPolicy>,
	lock: Res<TextMenuInputLock>,
	leave: Option<Res<CharacterLeavePrompt>>,
	overlay: Res<ActiveOverlayKey>,
	pad: Option<Res<VirtualPad>>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	mut live: LiveWorldDrop,
	screens: Query<(), With<CharacterScreen>>,
	viewports: Query<(Entity, &HudMenu), With<OverlaySelectViewport>>,
	items: Query<(&HudMenuItem, &MenuButton<MenuEvent>)>,
) {
	prompt.opened_this_frame = false;
	if screens.is_empty() || lock.0 || leave.is_some_and(|leave| leave.is_open()) {
		return;
	}
	let x = pad.is_some_and(|pad| pad.just_pressed(PadButton::X));
	if !catalog_drop_edge(overlay.0, prompt.is_open(), x) {
		return;
	}
	let Some((viewport, menu)) = viewports.iter().next() else {
		return;
	};
	let Some(index) = items.iter().find_map(|(item, button)| {
		if item.menu == viewport && item.index == menu.selected {
			drop_index_from_event(button.0)
		} else {
			None
		}
	}) else {
		return;
	};
	let Some(inventory) = menu_state.0.inventory.as_ref() else {
		return;
	};
	if inventory.can_drop(index, policy.0).is_err() {
		return;
	}
	let recoverable = live.is_live();
	if drop_needs_confirm(policy.0.require_confirm, recoverable) {
		let label = inventory
			.items
			.get(index)
			.map(character_items::InventoryItem::name)
			.unwrap_or_default();
		prompt.open(index, label, &mut stack);
		return;
	}
	commit_catalog_drop(
		index,
		policy.0,
		&mut menu_state,
		&mut ui_sync,
		&mut overlay_sync,
		&mut menu_events,
		&mut live,
	);
}

fn close_drop_prompt_without_character(
	screens: Query<(), With<CharacterScreen>>,
	mut prompt: ResMut<CharacterDropPrompt>,
	mut stack: ResMut<MenuModalStack>,
	roots: Query<Entity, With<CharacterDropPromptRoot>>,
	mut commands: Commands,
) {
	if screens.is_empty() && prompt.is_open() {
		close_drop_prompt(&mut prompt, &mut stack, &roots, &mut commands);
	}
}

fn close_drop_on_modal_dismiss(
	mut dismiss: MessageReader<MenuModalDismiss>,
	mut prompt: ResMut<CharacterDropPrompt>,
	mut stack: ResMut<MenuModalStack>,
	roots: Query<Entity, With<CharacterDropPromptRoot>>,
	mut commands: Commands,
) {
	if prompt.opened_this_frame {
		return;
	}
	if !dismiss.read().any(|event| event.is(MenuModalId::ITEM_DROP)) {
		return;
	}
	close_drop_prompt(&mut prompt, &mut stack, &roots, &mut commands);
}

fn sync_character_drop_prompt(
	prompt: Res<CharacterDropPrompt>,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	roots: Query<(Entity, &CharacterDropPromptIndex), With<CharacterDropPromptRoot>>,
	mut commands: Commands,
) {
	let have = roots.iter().next().map(|(_, index)| index.0);
	if prompt.index == have {
		return;
	}
	for (entity, _) in &roots {
		commands.entity(entity).try_despawn();
	}
	let Some(index) = prompt.index else {
		return;
	};
	let Ok(screen) = screens.single() else {
		return;
	};
	let Ok(mut screen) = commands.get_entity(screen) else {
		return;
	};
	screen.with_children(|parent| {
		spawn_drop_prompt(parent, &HudFonts::load(asset_server.as_ref()), index, &prompt.label);
	});
}

fn spawn_drop_prompt(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	index: usize,
	label: &str,
) {
	let title =
		if label.is_empty() { String::from("Drop this item?") } else { format!("Drop {label}?") };
	parent
		.spawn((
			CharacterDropPromptRoot,
			CharacterDropPromptIndex(index),
			MenuModal::ITEM_DROP,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				padding: UiRect::bottom(Val::Percent(22.0)),
				..default()
			},
			GlobalZIndex(110),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Button,
				DropPromptCancel,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
			));
			root.spawn((
				Node {
					width: Val::Px(720.0),
					max_width: Val::Percent(92.0),
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::FlexStart,
					row_gap: Val::Px(PANEL_ROW_GAP),
					..default()
				},
				Pickable::default(),
			))
			.with_children(|card| {
				spawn_hud_text(
					card,
					fonts.header(HEADER_FONT_SIZE * 0.42),
					&title,
					TEXT_YELLOW,
					Justify::Left,
				);
				let mut list = card.spawn((
					CharacterDropPromptMenu,
					HudOverlayMenu,
					HudMenu::new(2),
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::FlexStart,
						row_gap: Val::Px(PANEL_ROW_GAP),
						..default()
					},
					Pickable::IGNORE,
				));
				let menu = list.id();
				list.with_children(|list| {
					spawn_hud_action(
						list,
						fonts,
						"Drop",
						JustifyContent::FlexStart,
						(DropPromptConfirm, HudMenuItem { index: 0, menu }),
					);
					spawn_hud_action(
						list,
						fonts,
						"Cancel",
						JustifyContent::FlexStart,
						(DropPromptCancel, HudMenuItem { index: 1, menu }),
					);
				});
			});
		});
}

#[allow(clippy::too_many_arguments)]
fn on_drop_prompt_click(
	mut click: On<Pointer<Click>>,
	confirms: Query<(), With<DropPromptConfirm>>,
	cancels: Query<(), With<DropPromptCancel>>,
	child_of: Query<&ChildOf>,
	roots: Query<Entity, With<CharacterDropPromptRoot>>,
	mut prompt: ResMut<CharacterDropPrompt>,
	mut stack: ResMut<MenuModalStack>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	mut live: LiveWorldDrop,
	policy: Res<ActiveInventoryDropPolicy>,
	mut commands: Commands,
) {
	if pointer_target(&click, &child_of, |entity| confirms.contains(entity)).is_some() {
		click.propagate(false);
		apply_drop_prompt(
			DropPromptAction::Confirm,
			&mut prompt,
			&mut stack,
			&roots,
			&mut menu_state,
			&mut ui_sync,
			&mut overlay_sync,
			&mut menu_events,
			&mut live,
			policy.0,
			&mut commands,
		);
		return;
	}
	if pointer_target(&click, &child_of, |entity| cancels.contains(entity)).is_some() {
		click.propagate(false);
		apply_drop_prompt(
			DropPromptAction::Dismiss,
			&mut prompt,
			&mut stack,
			&roots,
			&mut menu_state,
			&mut ui_sync,
			&mut overlay_sync,
			&mut menu_events,
			&mut live,
			policy.0,
			&mut commands,
		);
	}
}

#[allow(clippy::too_many_arguments)]
fn on_drop_prompt_nav(
	impulse: On<MenuNavImpulse>,
	menus: Query<&HudMenu, With<CharacterDropPromptMenu>>,
	roots: Query<Entity, With<CharacterDropPromptRoot>>,
	mut prompt: ResMut<CharacterDropPrompt>,
	mut stack: ResMut<MenuModalStack>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
	mut live: LiveWorldDrop,
	policy: Res<ActiveInventoryDropPolicy>,
	mut commands: Commands,
) {
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	match impulse.event().nav {
		MenuNav::Select => {
			if prompt.opened_this_frame {
				return;
			}
			let action = if menu.selected == 0 {
				DropPromptAction::Confirm
			} else {
				DropPromptAction::Dismiss
			};
			if action == DropPromptAction::Dismiss {
				stack.mark_consumed();
			}
			apply_drop_prompt(
				action,
				&mut prompt,
				&mut stack,
				&roots,
				&mut menu_state,
				&mut ui_sync,
				&mut overlay_sync,
				&mut menu_events,
				&mut live,
				policy.0,
				&mut commands,
			);
		}
		MenuNav::Back => {
			if prompt.opened_this_frame {
				return;
			}
			stack.mark_consumed();
			apply_drop_prompt(
				DropPromptAction::Dismiss,
				&mut prompt,
				&mut stack,
				&roots,
				&mut menu_state,
				&mut ui_sync,
				&mut overlay_sync,
				&mut menu_events,
				&mut live,
				policy.0,
				&mut commands,
			);
		}
		_ => {}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DropPromptAction {
	Confirm,
	Dismiss,
}

#[allow(clippy::too_many_arguments)]
fn apply_drop_prompt(
	action: DropPromptAction,
	prompt: &mut CharacterDropPrompt,
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<CharacterDropPromptRoot>>,
	menu_state: &mut CharacterMenuState,
	ui_sync: &mut CharacterUiSyncState,
	overlay_sync: &mut OverlayUiSyncState,
	menu_events: &mut MessageWriter<CharacterMenuEvent<MenuEvent>>,
	live: &mut LiveWorldDrop,
	policy: InventoryDropPolicy,
	commands: &mut Commands,
) {
	if action == DropPromptAction::Confirm {
		if let Some(index) = prompt.index {
			commit_catalog_drop(
				index,
				policy,
				menu_state,
				ui_sync,
				overlay_sync,
				menu_events,
				live,
			);
		}
	}
	close_drop_prompt(prompt, stack, roots, commands);
}

fn commit_catalog_drop(
	index: usize,
	policy: InventoryDropPolicy,
	menu_state: &mut CharacterMenuState,
	ui_sync: &mut CharacterUiSyncState,
	overlay_sync: &mut OverlayUiSyncState,
	menu_events: &mut MessageWriter<CharacterMenuEvent<MenuEvent>>,
	live: &mut LiveWorldDrop,
) {
	let Some(inventory) = menu_state.0.inventory.as_ref() else {
		return;
	};
	if inventory.can_drop(index, policy).is_err() {
		return;
	}
	match live.request(index, policy) {
		LiveDropCommit::Requested | LiveDropCommit::OwnerUnavailable => {}
		LiveDropCommit::LocalEditor => {
			apply_drop_inventory(index, policy, menu_state, ui_sync, overlay_sync, menu_events);
		}
	}
}

fn apply_drop_inventory(
	index: usize,
	policy: InventoryDropPolicy,
	menu_state: &mut CharacterMenuState,
	ui_sync: &mut CharacterUiSyncState,
	overlay_sync: &mut OverlayUiSyncState,
	menu_events: &mut MessageWriter<CharacterMenuEvent<MenuEvent>>,
) -> Option<InventoryItem> {
	let item = menu_state
		.0
		.inventory
		.as_ref()
		.and_then(|inventory| inventory.items.get(index).cloned());
	if !menu_state.0.apply_drop(index, policy) {
		return None;
	}
	ui_sync.menu_dirty = true;
	overlay_sync.menu_dirty = true;
	menu_events.write(CharacterMenuEvent::Menu(MenuEvent::DropInventory(index)));
	item
}

/// Copy the leftover bag into the editor and apply only that removal to the
/// discard baseline, so earlier equipment-queue edits stay undoable.
fn apply_accepted_drop_to_editor(
	leftover: &Inventory,
	index: usize,
	menu: &mut character_creation_menus::CharacterMenu,
	baseline: Option<&mut CharacterEditBaseline>,
) {
	menu.set_inventory(leftover.clone());
	if let Some(baseline) = baseline {
		baseline.apply_accepted_drop(index);
	}
}

fn apply_accepted_world_drops(
	mut dropped: MessageReader<InventoryDropped>,
	return_to: Option<Res<CharacterEditorReturn>>,
	screens: Query<(), With<CharacterScreen>>,
	bags: Query<&Inventory>,
	mut menu_state: ResMut<CharacterMenuState>,
	mut baseline: Option<ResMut<CharacterEditBaseline>>,
	mut ui_sync: ResMut<CharacterUiSyncState>,
	mut overlay_sync: ResMut<OverlayUiSyncState>,
	mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
) {
	if screens.is_empty() || return_to.is_none_or(|return_to| !return_to.uses_live_world_player()) {
		return;
	}
	for event in dropped.read() {
		let leftover = match bags.get(event.bag) {
			Ok(bag) => bag.clone(),
			Err(_) => continue,
		};
		apply_accepted_drop_to_editor(
			&leftover,
			event.outcome.index,
			&mut menu_state.0,
			baseline.as_deref_mut(),
		);
		ui_sync.menu_dirty = true;
		overlay_sync.menu_dirty = true;
		menu_events.write(CharacterMenuEvent::Menu(MenuEvent::DropInventory(event.outcome.index)));
	}
}

fn close_drop_prompt(
	prompt: &mut CharacterDropPrompt,
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<CharacterDropPromptRoot>>,
	commands: &mut Commands,
) {
	prompt.index = None;
	prompt.label.clear();
	prompt.opened_this_frame = false;
	stack.close(MenuModalId::ITEM_DROP);
	for entity in roots.iter() {
		commands.entity(entity).try_despawn();
	}
}

fn pointer_target(
	click: &On<Pointer<Click>>,
	child_of: &Query<&ChildOf>,
	matches: impl Fn(Entity) -> bool,
) -> Option<Entity> {
	let mut current = Some(click.entity);
	while let Some(entity) = current {
		if matches(entity) {
			return Some(entity);
		}
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	None
}

#[cfg(test)]
mod tests {
	use super::{
		apply_accepted_drop_to_editor, apply_drop_inventory, catalog_drop_edge,
		commit_catalog_drop, drop_index_from_event, drop_needs_confirm, CharacterDropPrompt,
		LiveWorldDrop,
	};
	use crate::character::{
		CharacterEditBaseline, CharacterMenuState, CharacterUiSyncState, OverlayUiSyncState,
	};
	use crate::session::CharacterEditorReturn;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_creation_menus::MenuEvent;
	use character_inventory_user::InventoryUser;
	use character_items::InventoryDropRequest;
	use character_items::{
		ClothingMaterial, ClothingMesh, FirearmMesh, Inventory, InventoryDropPolicy, InventoryItem,
		ItemColor,
	};
	use maybraid_character_ui_menu_renderer::CharacterMenuEvent;
	use menu_components::{MenuModalId, MenuModalLayer, MenuModalStack};
	use player::Player;

	#[test]
	fn only_inventory_catalogs_offer_drop() {
		assert!(catalog_drop_edge(Some("Inventory"), false, true));
		assert!(!catalog_drop_edge(Some("Clothing"), false, true));
		assert!(!catalog_drop_edge(Some("Weapons"), false, true));
		assert!(!catalog_drop_edge(Some("Loadout"), false, true));
		assert!(!catalog_drop_edge(Some("Inventory"), true, true));
		assert!(!catalog_drop_edge(Some("Inventory"), false, false));
		assert!(!catalog_drop_edge(None, false, true));
	}

	#[test]
	fn toggle_or_drop_events_carry_the_bag_index() {
		assert_eq!(drop_index_from_event(MenuEvent::ToggleInventory(3)), Some(3));
		assert_eq!(drop_index_from_event(MenuEvent::DropInventory(1)), Some(1));
		assert_eq!(drop_index_from_event(MenuEvent::Save), None);
	}

	#[test]
	fn opening_the_prompt_pushes_item_drop_above_the_overlay() {
		let mut prompt = CharacterDropPrompt::default();
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::OVERLAY_SELECT);
		prompt.open(2, String::from("Test"), &mut stack);
		assert_eq!(prompt.index, Some(2));
		assert_eq!(stack.top().map(|layer| layer.id), Some(MenuModalId::ITEM_DROP));
		assert!(stack.consumed());
	}

	#[test]
	fn confirm_applies_drop_and_compacts_the_weapon_queue() -> anyhow::Result<()> {
		let items = vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Reltor),
		];
		let mut world = World::new();
		world.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
		world.insert_resource(CharacterMenuState::for_create(items));
		world.init_resource::<CharacterUiSyncState>();
		world.init_resource::<OverlayUiSyncState>();
		world
			.run_system_once(
				|mut menu_state: ResMut<CharacterMenuState>,
				 mut ui_sync: ResMut<CharacterUiSyncState>,
				 mut overlay_sync: ResMut<OverlayUiSyncState>,
				 mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>| {
					apply_drop_inventory(
						1,
						InventoryDropPolicy::PLACEHOLDER,
						&mut menu_state,
						&mut ui_sync,
						&mut overlay_sync,
						&mut menu_events,
					);
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		let menu = &world.resource::<CharacterMenuState>().0;
		let inventory = menu.inventory.as_ref().ok_or_else(|| anyhow::anyhow!("inventory"))?;
		assert_eq!(inventory.items.len(), 2);
		assert_eq!(inventory.weapons, vec![1]);
		assert_eq!(
			inventory.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
		assert!(world.resource::<CharacterUiSyncState>().menu_dirty);
		Ok(())
	}

	fn mixed_editor_bag() -> Inventory {
		Inventory::with_starter_outfit(vec![
			InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			),
			InventoryItem::firearm(FirearmMesh::Bullpup),
			InventoryItem::firearm(FirearmMesh::Reltor),
		])
	}

	#[test]
	fn accepted_live_drop_updates_editor_from_leftover_bag() {
		let leftover = Inventory {
			items: vec![InventoryItem::clothing(
				ClothingMesh::Pants,
				ClothingMaterial::Cloth,
				ItemColor::Natural,
			)],
			clothing: vec![0],
			weapons: Vec::new(),
			skills: Vec::new(),
		};
		let mut menu = CharacterMenuState::for_create(mixed_editor_bag().items.clone()).0;
		let mut baseline = CharacterEditBaseline::capture(&menu);
		apply_accepted_drop_to_editor(&leftover, 1, &mut menu, Some(&mut baseline));
		assert_eq!(menu.inventory.as_ref().map(|bag| bag.items.len()), Some(1));
		assert_eq!(baseline.inventory.items.len(), leftover.items.len() + 1);
		assert_eq!(baseline.inventory.weapons.len(), 1);
	}

	#[test]
	fn discard_after_equip_change_and_drop_restores_pre_change_equipment() {
		let mut menu = CharacterMenuState::for_create(mixed_editor_bag().items).0;
		let mut baseline = CharacterEditBaseline::capture(&menu);
		assert_eq!(baseline.inventory.clothing, vec![0]);
		assert!(menu.apply(MenuEvent::ToggleInventory(0)));
		assert!(menu.inventory.as_ref().is_some_and(|bag| bag.clothing.is_empty()));
		let leftover = menu.inventory.clone().expect("inventory");
		let leftover = {
			let mut leftover = leftover;
			assert!(leftover.try_drop(1, InventoryDropPolicy::PLACEHOLDER).is_ok());
			leftover
		};
		apply_accepted_drop_to_editor(&leftover, 1, &mut menu, Some(&mut baseline));
		assert!(menu.inventory.as_ref().is_some_and(|bag| bag.clothing.is_empty()));
		assert_eq!(baseline.inventory.clothing, vec![0]);
		assert_eq!(baseline.inventory.items.len(), 2);
		baseline.restore_into(&mut menu);
		let restored = menu.inventory.as_ref().expect("inventory");
		assert_eq!(restored.clothing, vec![0]);
		assert_eq!(restored.items.len(), 2);
		assert_eq!(
			restored.primary_weapon().and_then(InventoryItem::firearm_mesh),
			Some(FirearmMesh::Reltor)
		);
	}

	#[test]
	fn confirmation_rechecks_the_resolved_policy() -> anyhow::Result<()> {
		let items = vec![InventoryItem::firearm(FirearmMesh::Bullpup)];
		let mut world = World::new();
		world.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
		world.insert_resource(CharacterMenuState::for_create(items));
		world.init_resource::<CharacterUiSyncState>();
		world.init_resource::<OverlayUiSyncState>();
		let refuse =
			InventoryDropPolicy { allow_equipped: false, ..InventoryDropPolicy::PLACEHOLDER };
		world
			.run_system_once(
				move |mut menu_state: ResMut<CharacterMenuState>,
				      mut ui_sync: ResMut<CharacterUiSyncState>,
				      mut overlay_sync: ResMut<OverlayUiSyncState>,
				      mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>| {
					assert!(apply_drop_inventory(
						0,
						refuse,
						&mut menu_state,
						&mut ui_sync,
						&mut overlay_sync,
						&mut menu_events,
					)
					.is_none());
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert_eq!(
			world
				.resource::<CharacterMenuState>()
				.0
				.inventory
				.as_ref()
				.map(|bag| bag.items.len()),
			Some(1)
		);
		Ok(())
	}

	#[test]
	fn gallery_drop_mutates_the_menu_bag_locally() -> anyhow::Result<()> {
		let items = vec![InventoryItem::firearm(FirearmMesh::Bullpup)];
		let mut world = World::new();
		world.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
		world.insert_resource(CharacterMenuState::for_create(items));
		world.init_resource::<CharacterUiSyncState>();
		world.init_resource::<OverlayUiSyncState>();
		world
			.run_system_once(
				|mut menu_state: ResMut<CharacterMenuState>,
				 mut ui_sync: ResMut<CharacterUiSyncState>,
				 mut overlay_sync: ResMut<OverlayUiSyncState>,
				 mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>| {
					apply_drop_inventory(
						0,
						InventoryDropPolicy::PLACEHOLDER,
						&mut menu_state,
						&mut ui_sync,
						&mut overlay_sync,
						&mut menu_events,
					)
					.expect("dropped");
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		assert!(world
			.resource::<CharacterMenuState>()
			.0
			.inventory
			.as_ref()
			.is_some_and(|bag| bag.items.is_empty()));
		Ok(())
	}

	fn commit_drop_world(live: bool, with_owner: bool) -> World {
		let items = vec![InventoryItem::firearm(FirearmMesh::Bullpup)];
		let mut world = World::new();
		world.init_resource::<Messages<CharacterMenuEvent<MenuEvent>>>();
		world.init_resource::<Messages<InventoryDropRequest>>();
		world.insert_resource(CharacterMenuState::for_create(items));
		world.init_resource::<CharacterUiSyncState>();
		world.init_resource::<OverlayUiSyncState>();
		if live {
			world.insert_resource(CharacterEditorReturn::InGame);
		}
		if with_owner {
			let bag = world.spawn_empty().id();
			world.spawn((Player, InventoryUser::carrying(bag)));
		}
		world
	}

	fn run_commit_catalog_drop(world: &mut World) -> anyhow::Result<()> {
		world
			.run_system_once(
				|mut menu_state: ResMut<CharacterMenuState>,
				 mut ui_sync: ResMut<CharacterUiSyncState>,
				 mut overlay_sync: ResMut<OverlayUiSyncState>,
				 mut menu_events: MessageWriter<CharacterMenuEvent<MenuEvent>>,
				 mut live: LiveWorldDrop| {
					commit_catalog_drop(
						0,
						InventoryDropPolicy::PLACEHOLDER,
						&mut menu_state,
						&mut ui_sync,
						&mut overlay_sync,
						&mut menu_events,
						&mut live,
					);
				},
			)
			.map_err(|error| anyhow::anyhow!("{error:?}"))?;
		Ok(())
	}

	#[test]
	fn catalog_commit_on_the_local_editor_mutates_the_bag() -> anyhow::Result<()> {
		let mut world = commit_drop_world(false, false);
		run_commit_catalog_drop(&mut world)?;
		assert!(world
			.resource::<CharacterMenuState>()
			.0
			.inventory
			.as_ref()
			.is_some_and(|bag| bag.items.is_empty()));
		assert_eq!(world.resource_mut::<Messages<InventoryDropRequest>>().drain().count(), 0);
		Ok(())
	}

	#[test]
	fn live_catalog_commit_requests_the_world_owner() -> anyhow::Result<()> {
		let mut world = commit_drop_world(true, true);
		run_commit_catalog_drop(&mut world)?;
		assert_eq!(
			world
				.resource::<CharacterMenuState>()
				.0
				.inventory
				.as_ref()
				.map(|bag| bag.items.len()),
			Some(1),
			"live commit must not mutate the editor copy"
		);
		assert_eq!(world.resource_mut::<Messages<InventoryDropRequest>>().drain().count(), 1);
		Ok(())
	}

	#[test]
	fn live_catalog_commit_without_an_owner_refuses_local_fallback() -> anyhow::Result<()> {
		let mut world = commit_drop_world(true, false);
		run_commit_catalog_drop(&mut world)?;
		assert_eq!(
			world
				.resource::<CharacterMenuState>()
				.0
				.inventory
				.as_ref()
				.map(|bag| bag.items.len()),
			Some(1),
			"missing live owner must not fall back to the editor bag"
		);
		assert_eq!(world.resource_mut::<Messages<InventoryDropRequest>>().drain().count(), 0);
		Ok(())
	}

	#[test]
	fn open_drop_prompt_owns_nav_until_it_closes() {
		let mut prompt = CharacterDropPrompt::default();
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::OVERLAY_SELECT);
		assert!(!stack.owns_nav());
		prompt.open(1, String::from("Bullpup"), &mut stack);
		assert!(stack.owns_nav());
		assert_eq!(stack.nav_owner(), Some(MenuModalId::ITEM_DROP));
		stack.close(MenuModalId::ITEM_DROP);
		prompt.index = None;
		assert!(!stack.owns_nav());
	}

	#[test]
	fn recoverable_world_drops_skip_the_confirm_card() {
		assert!(drop_needs_confirm(true, false));
		assert!(!drop_needs_confirm(true, true));
		assert!(!drop_needs_confirm(false, false));
	}
}
