//! Labels, body copy, and the reserved cursor gutter.

use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, LineHeight, TextSpan};

use crate::theme::{
	PANEL_BLOCK_FONT_SIZE, PANEL_GROUP_FONT_SIZE, PANEL_HEADER_FONT_SIZE, PANEL_ITEM_FONT_SIZE,
	TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

use super::display::menu_display_name;
use super::HudFonts;

/// Largest panel title: section chrome and overlay picker headers.
pub fn spawn_panel_title(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, label: &str) {
	spawn_header_line(parent, fonts, label, None, PANEL_HEADER_FONT_SIZE, TEXT_YELLOW);
}

/// Header title plus an optional trailing value in one text run.
///
/// Sibling `Text` nodes do not share a glyph baseline in Bevy UI; mixed
/// sizes have to live in one text block via `TextSpan`. The value uses the
/// same size as the title so the line reads as one face.
pub fn spawn_header_line(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	title: &str,
	value: Option<&str>,
	title_size: f32,
	title_color: Color,
) {
	parent
		.spawn((
			Text::new(menu_display_name(title)),
			fonts.header(title_size),
			TextColor(title_color),
			TextLayout::new(Justify::Left, LineBreak::NoWrap),
			LineHeight::RelativeToFont(1.0),
			Pickable::IGNORE,
		))
		.with_children(|text| {
			if let Some(value) = value.filter(|value| !value.is_empty()) {
				text.spawn((
					TextSpan::new(format!("  {}", menu_display_name(value))),
					fonts.body(title_size),
					TextColor(TEXT_YELLOW_FAINT),
					LineHeight::RelativeToFont(1.0),
				));
			}
		});
}

/// Block title above a field group (`Eyes`, `Hair`).
pub fn spawn_block_label(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, label: &str) {
	spawn_hud_text(parent, fonts.header(PANEL_BLOCK_FONT_SIZE), label, TEXT_YELLOW, Justify::Left);
}

/// Muted subheading above a grouped select list.
pub fn spawn_group_label(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, label: &str) {
	spawn_hud_text(
		parent,
		fonts.body(PANEL_GROUP_FONT_SIZE),
		label,
		TEXT_YELLOW_FAINT,
		Justify::Left,
	);
}

pub fn spawn_hud_text(
	parent: &mut ChildSpawnerCommands,
	font: TextFont,
	value: &str,
	color: Color,
	justify: Justify,
) {
	parent.spawn((
		Text::new(menu_display_name(value)),
		font,
		TextColor(color),
		TextLayout::new(justify, bevy::text::LineBreak::WordBoundary),
		LineHeight::RelativeToFont(1.0),
		Pickable::IGNORE,
	));
}

/// Numeric / signed values that must keep their original spelling (`+8`, `DPC`).
pub fn spawn_hud_plain(
	parent: &mut ChildSpawnerCommands,
	font: TextFont,
	value: &str,
	color: Color,
	justify: Justify,
) {
	parent.spawn((
		Text::new(value),
		font,
		TextColor(color),
		TextLayout::new(justify, bevy::text::LineBreak::WordBoundary),
		LineHeight::RelativeToFont(1.0),
		Pickable::IGNORE,
	));
}

pub fn spawn_item_label(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	color: Color,
) {
	spawn_hud_text(parent, fonts.item(PANEL_ITEM_FONT_SIZE), label, color, Justify::Left);
}
