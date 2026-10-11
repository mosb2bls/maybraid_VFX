//! Text column, authored as BSN [`Scene`]s. Default anchor is bottom-left.
//!
//! Pickable rows stamp a screen-specific choice component `E`. Selection
//! triggers [`MenuFocus<E>`] on the [`TextMenu`]; click / Enter trigger
//! [`MenuActivate<E>`]. Both bubble to the screen. [`republish_menu_activate`]
//! copies activate onto [`Message<E>`] for listeners outside the screen.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, Justify};

use crate::theme::{
	BARLOW_BLACK, BARLOW_SEMIBOLD, COLUMN_BOTTOM, COLUMN_INSET, CORNER_BOTTOM, CORNER_INSET,
	HEADER_FONT_SIZE, HEADER_MARGIN_BOTTOM, ITEM_FONT_SIZE, ITEM_ROW_GAP, TEXT_YELLOW,
	TEXT_YELLOW_FAINT, TEXT_YELLOW_FAINT_FOCUS, TEXT_YELLOW_HOVER,
};
use maybraid_input::{MenuNav, MenuNavImpulse};

/// When `true`, arrow keys, Enter, and pick activations stay with the command line.
#[derive(Resource, Default, Clone, Copy, Debug)]
pub struct TextMenuInputLock(pub bool);

/// When `false`, raw keyboard nav is off so a [`MenuNavImpulse`] controller owns it.
#[derive(Resource, Clone, Copy, Debug)]
pub struct KeyboardMenuNav(pub bool);

impl Default for KeyboardMenuNav {
	fn default() -> Self {
		Self(true)
	}
}

impl KeyboardMenuNav {
	pub fn is_enabled(self) -> bool {
		self.0
	}
}

/// Root of a vertical text menu. [`selected`] is the keyboard / hover index.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenu {
	pub selected: usize,
	pub item_count: usize,
}

impl TextMenu {
	pub fn new(item_count: usize) -> Self {
		Self { selected: 0, item_count }
	}

	pub fn with_selected(item_count: usize, selected: usize) -> Self {
		Self {
			selected: if item_count == 0 { 0 } else { selected.min(item_count - 1) },
			item_count,
		}
	}

	pub fn step(&mut self, delta: i32) {
		if self.item_count == 0 {
			return;
		}
		let n = self.item_count as i32;
		self.selected = (self.selected as i32 + delta).rem_euclid(n) as usize;
	}

	pub fn apply_nav(&mut self, nav: MenuNav) {
		match nav {
			MenuNav::Up | MenuNav::Left => self.step(-1),
			MenuNav::Down | MenuNav::Right => self.step(1),
			MenuNav::Select | MenuNav::Back => {}
		}
	}
}

/// Chrome on a selectable row. The screen’s choice type is a separate component.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuItem {
	pub index: usize,
	pub idle: Color,
	pub active: Color,
}

/// Locked pickable row: still receives [`MenuFocus`], never [`MenuActivate`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuItemLocked;

/// Row that consumes left/right to change a value instead of moving the cursor.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuStepper;

impl TextMenuItem {
	pub fn yellow(index: usize) -> Self {
		Self { index, idle: TEXT_YELLOW, active: TEXT_YELLOW_HOVER }
	}

	/// Dampened chrome for a locked row (idle and focused).
	pub fn faint_yellow(index: usize) -> Self {
		Self { index, idle: TEXT_YELLOW_FAINT, active: TEXT_YELLOW_FAINT_FOCUS }
	}

