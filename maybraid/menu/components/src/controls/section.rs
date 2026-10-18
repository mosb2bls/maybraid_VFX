//! Overlay-opening section header.

use bevy::prelude::*;

use crate::theme::{PANEL_HEADER_FONT_SIZE, TEXT_YELLOW};

use super::focus::{first_focus_tag, paint_focus_tag, spawn_focus_tag, FocusTag};
use super::hud_menu::{HudMenu, HudMenuItem};
use super::text::spawn_header_line;
use super::HudFonts;

/// Marker on a header that opens an overlay select.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct OverlayHeader;

/// Label key for [`ActiveOverlayKey`] matching.
#[derive(Component, Debug, Clone, Copy)]
pub struct OverlayHeaderKey(pub &'static str);

/// Currently open overlay key; empty means the panel is idle.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ActiveOverlayKey(pub Option<&'static str>);

/// Pickable header. `extra` is typically `OpenSelectKey`.
///
/// The tag starts idle; [`sync_hud_focus_tags`] inverts it when the row is
/// hovered, focused, or its overlay is open.
pub fn spawn_section_header(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	value: Option<&str>,
	justify: JustifyContent,
	color: Color,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			OverlayHeader,
			OverlayHeaderKey(label),
			CursorRow,
			extra,
			Node {
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_focus_tag(row, color, false, |tag| {
				spawn_header_line(tag, fonts, label, value, PANEL_HEADER_FONT_SIZE, color);
			});
		});
}

/// Marker on HUD rows that invert a [`FocusTag`] while live.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct CursorRow;

/// Invert the tag while a cursor row is hovered, focused, or its overlay is open.
pub fn sync_hud_focus_tags(
	active: Res<ActiveOverlayKey>,
	rows: Query<
		(&Children, Option<&HudMenuItem>, Option<&OverlayHeaderKey>, Option<&Interaction>),
		Or<(With<CursorRow>, With<OverlayHeader>)>,
	>,
	menus: Query<&HudMenu>,
	tags: Query<(), With<FocusTag>>,
	tag_idle: Query<&FocusTag>,
	children: Query<&Children>,
	mut backgrounds: Query<&mut BackgroundColor, With<FocusTag>>,
	mut colors: Query<&mut TextColor>,
) {
	for (row_children, item, key, interaction) in &rows {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let overlay = key.is_some_and(|key| active.0 == Some(key.0));
		let hovered = matches!(interaction, Some(Interaction::Hovered | Interaction::Pressed));
		let show = focused || overlay || hovered;
		let Some(tag) = first_focus_tag(row_children, &tags) else {
			continue;
		};
		let idle = tag_idle.get(tag).map(|tag| tag.idle).unwrap_or(TEXT_YELLOW);
		paint_focus_tag(tag, show, false, idle, &children, &mut backgrounds, &mut colors);
	}
}
