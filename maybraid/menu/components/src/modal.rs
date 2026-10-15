//! Shared menu-modal stack: open/close plus ordered Back consumption.
//!
//! Screens must not see Back (pad **B**, Escape, or [`ScreenBack`]) while any
//! layer is open or while the press that closed a layer is still held. Each
//! popup registers a [`MenuModal`] on its root and/or calls
//! [`MenuModalStack::open`]; [`consume_menu_modal_back`] takes Back for the
//! top layer before any screen leave system runs.

use bevy::prelude::*;
use maybraid_input::{MenuNav, MenuNavPad, PadButton, VirtualPad};

use crate::controls::section::ActiveOverlayKey;
use crate::controls::short_text::ShortTextModal;
use crate::single_select::text_cursor::ScreenBackPressed;

/// Ordered Back / dismiss work inside [`TextMenuSystems::InputLock`](crate::TextMenuSystems::InputLock).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuModalSystems {
	/// Clear the one-shot consume flag and rebuild auto-synced layers.
	Sync,
	/// Top modal takes Back / Escape.
	Consume,
	/// Individual modals apply dismiss.
	Apply,
}

/// Stable identity for a layer on [`MenuModalStack`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct MenuModalId(pub &'static str);

impl MenuModalId {
	pub const OVERLAY_SELECT: Self = Self("overlay-select");
	pub const SHORT_TEXT: Self = Self("short-text");
	pub const ITEM_DROP: Self = Self("item-drop");
	pub const CHARACTER_LEAVE: Self = Self("character-leave");

	pub fn as_str(self) -> &'static str {
		self.0
	}

	/// Resource-driven layers rebuilt every [`MenuModalSystems::Sync`].
	pub fn is_auto(self) -> bool {
		matches!(self, Self::OVERLAY_SELECT | Self::SHORT_TEXT)
	}

	/// Confirm / text layers take directional nav, stick, and wheel away from
	/// the parent list. Overlay select *is* the focused catalog, so it does not.
	pub fn owns_nav(self) -> bool {
		matches!(self, Self::SHORT_TEXT | Self::ITEM_DROP | Self::CHARACTER_LEAVE)
	}
}

/// One open popup. Higher [`order`](Self::order) is closer to the user.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuModalLayer {
	pub id: MenuModalId,
	pub order: i32,
}

impl MenuModalLayer {
	pub const OVERLAY_SELECT: Self = Self { id: MenuModalId::OVERLAY_SELECT, order: 10 };
	pub const SHORT_TEXT: Self = Self { id: MenuModalId::SHORT_TEXT, order: 20 };
	pub const ITEM_DROP: Self = Self { id: MenuModalId::ITEM_DROP, order: 25 };
	pub const CHARACTER_LEAVE: Self = Self { id: MenuModalId::CHARACTER_LEAVE, order: 30 };

	pub fn new(id: MenuModalId, order: i32) -> Self {
		Self { id, order }
	}
}

/// Marker on a modal root. Presence documents the layer; auto ids also sync
/// from [`ShortTextModal`] / [`ActiveOverlayKey`].
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MenuModal {
	pub id: MenuModalId,
	pub order: i32,
}

impl MenuModal {
	pub const OVERLAY_SELECT: Self =
		Self { id: MenuModalId::OVERLAY_SELECT, order: MenuModalLayer::OVERLAY_SELECT.order };
	pub const SHORT_TEXT: Self =
		Self { id: MenuModalId::SHORT_TEXT, order: MenuModalLayer::SHORT_TEXT.order };
	pub const ITEM_DROP: Self =
		Self { id: MenuModalId::ITEM_DROP, order: MenuModalLayer::ITEM_DROP.order };
	pub const CHARACTER_LEAVE: Self =
		Self { id: MenuModalId::CHARACTER_LEAVE, order: MenuModalLayer::CHARACTER_LEAVE.order };

	pub fn layer(self) -> MenuModalLayer {
		MenuModalLayer { id: self.id, order: self.order }
	}
}

/// Dismiss the named layer. Written by [`consume_menu_modal_back`].
#[derive(Message, Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuModalDismiss {
	pub id: MenuModalId,
}

impl MenuModalDismiss {
	pub fn new(id: MenuModalId) -> Self {
		Self { id }
	}

	pub fn is(self, id: MenuModalId) -> bool {
		self.id == id
	}
}

/// Ordered stack of open menus plus the Back/Start latch that keeps a closing
/// edge from landing on the parent screen.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct MenuModalStack {
	layers: Vec<MenuModalLayer>,
	/// Top modal took Back this frame (or a caller marked the opening edge).
	consumed: bool,
	back_held: bool,
	start_held: bool,
}