	/// Pickable row that stamps `action` for [`MenuFocus`] / [`MenuActivate`].
	pub fn scene<E>(
		self,
		label: impl Into<String>,
		action: E,
		align: TextColumnAlign,
	) -> impl Scene + 'static
	where
		E: Component + Copy + Default + Unpin + Send + Sync + 'static,
	{
		let label = label.into();
		let justify_content = align.justify_content();
		let text_justify = align.text_justify();
		bsn! {
			Button
			template_value(self)
			template_value(action)
			Node {
				padding: UiRect::axes(px(0.0), px(2.0)),
				justify_content: justify_content,
				align_items: AlignItems::Center,
			}
			BackgroundColor(Color::NONE)
			Children [(
				template_value(Text::new(label))
				TextFont {
					font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
					font_size: px(ITEM_FONT_SIZE),
				}
				TextColor(TEXT_YELLOW)
				TextLayout::new(text_justify, bevy::text::LineBreak::NoWrap)
				TextMenuItemLabel
				Pickable::IGNORE
			)]
		}
	}
}

/// Marker on the label [`Text`] child of a [`TextMenuItem`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuItemLabel;

/// Non-interactive title above the options.
pub struct TextMenuHeader {
	pub label: String,
	pub margin_left: f32,
}

impl TextMenuHeader {
	pub fn new(label: impl Into<String>) -> Self {
		Self { label: label.into(), margin_left: 0.0 }
	}

	pub fn with_margin_left(mut self, margin_left: f32) -> Self {
		self.margin_left = margin_left;
		self
	}

	pub fn scene(self) -> impl Scene + 'static {
		let label = self.label;
		let margin_left = self.margin_left;
		bsn! {
			template_value(Text::new(label))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_BLACK),
				font_size: px(HEADER_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW)
			Node {
				margin: UiRect::new(px(margin_left), px(0.0), px(0.0), px(HEADER_MARGIN_BOTTOM)),
			}
			Pickable::IGNORE
		}
	}
}

/// Where a shrink-wrapped text column sits on the screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextColumnAnchor {
	/// Home and gallery: inset from the top-left.
	#[default]
	TopLeft,
	/// Inset from the bottom-left (above a description strip).
	BottomLeft,
	/// Inset from the bottom-right.
	BottomRight,
	/// Pause-style: shrink-wrap and let the parent flex-center this node.
	Center,
}

/// How labels sit inside the column and inside each row.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TextColumnAlign {
	/// Reserved cursor gutter; labels share a left edge.
	#[default]
	Start,
	/// Each label is centered on the column axis.
	Center,
}

impl TextColumnAlign {
	pub fn items(self) -> AlignItems {
		match self {
			Self::Start => AlignItems::FlexStart,
			Self::Center => AlignItems::Center,
		}
	}

	pub fn justify_content(self) -> JustifyContent {
		match self {
			Self::Start => JustifyContent::FlexStart,
			Self::Center => JustifyContent::Center,
		}
	}

	pub fn text_justify(self) -> Justify {
		match self {
			Self::Start => Justify::Left,
			Self::Center => Justify::Center,
		}
	}
}

impl TextColumnAnchor {
	pub fn node(self, align: TextColumnAlign) -> Node {
		let align_items = align.items();
		match self {
			Self::TopLeft => Node {
				position_type: PositionType::Absolute,
				left: Val::Px(COLUMN_INSET),
				top: Val::Px(COLUMN_INSET),
				flex_direction: FlexDirection::Column,
				align_items,
				row_gap: Val::Px(ITEM_ROW_GAP),
				..default()
			},
			Self::BottomLeft => Node {
				position_type: PositionType::Absolute,
				left: Val::Px(COLUMN_INSET),
				bottom: Val::Px(COLUMN_BOTTOM),
				flex_direction: FlexDirection::Column,
				align_items,
				row_gap: Val::Px(ITEM_ROW_GAP),
				..default()
			},
			Self::BottomRight => Node {
				position_type: PositionType::Absolute,
				right: Val::Px(CORNER_INSET),
				bottom: Val::Px(CORNER_BOTTOM),
				flex_direction: FlexDirection::Column,
				align_items,
				row_gap: Val::Px(ITEM_ROW_GAP),
				..default()
			},
			Self::Center => Node {
				flex_direction: FlexDirection::Column,
				align_items,
				row_gap: Val::Px(ITEM_ROW_GAP),
				..default()
			},
		}
	}
}

