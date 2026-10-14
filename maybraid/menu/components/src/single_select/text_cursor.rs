//! Text menu whose active row shows an animated mark in a reserved gutter.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, Justify};

use crate::icons::maybraid::AnimatedIcon;

use super::text_menu::{
	MenuItemLocked, MenuStepper, TextColumnAlign, TextColumnAnchor, TextMenu, TextMenuHeader,
	TextMenuItem, TextMenuItemLabel,
};
use crate::controls::scroll::{entity_is_under, reveal_item_in_viewport};
use crate::controls::section::CursorRow;
use crate::info::description::TextMenuDescription;
use crate::theme::{
	BARLOW_SEMIBOLD, COLUMN_BOTTOM, CORNER_BOTTOM, CORNER_INSET, CURSOR_ICON_GAP, CURSOR_ICON_SIZE,
	DESCRIPTION_FONT_SIZE, DESCRIPTION_PANE_LEFT_PERCENT, ITEM_FONT_SIZE, ITEM_ROW_GAP,
	OBJECTIVE_MARKER_BORDER, OBJECTIVE_MARKER_FONT_SIZE, OBJECTIVE_MARKER_OFFSET_Y,
	OBJECTIVE_MARKER_PAD_X, OBJECTIVE_MARKER_PAD_Y, OBJECTIVE_MARKER_RADIUS, TEXT_LIME,
	TEXT_PURPLE, TEXT_SALMON, TEXT_YELLOW, TEXT_YELLOW_FAINT, TEXT_YELLOW_FAINT_FOCUS,
	TILE_FOCUS_PAD,
};

/// Marker on a text-cursor column. Shares [`TextMenu`] selection with the plain column.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorMenu;

/// Scroll viewport under a sticky [`TextCursorColumn`] header.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorScroll;

/// Reserved gutter on a row; the animated mark is a child.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextCursorSlot;

/// Onboarding / availability badge copy and color. Screens set a kind rather
/// than free-stringing the strings.
#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuObjectiveKind {
	#[default]
	ComingSoon,
	TemporarilyRemoved,
	StartHere,
	NeedsCharacter,
	Selected,
}

impl MenuObjectiveKind {
	pub fn label(self) -> &'static str {
		match self {
			Self::ComingSoon => "Coming Soon",
			Self::TemporarilyRemoved => "Temporarily Removed",
			Self::StartHere => "Start Here",
			Self::NeedsCharacter => "Needs Character.",
			Self::Selected => "Selected",
		}
	}

	pub fn color(self) -> Color {
		match self {
			Self::ComingSoon | Self::TemporarilyRemoved => TEXT_PURPLE,
			Self::StartHere => TEXT_LIME,
			Self::NeedsCharacter => TEXT_SALMON,
			Self::Selected => TEXT_LIME,
		}
	}

	pub fn marker(self) -> MenuObjectiveMarker {
		MenuObjectiveMarker { label: String::from(self.label()), color: self.color() }
	}
}

/// Badge beside a text-cursor title. Lives on the pickable row, not in subtext.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct MenuObjectiveMarker {
	pub label: String,
	pub color: Color,
}

/// One labeled action, optionally with a caption under the title.
pub struct TextCursorRow<E> {
	pub label: String,
	pub subtext: Option<String>,
	pub action: E,
	pub objective: Option<MenuObjectiveKind>,
	/// When `false`, the badge node is still spawned so a screen can show it later.
	pub objective_visible: bool,
	pub locked: bool,
	/// Left/right change this row's value instead of moving the cursor.
	pub stepper: bool,
}

impl<E> TextCursorRow<E> {
	pub fn new(label: impl Into<String>, action: E) -> Self {
		Self {
			label: label.into(),
			subtext: None,
			action,
			objective: None,
			objective_visible: true,
			locked: false,
			stepper: false,
		}
	}

	pub fn with_subtext(mut self, subtext: impl Into<String>) -> Self {
		let subtext = subtext.into();
		self.subtext = (!subtext.is_empty()).then_some(subtext);
		self
	}