impl MenuModalStack {
	pub fn is_open(&self) -> bool {
		!self.layers.is_empty()
	}

	pub fn contains(&self, id: MenuModalId) -> bool {
		self.layers.iter().any(|layer| layer.id == id)
	}

	pub fn top(&self) -> Option<MenuModalLayer> {
		self.layers.iter().max_by_key(|layer| layer.order).copied()
	}

	pub fn layers(&self) -> &[MenuModalLayer] {
		&self.layers
	}

	pub fn consumed(&self) -> bool {
		self.consumed
	}

	/// Top layer steals stick / D-pad / keyboard / wheel from the parent HUD.
	pub fn owns_nav(&self) -> bool {
		self.nav_owner().is_some()
	}

	pub fn nav_owner(&self) -> Option<MenuModalId> {
		self.top().map(|layer| layer.id).filter(|id| id.owns_nav())
	}

	pub fn blocks_screen_back(&self) -> bool {
		self.is_open() || self.consumed || self.back_held
	}

	pub fn blocks_screen_select(&self) -> bool {
		self.is_open() || self.consumed || self.start_held
	}

	/// Drop the one-shot consume flag. Held latches persist until release.
	pub fn begin_frame(&mut self) {
		self.consumed = false;
	}

	pub fn open(&mut self, layer: MenuModalLayer) {
		self.layers.retain(|open| open.id != layer.id);
		self.layers.push(layer);
	}

	pub fn close(&mut self, id: MenuModalId) {
		self.layers.retain(|layer| layer.id != id);
	}

	/// Remember that this frame's Back/Start belongs to a modal.
	pub fn mark_consumed(&mut self) {
		self.consumed = true;
	}

	/// If `back` is set and a layer is open, consume it and return the top id.
	pub fn take_back(&mut self, back: bool) -> Option<MenuModalId> {
		if !back {
			return None;
		}
		let id = self.top()?.id;
		self.consumed = true;
		Some(id)
	}

	/// Keep Start/Back until release so a late `just_pressed` cannot land on
	/// the HUD after the popup closes.
	pub fn tick_buttons(&mut self, back_down: bool, start_down: bool) {
		let armed = self.is_open() || self.consumed;
		self.back_held = Self::latch(self.back_held, armed, back_down);
		self.start_held = Self::latch(self.start_held, armed, start_down);
	}

	pub fn latch(armed: bool, modal: bool, down: bool) -> bool {
		if !down {
			false
		} else if modal {
			true
		} else {
			armed
		}
	}

	/// Rebuild resource-driven layers. Manual layers (leave prompt) stay until
	/// [`Self::close`].
	pub fn sync_auto(&mut self, short_text: bool, overlay: bool) {
		self.set_auto(MenuModalLayer::SHORT_TEXT, short_text);
		self.set_auto(MenuModalLayer::OVERLAY_SELECT, overlay);
	}

	fn set_auto(&mut self, layer: MenuModalLayer, open: bool) {
		if open {
			self.open(layer);
		} else {
			self.close(layer.id);
		}
	}
}

pub fn begin_menu_modal_frame(mut stack: ResMut<MenuModalStack>) {
	stack.begin_frame();
}

pub fn sync_menu_modal_stack(
	mut stack: ResMut<MenuModalStack>,
	short_text: Res<ShortTextModal>,
	overlay: Res<ActiveOverlayKey>,
) {
	stack.sync_auto(short_text.is_open(), overlay.0.is_some());
}

pub fn consume_menu_modal_back(
	mut stack: ResMut<MenuModalStack>,
	mut nav: ResMut<MenuNavPad>,
	keyboard: Res<ButtonInput<KeyCode>>,
	mut dismiss: MessageWriter<MenuModalDismiss>,
) {
	let back = nav.just_pressed(MenuNav::Back) || keyboard.just_pressed(KeyCode::Escape);
	let Some(id) = stack.take_back(back) else {
		return;
	};
	nav.events.retain(|event| *event != MenuNav::Back);
	dismiss.write(MenuModalDismiss::new(id));
}

pub fn tick_menu_modal_holds(
	mut stack: ResMut<MenuModalStack>,
	pad: Option<Res<VirtualPad>>,
	keyboard: Res<ButtonInput<KeyCode>>,
	nav: Res<MenuNavPad>,
) {
	let back_down = pad.as_deref().is_some_and(|pad| pad.pressed(PadButton::B))
		|| keyboard.pressed(KeyCode::Escape)
		|| nav.just_pressed(MenuNav::Back);
	let start_down = pad.as_deref().is_some_and(|pad| pad.pressed(PadButton::Start))
		|| keyboard.pressed(KeyCode::Enter);
	stack.tick_buttons(back_down, start_down);
}