/// Header plus labeled actions.
pub struct TextMenuColumn<E> {
	pub header: String,
	pub items: Vec<(String, E)>,
	pub anchor: TextColumnAnchor,
	pub align: TextColumnAlign,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> TextMenuColumn<E> {
	pub fn new(
		header: impl Into<String>,
		items: impl IntoIterator<Item = (impl Into<String>, E)>,
	) -> Self {
		Self {
			header: header.into(),
			items: items.into_iter().map(|(label, action)| (label.into(), action)).collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
		}
	}

	pub fn anchored(mut self, anchor: TextColumnAnchor) -> Self {
		self.anchor = anchor;
		self
	}

	pub fn aligned(mut self, align: TextColumnAlign) -> Self {
		self.align = align;
		self
	}

	pub fn scene(self) -> impl Scene + 'static {
		let item_count = self.items.len();
		let mut children: Vec<Box<dyn Scene>> = Vec::with_capacity(item_count + 1);
		children.push(Box::new(TextMenuHeader::new(self.header).scene()));
		for (index, (label, action)) in self.items.into_iter().enumerate() {
			children.push(Box::new(TextMenuItem::yellow(index).scene(label, action, self.align)));
		}
		let node = self.anchor.node(self.align);
		bsn! {
			template_value(TextMenu::new(item_count))
			template_value(node)
			Children [ {children} ]
		}
	}
}

/// Walk parents from a row to the [`TextMenu`] that owns it.
///
/// Cursor columns wrap each pickable in a layout node (for subtext), so the
/// immediate parent is not the menu.
pub fn text_menu_entity(
	start: Entity,
	child_of: &Query<&ChildOf>,
	menus: &Query<(), With<TextMenu>>,
) -> Option<Entity> {
	let mut entity = start;
	loop {
		if menus.contains(entity) {
			return Some(entity);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}

pub fn select_text_menu_item_on_over(
	over: On<Pointer<Over>>,
	items: Query<&TextMenuItem>,
	child_of: Query<&ChildOf>,
	mut menus: Query<&mut TextMenu>,
) {
	let Ok(item) = items.get(over.entity) else {
		return;
	};
	let mut entity = over.entity;
	loop {
		if let Ok(mut menu) = menus.get_mut(entity) {
			if menu.item_count == 0 {
				return;
			}
			menu.selected = item.index.min(menu.item_count - 1);
			return;
		}
		let Ok(parent) = child_of.get(entity) else {
			return;
		};
		entity = parent.parent();
	}
}

/// In-screen focus: triggered on the [`TextMenu`], bubbles to the screen root.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuFocus<E> {
	pub entity: Entity,
	pub choice: E,
}

/// In-screen activate: triggered on the [`TextMenu`], bubbles to the screen root.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuActivate<E> {
	pub entity: Entity,
	pub choice: E,
}

/// In-screen left/right step: triggered on the [`TextMenu`], bubbles to the screen.
#[derive(EntityEvent, Clone, Copy, Debug)]
#[entity_event(propagate, auto_propagate)]
pub struct MenuStep<E> {
	pub entity: Entity,
	pub choice: E,
	pub delta: i32,
}

fn selected_item<E: Component + Copy>(
	menu_entity: Entity,
	menu: &TextMenu,
	items: &Query<(Entity, &TextMenuItem, &E, &ChildOf)>,
	child_of: &Query<&ChildOf>,
) -> Option<(Entity, E)> {
	items.iter().find_map(|(entity, item, choice, item_child)| {
		(item.index == menu.selected && entity_is_under(item_child.parent(), menu_entity, child_of))
			.then_some((entity, *choice))
	})
}

fn selected_choice<E: Component + Copy>(
	menu_entity: Entity,
	menu: &TextMenu,
	items: &Query<(Entity, &TextMenuItem, &E, &ChildOf)>,
	child_of: &Query<&ChildOf>,
) -> Option<E> {
	selected_item(menu_entity, menu, items, child_of).map(|(_, choice)| choice)
}