	pub fn with_objective(mut self, kind: MenuObjectiveKind) -> Self {
		self.objective = Some(kind);
		self.objective_visible = true;
		self
	}

	/// Same badge as [`Self::with_objective`], hidden until a system reveals it.
	pub fn with_hidden_objective(mut self, kind: MenuObjectiveKind) -> Self {
		self.objective = Some(kind);
		self.objective_visible = false;
		self
	}

	pub fn locked(mut self) -> Self {
		self.locked = true;
		self
	}

	pub fn stepper(mut self) -> Self {
		self.stepper = true;
		self
	}
}

/// Header plus labeled actions, with an animated mark beside the active row.
pub struct TextCursorColumn<E> {
	pub header: Option<String>,
	pub items: Vec<TextCursorRow<E>>,
	pub anchor: TextColumnAnchor,
	pub align: TextColumnAlign,
	pub selected: usize,
	pub description: Option<String>,
	pub scrollable: bool,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> TextCursorColumn<E> {
	pub fn new(
		header: impl Into<String>,
		items: impl IntoIterator<Item = (impl Into<String>, E)>,
	) -> Self {
		Self {
			header: Some(header.into()),
			items: items
				.into_iter()
				.map(|(label, action)| TextCursorRow::new(label, action))
				.collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
			scrollable: false,
		}
	}

	/// Column of actions with no title above the first row.
	pub fn untitled(items: impl IntoIterator<Item = (impl Into<String>, E)>) -> Self {
		Self {
			header: None,
			items: items
				.into_iter()
				.map(|(label, action)| TextCursorRow::new(label, action))
				.collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
			scrollable: false,
		}
	}

	pub fn rows(
		header: impl Into<String>,
		items: impl IntoIterator<Item = TextCursorRow<E>>,
	) -> Self {
		Self { header: Some(header.into()), ..Self::untitled_rows(items) }
	}

	/// [`Self::rows`] with no title above the first row.
	pub fn untitled_rows(items: impl IntoIterator<Item = TextCursorRow<E>>) -> Self {
		Self {
			header: None,
			items: items.into_iter().collect(),
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
			scrollable: false,
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

	pub fn with_selected(mut self, selected: usize) -> Self {
		self.selected = selected;
		self
	}

	pub fn with_description(mut self, description: impl Into<String>) -> Self {
		self.description = Some(description.into());
		self
	}

	/// Pin the column between the top inset and the footer band and scroll
	/// overflow so a long roster stays reachable.
	pub fn scrollable(mut self) -> Self {
		self.scrollable = true;
		self
	}

	pub fn scene(self) -> impl Scene + 'static {
		let item_count = self.items.len();
		let selected = if item_count == 0 { 0 } else { self.selected.min(item_count - 1) };
		let mut children: Vec<Box<dyn Scene>> = Vec::with_capacity(
			item_count
				+ usize::from(self.header.is_some())
				+ usize::from(self.description.is_some()),
		);
		if let Some(header) = self.header {
			children.push(Box::new(TextMenuHeader::new(header).scene()));
		}
		let mut rows: Vec<Box<dyn Scene>> = Vec::with_capacity(item_count);
		for (index, row) in self.items.into_iter().enumerate() {
			let item = if row.locked {
				TextMenuItem::faint_yellow(index)
			} else {
				TextMenuItem::yellow(index)
			};
			rows.push(Box::new(cursor_row_scene(item, row, self.align, selected)));
		}
		let mut node = self.anchor.node(self.align);
		if self.scrollable {
			node.bottom = Val::Px(COLUMN_BOTTOM);
			node.max_width = Val::Percent(DESCRIPTION_PANE_LEFT_PERCENT);
			node.min_height = Val::Px(0.0);
			let scroll_node = Node {
				width: Val::Percent(100.0),
				flex_grow: 1.0,
				flex_shrink: 1.0,
				min_height: Val::Px(0.0),
				flex_direction: FlexDirection::Column,
				align_items: self.align.items(),
				row_gap: Val::Px(ITEM_ROW_GAP),
				overflow: Overflow::scroll_y(),
				..default()
			};
			children.push(Box::new(bsn! {
				TextCursorScroll
				template_value(scroll_node)
				ScrollPosition::default()
				Children [ {rows} ]
			}));
		} else {
			children.extend(rows);
		}
		if let Some(description) = self.description {
			children.push(Box::new(TextMenuDescription::under_column(description)));
		}
		bsn! {
			TextCursorMenu
			template_value(TextMenu::with_selected(item_count, selected))
			template_value(node)
			Children [ {children} ]
		}
	}
}

/// Cursor-marked action plus a separate caption under it (e.g. Next + progress).
///
/// The mark lives on the label row only, same as [`TextCursorColumn`]. Subtext is
/// its own node, indented to line up with the title.
pub struct ButtonWithSubtext<E> {
	pub label: String,
	pub subtext: String,
	pub action: E,
	pub anchor: TextColumnAnchor,
}

impl<E: Component + Copy + Default + Unpin + Send + Sync + 'static> ButtonWithSubtext<E> {
	pub fn new(label: impl Into<String>, subtext: impl Into<String>, action: E) -> Self {
		Self {
			label: label.into(),
			subtext: subtext.into(),
			action,
			anchor: TextColumnAnchor::BottomRight,
		}
	}

	pub fn anchored(mut self, anchor: TextColumnAnchor) -> Self {
		self.anchor = anchor;
		self
	}

	pub fn scene(self) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![
			cursor_item_scene(
				TextMenuItem::yellow(0),
				self.label,
				self.action,
				TextColumnAlign::Start,
				0,
				false,
				false,
				None,
				true,
			),
			Box::new(subtext_caption_scene(self.subtext, TextColumnAlign::Start)),
		];
		let node = self.anchor.node(TextColumnAlign::Start);
		bsn! {
			TextCursorMenu
			template_value(TextMenu::new(1))
			template_value(node)
			Children [ {children} ]
		}
	}
}