/// HudMenu under the exclusive nav owner. Short text uses the keypad pad.
pub fn hud_menu_under_modal(
	id: MenuModalId,
	roots: &Query<(Entity, &MenuModal)>,
	menus: &Query<Entity, With<crate::HudMenu>>,
	children: &Query<&Children>,
) -> Option<Entity> {
	for (root, modal) in roots.iter() {
		if modal.id != id {
			continue;
		}
		if menus.contains(root) {
			return Some(root);
		}
		if let Some(menu) = children.iter_descendants(root).find(|entity| menus.contains(*entity)) {
			return Some(menu);
		}
	}
	None
}

/// True when `entity` sits under the stack's exclusive nav owner (or none is open).
pub fn entity_under_nav_owner(
	entity: Entity,
	stack: &MenuModalStack,
	roots: &Query<(Entity, &MenuModal)>,
	child_of: &Query<&ChildOf>,
) -> bool {
	let Some(id) = stack.nav_owner() else {
		return true;
	};
	let mut current = Some(entity);
	while let Some(here) = current {
		if roots.get(here).is_ok_and(|(_, modal)| modal.id == id) {
			return true;
		}
		current = child_of.get(here).ok().map(ChildOf::parent);
	}
	false
}

/// Click on [`crate::ScreenBack`], or pad B, while no modal is holding Back.
pub fn consume_screen_back(
	nav: &MenuNavPad,
	stack: &MenuModalStack,
	backs: &mut MessageReader<ScreenBackPressed>,
) -> bool {
	let clicked = backs.read().next().is_some();
	if stack.blocks_screen_back() {
		return false;
	}
	clicked || nav.just_pressed(MenuNav::Back)
}

#[cfg(test)]
mod tests {
	use super::{
		consume_menu_modal_back, consume_screen_back, tick_menu_modal_holds, MenuModalDismiss,
		MenuModalId, MenuModalLayer, MenuModalStack,
	};
	use crate::single_select::text_cursor::ScreenBackPressed;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_input::{MenuNav, MenuNavPad};