fn entity_is_under(mut entity: Entity, ancestor: Entity, child_of: &Query<&ChildOf>) -> bool {
	loop {
		if entity == ancestor {
			return true;
		}
		let Ok(parent) = child_of.get(entity) else {
			return false;
		};
		entity = parent.parent();
	}
}

/// Trigger [`MenuFocus<E>`] on a menu when its [`TextMenu::selected`] changes.
pub fn emit_menu_focus<E: Component + Copy + Send + Sync + 'static>(
	menus: Query<(Entity, &TextMenu), Changed<TextMenu>>,
	items: Query<(Entity, &TextMenuItem, &E, &ChildOf)>,
	child_of: Query<&ChildOf>,
	mut commands: Commands,
) {
	for (menu_entity, menu) in &menus {
		if let Some(choice) = selected_choice(menu_entity, menu, &items, &child_of) {
			commands.trigger(MenuFocus { entity: menu_entity, choice });
		}
	}
}

/// Trigger [`MenuActivate<E>`] on the row’s parent menu.
pub fn emit_menu_activate_on_click<E: Component + Copy>(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	items: Query<&E, With<TextMenuItem>>,
	locked: Query<(), With<MenuItemLocked>>,
	child_of: Query<&ChildOf>,
	menus: Query<(), With<TextMenu>>,
	mut commands: Commands,
) {
	if lock.0 || locked.contains(click.entity) {
		return;
	}
	let Ok(choice) = items.get(click.entity) else {
		return;
	};
	let Some(menu_entity) = text_menu_entity(click.entity, &child_of, &menus) else {
		return;
	};
	commands.trigger(MenuActivate { entity: menu_entity, choice: *choice });
}

/// Trigger [`MenuActivate<E>`] for the keyboard-selected row.
pub fn emit_menu_activate_on_enter<E: Component + Copy + Send + Sync + 'static>(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	menus: Query<(Entity, &TextMenu)>,
	items: Query<(Entity, &TextMenuItem, &E, &ChildOf)>,
	locked: Query<(), With<MenuItemLocked>>,
	child_of: Query<&ChildOf>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled() || lock.0 || !keyboard.just_pressed(KeyCode::Enter) {
		return;
	}
	for (menu_entity, menu) in &menus {
		if let Some((item, choice)) = selected_item(menu_entity, menu, &items, &child_of) {
			if locked.contains(item) {
				continue;
			}
			commands.trigger(MenuActivate { entity: menu_entity, choice });
		}
	}
}

/// Activate the focused row when a controller fires [`MenuNav::Select`].
pub fn emit_menu_activate_on_nav<E: Component + Copy + Send + Sync + 'static>(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	menus: Query<&TextMenu>,
	items: Query<(Entity, &TextMenuItem, &E, &ChildOf)>,
	locked: Query<(), With<MenuItemLocked>>,
	child_of: Query<&ChildOf>,
	mut commands: Commands,
) {
	if lock.0 || impulse.event().nav != MenuNav::Select {
		return;
	}
	let menu_entity = impulse.entity;
	let Ok(menu) = menus.get(menu_entity) else {
		return;
	};
	if let Some((item, choice)) = selected_item(menu_entity, menu, &items, &child_of) {
		if locked.contains(item) {
			return;
		}
		commands.trigger(MenuActivate { entity: menu_entity, choice });
	}
}

pub fn apply_text_menu_nav(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	mut menus: Query<&mut TextMenu>,
	items: Query<(&TextMenuItem, &ChildOf), With<MenuStepper>>,
	child_of: Query<&ChildOf>,
) {
	if lock.0 {
		return;
	}
	let Ok(mut menu) = menus.get_mut(impulse.entity) else {
		return;
	};
	let nav = impulse.event().nav;
	if matches!(nav, MenuNav::Left | MenuNav::Right)
		&& selected_item_is_stepper(impulse.entity, &menu, &items, &child_of)
	{
		return;
	}
	menu.apply_nav(nav);
}