/// Lower-left screen chrome. Click (and host `B` / Escape) leave the screen.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ScreenBack;

/// One-shot leave request from [`ScreenBack`].
#[derive(Message, Debug, Default, Clone, Copy)]
pub struct ScreenBackPressed;

/// Bottom-left Back control. Not part of the screen's [`TextMenu`] so Enter
/// on the main list cannot fire it.
pub fn screen_back_scene() -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(cursor_slot_scene(Visibility::Hidden, TextColumnAlign::Start, TEXT_YELLOW)),
		Box::new(cursor_label_scene(String::from("Back"), TextColumnAlign::Start, TEXT_YELLOW)),
	];
	bsn! {
		Button
		ScreenBack
		CursorRow
		Node {
			position_type: PositionType::Absolute,
			left: px(CORNER_INSET),
			bottom: px(CORNER_BOTTOM),
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: px(CURSOR_ICON_GAP),
			padding: UiRect::axes(px(0.0), px(2.0)),
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

pub fn emit_screen_back_on_click(
	click: On<Pointer<Click>>,
	backs: Query<(), With<ScreenBack>>,
	mut pressed: MessageWriter<ScreenBackPressed>,
) {
	if backs.contains(click.entity) {
		pressed.write(ScreenBackPressed);
	}
}

/// Lower-right screen chrome. Click opens the active character in the editor.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ScreenEdit;

#[derive(Message, Debug, Default, Clone, Copy)]
pub struct ScreenEditPressed;

pub fn screen_edit_scene() -> impl Scene + 'static {
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(cursor_slot_scene(Visibility::Hidden, TextColumnAlign::Start, TEXT_YELLOW)),
		Box::new(cursor_label_scene(String::from("Edit"), TextColumnAlign::Start, TEXT_YELLOW)),
	];
	bsn! {
		Button
		ScreenEdit
		CursorRow
		Node {
			position_type: PositionType::Absolute,
			right: px(CORNER_INSET),
			bottom: px(CORNER_BOTTOM),
			flex_direction: FlexDirection::Row,
			align_items: AlignItems::Center,
			column_gap: px(CURSOR_ICON_GAP),
			padding: UiRect::axes(px(0.0), px(2.0)),
		}
		BackgroundColor(Color::NONE)
		Children [ {children} ]
	}
}