	#[test]
	fn top_layer_is_highest_order() {
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::OVERLAY_SELECT);
		stack.open(MenuModalLayer::SHORT_TEXT);
		assert_eq!(stack.top().map(|layer| layer.id), Some(MenuModalId::SHORT_TEXT));
		stack.open(MenuModalLayer::ITEM_DROP);
		assert_eq!(stack.top().map(|layer| layer.id), Some(MenuModalId::ITEM_DROP));
		stack.open(MenuModalLayer::CHARACTER_LEAVE);
		assert_eq!(stack.top().map(|layer| layer.id), Some(MenuModalId::CHARACTER_LEAVE));
	}

	#[test]
	fn entity_under_nav_owner_hides_the_parent_catalog() {
		use super::{entity_under_nav_owner, MenuModal};
		use crate::HudMenu;
		use bevy::ecs::system::RunSystemOnce;

		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		let catalog = world.spawn(HudMenu::new(3)).id();
		let drop_item = world.spawn_empty().id();
		let drop_root = world.spawn(MenuModal::ITEM_DROP).id();
		world.entity_mut(drop_root).add_child(drop_item);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::ITEM_DROP);
		let (under_drop, under_catalog) = world
			.run_system_once(
				move |stack: Res<MenuModalStack>,
				      roots: Query<(Entity, &MenuModal)>,
				      child_of: Query<&ChildOf>| {
					(
						entity_under_nav_owner(drop_item, &stack, &roots, &child_of),
						entity_under_nav_owner(catalog, &stack, &roots, &child_of),
					)
				},
			)
			.expect("under");
		assert!(under_drop);
		assert!(!under_catalog);
		world.resource_mut::<MenuModalStack>().close(MenuModalId::ITEM_DROP);
		let restored = world
			.run_system_once(
				move |stack: Res<MenuModalStack>,
				      roots: Query<(Entity, &MenuModal)>,
				      child_of: Query<&ChildOf>| {
					entity_under_nav_owner(catalog, &stack, &roots, &child_of)
				},
			)
			.expect("restored");
		assert!(restored);
	}

	#[test]
	fn confirm_layers_own_nav_and_overlay_does_not() {
		assert!(!MenuModalId::OVERLAY_SELECT.owns_nav());
		assert!(MenuModalId::SHORT_TEXT.owns_nav());
		assert!(MenuModalId::ITEM_DROP.owns_nav());
		assert!(MenuModalId::CHARACTER_LEAVE.owns_nav());
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::OVERLAY_SELECT);
		assert!(!stack.owns_nav());
		assert_eq!(stack.nav_owner(), None);
		stack.open(MenuModalLayer::ITEM_DROP);
		assert!(stack.owns_nav());
		assert_eq!(stack.nav_owner(), Some(MenuModalId::ITEM_DROP));
		stack.close(MenuModalId::ITEM_DROP);
		assert!(!stack.owns_nav());
	}

	#[test]
	fn take_back_consumes_only_the_top_layer() {
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::OVERLAY_SELECT);
		stack.open(MenuModalLayer::SHORT_TEXT);
		assert_eq!(stack.take_back(true), Some(MenuModalId::SHORT_TEXT));
		assert!(stack.consumed());
		assert!(stack.blocks_screen_back());
		assert!(stack.contains(MenuModalId::SHORT_TEXT));
		stack.close(MenuModalId::SHORT_TEXT);
		stack.begin_frame();
		assert_eq!(stack.take_back(true), Some(MenuModalId::OVERLAY_SELECT));
	}

	#[test]
	fn empty_stack_does_not_consume_back() {
		let mut stack = MenuModalStack::default();
		assert!(stack.take_back(true).is_none());
		assert!(!stack.blocks_screen_back());
	}

	#[test]
	fn latch_holds_back_after_close_until_release() {
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::SHORT_TEXT);
		assert_eq!(stack.take_back(true), Some(MenuModalId::SHORT_TEXT));
		stack.close(MenuModalId::SHORT_TEXT);
		stack.tick_buttons(true, false);
		stack.begin_frame();
		assert!(stack.blocks_screen_back());
		stack.tick_buttons(false, false);
		assert!(!stack.blocks_screen_back());
	}

	#[test]
	fn start_from_a_modal_does_not_bubble_after_submit() {
		assert!(MenuModalStack::latch(false, true, true));
		assert!(MenuModalStack::latch(true, false, true));
		assert!(!MenuModalStack::latch(true, false, false));
		assert!(!MenuModalStack::latch(false, false, true));

		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::SHORT_TEXT);
		stack.tick_buttons(false, true);
		assert!(stack.blocks_screen_select());
		stack.close(MenuModalId::SHORT_TEXT);
		stack.begin_frame();
		stack.tick_buttons(false, true);
		assert!(stack.blocks_screen_select());
		stack.tick_buttons(false, false);
		assert!(!stack.blocks_screen_select());
	}

	#[test]
	fn sync_auto_rebuilds_only_resource_layers() {
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::CHARACTER_LEAVE);
		stack.sync_auto(true, false);
		assert!(stack.contains(MenuModalId::SHORT_TEXT));
		assert!(stack.contains(MenuModalId::CHARACTER_LEAVE));
		assert!(!stack.contains(MenuModalId::OVERLAY_SELECT));
		stack.sync_auto(false, true);
		assert!(!stack.contains(MenuModalId::SHORT_TEXT));
		assert!(stack.contains(MenuModalId::OVERLAY_SELECT));
		assert!(stack.contains(MenuModalId::CHARACTER_LEAVE));
	}

	#[test]
	fn consume_screen_back_ignores_parent_while_stack_blocks() {
		assert!(!screen_back_taken(true, true));
		assert!(screen_back_taken(true, false));
		assert!(!screen_back_taken(false, false));
	}

	#[test]
	fn consume_system_writes_dismiss_and_strips_nav_back() {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		assert!(world.resource::<MenuModalStack>().consumed());
		assert!(!world.resource::<MenuNavPad>().just_pressed(MenuNav::Back));
		let dismissed = world
			.run_system_once(|mut dismiss: MessageReader<MenuModalDismiss>| {
				dismiss.read().next().map(|event| event.id)
			})
			.expect("read dismiss");
		assert_eq!(dismissed, Some(MenuModalId::SHORT_TEXT));
	}

	#[test]
	fn hold_tick_keeps_escape_from_reaching_the_parent_next_frame() {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		world.resource_mut::<MenuModalStack>().mark_consumed();
		world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
		world.run_system_once(tick_menu_modal_holds).expect("tick");
		world.resource_mut::<MenuModalStack>().close(MenuModalId::SHORT_TEXT);
		world.resource_mut::<MenuModalStack>().begin_frame();
		assert!(world.resource::<MenuModalStack>().blocks_screen_back());
	}

	fn screen_back_taken(back: bool, modal_open: bool) -> bool {
		let mut world = World::new();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<Messages<ScreenBackPressed>>();
		if modal_open {
			world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		}
		if back {
			world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		}
		world
			.run_system_once(
				|nav: Res<MenuNavPad>,
				 stack: Res<MenuModalStack>,
				 mut backs: MessageReader<ScreenBackPressed>| {
					consume_screen_back(&nav, &stack, &mut backs)
				},
			)
			.expect("consume_screen_back")
	}
}