fn selected_item_is_stepper(
	menu_entity: Entity,
	menu: &TextMenu,
	items: &Query<(&TextMenuItem, &ChildOf), With<MenuStepper>>,
	child_of: &Query<&ChildOf>,
) -> bool {
	items.iter().any(|(item, item_child)| {
		item.index == menu.selected && entity_is_under(item_child.parent(), menu_entity, child_of)
	})
}

/// Trigger [`MenuStep<E>`] when the focused row is a [`MenuStepper`].
pub fn emit_menu_step_on_nav<E: Component + Copy + Send + Sync + 'static>(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	menus: Query<&TextMenu>,
	items: Query<(Entity, &TextMenuItem, &E, &ChildOf), With<MenuStepper>>,
	locked: Query<(), With<MenuItemLocked>>,
	child_of: Query<&ChildOf>,
	mut commands: Commands,
) {
	if lock.0 {
		return;
	}
	let delta = match impulse.event().nav {
		MenuNav::Left => -1,
		MenuNav::Right => 1,
		_ => return,
	};
	let menu_entity = impulse.entity;
	let Ok(menu) = menus.get(menu_entity) else {
		return;
	};
	if let Some((item, choice)) = items.iter().find_map(|(entity, item, choice, item_child)| {
		(item.index == menu.selected
			&& entity_is_under(item_child.parent(), menu_entity, &child_of))
		.then_some((entity, *choice))
	}) {
		if locked.contains(item) {
			return;
		}
		commands.trigger(MenuStep { entity: menu_entity, choice, delta });
	}
}

/// Screen-boundary adapter: [`MenuActivate<E>`] → [`Message<E>`].
pub fn republish_menu_activate<E: Message + Copy>(
	activate: On<MenuActivate<E>>,
	mut writer: MessageWriter<E>,
) {
	writer.write(activate.event().choice);
}

pub fn navigate_text_menus(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	mut menus: Query<&mut TextMenu>,
) {
	if !keyboard_nav.is_enabled() || lock.0 {
		return;
	}
	let delta = if keyboard.just_pressed(KeyCode::ArrowDown) {
		1
	} else if keyboard.just_pressed(KeyCode::ArrowUp) {
		-1
	} else {
		return;
	};
	for mut menu in &mut menus {
		menu.step(delta);
	}
}

pub fn sync_text_menu_item_colors(
	menus: Query<&TextMenu>,
	child_of: Query<&ChildOf>,
	items: Query<(Entity, &TextMenuItem, &Children)>,
	mut labels: Query<&mut TextColor, With<TextMenuItemLabel>>,
) {
	for (entity, item, children) in &items {
		let Some(menu) = text_menu_for_item(entity, &child_of, &menus) else {
			continue;
		};
		let color = if item.index == menu.selected { item.active } else { item.idle };
		for child in children {
			if let Ok(mut text_color) = labels.get_mut(*child) {
				text_color.0 = color;
			}
		}
	}
}

fn text_menu_for_item<'a>(
	start: Entity,
	child_of: &Query<&ChildOf>,
	menus: &'a Query<&TextMenu>,
) -> Option<&'a TextMenu> {
	let mut entity = start;
	loop {
		if let Ok(menu) = menus.get(entity) {
			return Some(menu);
		}
		entity = child_of.get(entity).ok()?.parent();
	}
}

#[cfg(test)]
mod tests {
	use super::{TextColumnAlign, TextColumnAnchor, TextMenu};
	use crate::theme::COLUMN_INSET;
	use bevy::prelude::*;
	use bevy::text::Justify;

	#[test]
	fn center_align_centers_items_and_text() {
		assert_eq!(TextColumnAlign::Center.items(), AlignItems::Center);
		assert_eq!(TextColumnAlign::Center.justify_content(), JustifyContent::Center);
		assert_eq!(TextColumnAlign::Center.text_justify(), Justify::Center);
		assert_eq!(TextColumnAlign::Start.items(), AlignItems::FlexStart);
		assert_eq!(TextColumnAlign::Start.text_justify(), Justify::Left);
	}