pub fn emit_screen_edit_on_click(
	click: On<Pointer<Click>>,
	edits: Query<(), With<ScreenEdit>>,
	mut pressed: MessageWriter<ScreenEditPressed>,
) {
	if edits.contains(click.entity) {
		pressed.write(ScreenEditPressed);
	}
}

fn cursor_row_scene<E>(
	item: TextMenuItem,
	row: TextCursorRow<E>,
	align: TextColumnAlign,
	selected: usize,
) -> impl Scene + 'static
where
	E: Component + Copy + Default + Unpin + Send + Sync + 'static,
{
	let mut children: Vec<Box<dyn Scene>> = vec![cursor_item_scene(
		item,
		row.label,
		row.action,
		align,
		selected,
		row.locked,
		row.stepper,
		row.objective,
		row.objective_visible,
	)];
	if let Some(subtext) = row.subtext {
		children.push(Box::new(subtext_caption_scene(subtext, align)));
	}
	let align_items = align.items();
	bsn! {
		Node {
			flex_direction: FlexDirection::Column,
			align_items: align_items,
		}
		Pickable::IGNORE
		Children [ {children} ]
	}
}

/// Under a Start title the caption skips the cursor gutter; a centered title
/// hangs its gutter outside the row, so the caption centers with no indent.
fn subtext_caption_scene(subtext: String, align: TextColumnAlign) -> impl Scene + 'static {
	let margin = match align {
		TextColumnAlign::Start => {
			UiRect { left: Val::Px(CURSOR_ICON_SIZE + CURSOR_ICON_GAP), ..default() }
		}
		TextColumnAlign::Center => UiRect::default(),
	};
	let justify = align.text_justify();
	bsn! {
		Node {
			margin: margin,
		}
		Pickable::IGNORE
		Children [(
			template_value(Text::new(subtext))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
				font_size: px(DESCRIPTION_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW_FAINT)
			TextLayout::new(justify, bevy::text::LineBreak::NoWrap)
			Pickable::IGNORE
		)]
	}
}