	#[test]
	fn step_wraps() {
		let mut menu = TextMenu::new(5);
		menu.step(-1);
		assert_eq!(menu.selected, 4);
		menu.step(1);
		assert_eq!(menu.selected, 0);
	}

	#[test]
	fn with_selected_clamps() {
		assert_eq!(TextMenu::with_selected(3, 5).selected, 2);
		assert_eq!(TextMenu::with_selected(0, 2).selected, 0);
	}

	#[test]
	fn top_left_pins_header_to_the_window_corner() {
		let node = TextColumnAnchor::TopLeft.node(TextColumnAlign::Start);
		assert_eq!(node.left, Val::Px(COLUMN_INSET));
		assert_eq!(node.top, Val::Px(COLUMN_INSET));
		assert_eq!(node.bottom, Val::Auto);
	}

	#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
	enum TestChoice {
		#[default]
		Go,
	}

	#[derive(Resource, Default)]
	struct ActivateHits(u32);

	fn count_activate(
		_activate: On<super::MenuActivate<TestChoice>>,
		mut hits: ResMut<ActivateHits>,
	) {
		hits.0 += 1;
	}

	fn activate_app() -> App {
		let mut app = App::new();
		app.init_resource::<ButtonInput<KeyCode>>()
			.init_resource::<super::KeyboardMenuNav>()
			.init_resource::<super::TextMenuInputLock>()
			.init_resource::<ActivateHits>()
			.add_observer(count_activate)
			.add_systems(Update, super::emit_menu_activate_on_enter::<TestChoice>);
		app
	}

	fn spawn_row(app: &mut App, locked: bool) -> Entity {
		let menu = app.world_mut().spawn(super::TextMenu::new(1)).id();
		let mut entity =
			app.world_mut()
				.spawn((super::TextMenuItem::yellow(0), TestChoice::Go, ChildOf(menu)));
		if locked {
			entity.insert(super::MenuItemLocked);
		}
		menu
	}

	#[test]
	fn locked_row_does_not_activate_on_enter() {
		let mut app = activate_app();
		spawn_row(&mut app, true);
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Enter);
		app.update();
		assert_eq!(app.world().resource::<ActivateHits>().0, 0);
	}

	#[test]
	fn unlocked_row_activates_on_enter() {
		let mut app = activate_app();
		spawn_row(&mut app, false);
		app.world_mut().resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Enter);
		app.update();
		assert_eq!(app.world().resource::<ActivateHits>().0, 1);
	}

	#[test]
	fn locked_row_does_not_activate_on_nav_select() {
		let mut app = App::new();
		app.init_resource::<super::TextMenuInputLock>()
			.init_resource::<ActivateHits>()
			.add_observer(count_activate)
			.add_observer(super::emit_menu_activate_on_nav::<TestChoice>);
		let menu = spawn_row(&mut app, true);
		app.world_mut()
			.trigger(maybraid_input::MenuNavImpulse::new(menu, maybraid_input::MenuNav::Select));
		assert_eq!(app.world().resource::<ActivateHits>().0, 0);
	}

	#[test]
	fn stepper_row_keeps_selection_on_left_right() {
		let mut app = App::new();
		app.init_resource::<super::TextMenuInputLock>()
			.add_observer(super::apply_text_menu_nav);
		let menu = app.world_mut().spawn(super::TextMenu::new(2)).id();
		app.world_mut().spawn((
			super::TextMenuItem::yellow(0),
			TestChoice::Go,
			super::MenuStepper,
			ChildOf(menu),
		));
		app.world_mut()
			.spawn((super::TextMenuItem::yellow(1), TestChoice::Go, ChildOf(menu)));
		app.world_mut()
			.trigger(maybraid_input::MenuNavImpulse::new(menu, maybraid_input::MenuNav::Right));
		assert_eq!(app.world().entity(menu).get::<super::TextMenu>().unwrap().selected, 0);
	}
}