fn cursor_item_scene<E>(
	item: TextMenuItem,
	label: String,
	action: E,
	align: TextColumnAlign,
	selected: usize,
	locked: bool,
	stepper: bool,
	objective: Option<MenuObjectiveKind>,
	objective_visible: bool,
) -> Box<dyn Scene>
where
	E: Component + Copy + Default + Unpin + Send + Sync + 'static,
{
	let visibility =
		if item.index == selected { Visibility::Inherited } else { Visibility::Hidden };
	let icon_color = if locked { TEXT_YELLOW_FAINT_FOCUS } else { TEXT_YELLOW };
	let mut children: Vec<Box<dyn Scene>> = vec![
		Box::new(cursor_slot_scene(visibility, align, icon_color)),
		Box::new(cursor_label_scene(label, align, item.idle)),
	];
	if let Some(kind) = objective {
		let marker_visibility =
			if objective_visible { Visibility::Inherited } else { Visibility::Hidden };
		children.push(Box::new(objective_marker_scene(kind, marker_visibility)));
	}
	let column_gap = match align {
		TextColumnAlign::Start => Val::Px(CURSOR_ICON_GAP),
		TextColumnAlign::Center => Val::Px(0.0),
	};
	let justify_content = align.justify_content();
	if locked {
		Box::new(bsn! {
			Button
			template_value(item)
			template_value(action)
			MenuItemLocked
			Node {
				padding: UiRect::axes(px(0.0), px(2.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify_content,
				align_items: AlignItems::Center,
				column_gap: column_gap,
			}
			BackgroundColor(Color::NONE)
			Children [ {children} ]
		})
	} else if stepper {
		Box::new(bsn! {
			Button
			template_value(item)
			template_value(action)
			MenuStepper
			Node {
				padding: UiRect::axes(px(0.0), px(2.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify_content,
				align_items: AlignItems::Center,
				column_gap: column_gap,
			}
			BackgroundColor(Color::NONE)
			Children [ {children} ]
		})
	} else {
		Box::new(bsn! {
			Button
			template_value(item)
			template_value(action)
			Node {
				padding: UiRect::axes(px(0.0), px(2.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify_content,
				align_items: AlignItems::Center,
				column_gap: column_gap,
			}
			BackgroundColor(Color::NONE)
			Children [ {children} ]
		})
	}
}

fn objective_marker_scene(kind: MenuObjectiveKind, visibility: Visibility) -> impl Scene + 'static {
	let marker = kind.marker();
	let label = marker.label.clone();
	let color = marker.color;
	let fill = color.with_alpha(0.14);
	let children: Vec<Box<dyn Scene>> = vec![Box::new(bsn! {
		template_value(Text::new(label))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
			font_size: px(OBJECTIVE_MARKER_FONT_SIZE),
		}
		TextColor(color)
		TextLayout::new(Justify::Center, bevy::text::LineBreak::NoWrap)
		Pickable::IGNORE
	})];
	bsn! {
		template_value(kind)
		template_value(marker)
		template_value(visibility)
		Node {
			padding: UiRect::axes(px(OBJECTIVE_MARKER_PAD_X), px(OBJECTIVE_MARKER_PAD_Y)),
			border: UiRect::all(px(OBJECTIVE_MARKER_BORDER)),
			border_radius: BorderRadius::all(px(OBJECTIVE_MARKER_RADIUS)),
			margin: UiRect::top(px(OBJECTIVE_MARKER_OFFSET_Y)),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			flex_shrink: 0.0,
		}
		BorderColor::all(color)
		BackgroundColor(fill)
		Pickable::IGNORE
		Children [ {children} ]
	}
}

fn cursor_slot_scene(
	visibility: Visibility,
	align: TextColumnAlign,
	icon_color: Color,
) -> impl Scene {
	let children: Vec<Box<dyn Scene>> = vec![Box::new(
		AnimatedIcon::maybraid_scene_with_visibility(CURSOR_ICON_SIZE, icon_color, visibility),
	)];
	let node = match align {
		TextColumnAlign::Start => Node {
			width: Val::Px(CURSOR_ICON_SIZE),
			height: Val::Px(CURSOR_ICON_SIZE),
			flex_shrink: 0.0,
			..default()
		},
		TextColumnAlign::Center => Node {
			position_type: PositionType::Absolute,
			left: Val::Px(-(CURSOR_ICON_SIZE + CURSOR_ICON_GAP)),
			top: Val::Px(0.0),
			bottom: Val::Px(0.0),
			width: Val::Px(CURSOR_ICON_SIZE),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			flex_shrink: 0.0,
			..default()
		},
	};
	bsn! {
		TextCursorSlot
		template_value(node)
		Pickable::IGNORE
		Children [ {children} ]
	}
}

fn cursor_label_scene(label: String, align: TextColumnAlign, color: Color) -> impl Scene {
	let text_justify = align.text_justify();
	bsn! {
		template_value(Text::new(label))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
			font_size: px(ITEM_FONT_SIZE),
		}
		TextColor(color)
		TextLayout::new(text_justify, bevy::text::LineBreak::NoWrap)
		TextMenuItemLabel
		Pickable::IGNORE
	}
}

/// Show the animated mark only in the selected row’s gutter.
///
/// A visible [`MenuObjectiveKind::Selected`] chip moves the mark to [`ScreenEdit`].
pub fn sync_text_cursor_icons(
	menus: Query<&TextMenu, With<TextCursorMenu>>,
	items: Query<(Entity, &TextMenuItem)>,
	child_of: Query<&ChildOf>,
	children: Query<&Children>,
	kinds: Query<&MenuObjectiveKind>,
	markers: Query<&Visibility, (With<MenuObjectiveMarker>, Without<AnimatedIcon>)>,
	slots: Query<(), With<TextCursorSlot>>,
	mut icons: Query<&mut Visibility, (With<AnimatedIcon>, Without<MenuObjectiveMarker>)>,
) {
	for (item_entity, item) in &items {
		let Some(menu) = text_cursor_menu(item_entity, &child_of, &menus) else {
			continue;
		};
		let edit_cue = descendant_shows_kind(
			item_entity,
			MenuObjectiveKind::Selected,
			&kinds,
			&markers,
			&children,
		);
		let show = item.index == menu.selected && !edit_cue;
		let Ok(item_children) = children.get(item_entity) else {
			continue;
		};
		set_slot_icon_visibility(item_children, show, &slots, &children, &mut icons);
	}
}

/// Point the Maybraid mark at Edit when the active character row is live.
pub fn sync_screen_edit_cursor(
	menus: Query<&TextMenu, With<TextCursorMenu>>,
	items: Query<(Entity, &TextMenuItem, Option<&Interaction>)>,
	edits: Query<&Children, With<ScreenEdit>>,
	kinds: Query<&MenuObjectiveKind>,
	markers: Query<&Visibility, (With<MenuObjectiveMarker>, Without<AnimatedIcon>)>,
	slots: Query<(), With<TextCursorSlot>>,
	children: Query<&Children>,
	mut icons: Query<&mut Visibility, (With<AnimatedIcon>, Without<MenuObjectiveMarker>)>,
) {
	let cue = items.iter().any(|(entity, item, interaction)| {
		if !descendant_shows_kind(entity, MenuObjectiveKind::Selected, &kinds, &markers, &children)
		{
			return false;
		}
		let focused = menus.iter().any(|menu| menu.selected == item.index);
		let hovered = matches!(interaction, Some(Interaction::Hovered | Interaction::Pressed));
		focused || hovered
	});
	if !cue {
		return;
	}
	for row_children in &edits {
		set_slot_icon_visibility(row_children, true, &slots, &children, &mut icons);
	}
}

/// Follow the selected roster row so a long gallery stays on-screen.
pub fn scroll_text_cursor_selection_into_view(
	menus: Query<(&TextMenu, &Children), With<TextCursorMenu>>,
	mut scrolls: Query<(Entity, &ComputedNode, &mut ScrollPosition), With<TextCursorScroll>>,
	items: Query<(Entity, &TextMenuItem, &ComputedNode, &bevy::ui::UiGlobalTransform)>,
	transforms: Query<&bevy::ui::UiGlobalTransform>,
	child_of: Query<&ChildOf>,
) {
	for (menu, menu_children) in &menus {
		if menu.item_count == 0 {
			continue;
		}
		let Some(scroll_entity) = menu_children.iter().find(|child| scrolls.contains(*child))
		else {
			continue;
		};
		let Ok((_, viewport, mut scroll)) = scrolls.get_mut(scroll_entity) else {
			continue;
		};
		let Ok(view_tf) = transforms.get(scroll_entity) else {
			continue;
		};
		let Some((_, _, item_node, item_tf)) = items.iter().find(|(entity, item, _, _)| {
			item.index == menu.selected && entity_is_under(*entity, scroll_entity, &child_of)
		}) else {
			continue;
		};
		reveal_item_in_viewport(viewport, view_tf, item_node, item_tf, TILE_FOCUS_PAD, &mut scroll);
	}
}

fn set_slot_icon_visibility(
	row_children: &Children,
	show: bool,
	slots: &Query<(), With<TextCursorSlot>>,
	children: &Query<&Children>,
	icons: &mut Query<&mut Visibility, (With<AnimatedIcon>, Without<MenuObjectiveMarker>)>,
) {
	for child in row_children {
		if slots.get(*child).is_err() {
			continue;
		}
		let Ok(slot_children) = children.get(*child) else {
			continue;
		};
		for icon_entity in slot_children {
			if let Ok(mut visibility) = icons.get_mut(*icon_entity) {
				*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
			}
		}
	}
}

/// Visible badge of `kind` under `root`. Hidden Selected chips do not count, so
/// the Edit mark stays on the active character only.
fn descendant_shows_kind(
	root: Entity,
	kind: MenuObjectiveKind,
	kinds: &Query<&MenuObjectiveKind>,
	markers: &Query<&Visibility, (With<MenuObjectiveMarker>, Without<AnimatedIcon>)>,
	children: &Query<&Children>,
) -> bool {
	if kinds.get(root).is_ok_and(|found| *found == kind) {
		return !matches!(markers.get(root), Ok(Visibility::Hidden));
	}
	let Ok(kids) = children.get(root) else {
		return false;
	};
	kids.iter()
		.any(|child| descendant_shows_kind(child, kind, kinds, markers, children))
}

fn text_cursor_menu<'a>(
	start: Entity,
	child_of: &Query<&ChildOf>,
	menus: &'a Query<&TextMenu, With<TextCursorMenu>>,
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
	use super::{MenuObjectiveKind, TextCursorColumn, TextCursorRow};
	use crate::single_select::{TextColumnAlign, TextColumnAnchor};
	use crate::theme::{OBJECTIVE_MARKER_FONT_SIZE, TEXT_LIME, TEXT_PURPLE, TEXT_SALMON};
	use crate::ITEM_FONT_SIZE;

	#[derive(Clone, Copy)]
	enum RowAction {
		Go,
	}

	#[test]
	fn objective_kind_copy_and_color() {
		assert_eq!(MenuObjectiveKind::ComingSoon.label(), "Coming Soon");
		assert_eq!(MenuObjectiveKind::TemporarilyRemoved.label(), "Temporarily Removed");
		assert_eq!(MenuObjectiveKind::StartHere.label(), "Start Here");
		assert_eq!(MenuObjectiveKind::NeedsCharacter.label(), "Needs Character.");
		assert_eq!(MenuObjectiveKind::Selected.label(), "Selected");
		assert_eq!(MenuObjectiveKind::ComingSoon.color(), TEXT_PURPLE);
		assert_eq!(MenuObjectiveKind::TemporarilyRemoved.color(), TEXT_PURPLE);
		assert_eq!(MenuObjectiveKind::StartHere.color(), TEXT_LIME);
		assert_eq!(MenuObjectiveKind::NeedsCharacter.color(), TEXT_SALMON);
		assert_eq!(MenuObjectiveKind::Selected.color(), TEXT_LIME);
	}

	#[test]
	fn hidden_objective_keeps_the_badge_kind() {
		let row = TextCursorRow::new("Jeff", RowAction::Go)
			.with_hidden_objective(MenuObjectiveKind::Selected);
		assert_eq!(row.objective, Some(MenuObjectiveKind::Selected));
		assert!(!row.objective_visible);
	}

	#[test]
	fn marker_is_beside_the_title_not_subtext() {
		let row = TextCursorRow::new("Reliquary", RowAction::Go)
			.with_objective(MenuObjectiveKind::ComingSoon)
			.locked();
		assert_eq!(row.objective, Some(MenuObjectiveKind::ComingSoon));
		assert!(row.locked);
		assert!(row.subtext.is_none());
		assert!(OBJECTIVE_MARKER_FONT_SIZE < ITEM_FONT_SIZE);
	}

	#[test]
	fn subtext_stays_independent_of_the_objective() {
		let row = TextCursorRow::new("Jeff", RowAction::Go)
			.with_subtext("Braidman")
			.with_objective(MenuObjectiveKind::StartHere);
		assert_eq!(row.subtext.as_deref(), Some("Braidman"));
		assert_eq!(row.objective, Some(MenuObjectiveKind::StartHere));
		assert!(!row.locked);
		assert!(!row.stepper);
	}

	#[test]
	fn stepper_marks_the_row() {
		let row = TextCursorRow::new("Render Scale", RowAction::Go).stepper();
		assert!(row.stepper);
	}

	#[test]
	fn scrollable_roster_keeps_a_sticky_header() {
		let column = TextCursorColumn {
			header: Some(String::from("Characters")),
			items: vec![TextCursorRow::new("Ada", RowAction::Go)],
			anchor: TextColumnAnchor::TopLeft,
			align: TextColumnAlign::Start,
			selected: 0,
			description: None,
			scrollable: true,
		};
		assert!(column.scrollable);
		assert_eq!(column.header.as_deref(), Some("Characters"));
	}
}
