use bevy::prelude::*;
use bevy::text::{LineBreak, TextBounds};

pub const BUTTON_HEIGHT: f32 = 22.0;
pub const SELECT_TILE_SIZE: f32 = 52.0;
pub const SWATCH_SIZE: f32 = 20.0;
pub const MENU_VERTICAL_GAP: f32 = 6.0;
pub const MENU_CHIP_GAP: f32 = 6.0;
pub const MENU_BUTTON_PADDING_V: f32 = 4.0;
pub const ACTIVE: Color = Color::srgba(0.16, 0.34, 0.50, 0.95);
pub const INACTIVE: Color = Color::srgba(0.18, 0.20, 0.24, 0.92);
pub const MUTED: Color = Color::srgba(0.72, 0.78, 0.86, 1.0);

const TILE_LABEL_MAX_CHARS: usize = 16;
const TILE_TEXT_INSET: f32 = 8.0;

#[derive(Component, Clone, Copy, Debug)]
pub struct MenuButton<E: Copy + Send + Sync + 'static>(pub E);

#[derive(Component, Clone, Copy, Debug)]
pub struct ToggleSectionKey(pub &'static str);

pub fn render_button<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	event: E,
	active: bool,
) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(28.0),
				height: Val::Px(BUTTON_HEIGHT),
				padding: UiRect::axes(Val::Px(7.0), Val::Px(MENU_BUTTON_PADDING_V)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(if active { ACTIVE } else { INACTIVE }),
			MenuButton(event),
		))
		.with_children(|button| text(button, label, 10.0, Color::WHITE));
}

pub fn render_asset_button<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	event: E,
	active: bool,
	thumbnail: Option<Handle<Image>>,
) {
	parent
		.spawn((
			Button,
			Node {
				min_width: Val::Px(72.0),
				min_height: Val::Px(54.0),
				padding: UiRect::axes(Val::Px(5.0), Val::Px(6.0)),
				flex_direction: FlexDirection::Column,
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				row_gap: Val::Px(MENU_VERTICAL_GAP),
				..default()
			},
			BackgroundColor(if active { ACTIVE } else { INACTIVE }),
			MenuButton(event),
		))
		.with_children(|button| {
			if let Some(thumbnail) = thumbnail {
				button.spawn((
					ImageNode::new(thumbnail),
					Node { width: Val::Px(54.0), height: Val::Px(54.0), ..default() },
					Pickable::IGNORE,
				));
			}
			text(button, label, 9.0, Color::WHITE);
		});
}

pub fn text(parent: &mut ChildSpawnerCommands, value: &str, size: f32, color: Color) {
	parent.spawn((
		Text::new(value.to_string()),
		TextFont { font_size: FontSize::Px(size), ..default() },
		TextColor(color),
		Pickable::IGNORE,
	));
}

/// Short labels wrap inside the tile; very long labels are elided.
pub fn tile_label(label: &str) -> String {
	if label.chars().count() <= TILE_LABEL_MAX_CHARS {
		return label.to_string();
	}
	let mut end = TILE_LABEL_MAX_CHARS.saturating_sub(1);
	while end > 0 && !label.is_char_boundary(end) {
		end -= 1;
	}
	format!("{}…", &label[..end])
}

pub fn tile_text(parent: &mut ChildSpawnerCommands, label: &str, size: f32, color: Color) {
	let bounds = (SELECT_TILE_SIZE - TILE_TEXT_INSET).max(12.0);
	parent.spawn((
		Text::new(tile_label(label)),
		TextFont { font_size: FontSize::Px(size), ..default() },
		TextColor(color),
		TextLayout::new(Justify::Center, LineBreak::WordBoundary),
		TextBounds::new(bounds, bounds),
		Pickable::IGNORE,
	));
}

pub fn select_tile_node() -> Node {
	Node {
		width: Val::Px(SELECT_TILE_SIZE),
		height: Val::Px(SELECT_TILE_SIZE),
		flex_shrink: 0.,
		flex_grow: 0.,
		padding: UiRect::all(Val::Px(6.0)),
		flex_direction: FlexDirection::Column,
		justify_content: JustifyContent::Center,
		align_items: AlignItems::Center,
		overflow: Overflow::clip(),
		..default()
	}
}

/// Left-aligned row for labels beside a control group.
pub fn labeled_row() -> Node {
	Node {
		width: Val::Percent(100.0),
		flex_direction: FlexDirection::Row,
		column_gap: Val::Px(8.0),
		row_gap: Val::Px(MENU_VERTICAL_GAP),
		align_items: AlignItems::FlexStart,
		justify_content: JustifyContent::FlexStart,
		..default()
	}
}

/// Left-aligned wrapping row for swatches, species tiles, and similar chips.
pub fn inline_chip_row() -> Node {
	Node {
		flex_direction: FlexDirection::Row,
		flex_wrap: FlexWrap::Wrap,
		column_gap: Val::Px(MENU_CHIP_GAP),
		row_gap: Val::Px(MENU_CHIP_GAP),
		align_items: AlignItems::FlexStart,
		justify_content: JustifyContent::FlexStart,
		..default()
	}
}

pub fn swatch_node(selected: bool) -> Node {
	Node {
		width: Val::Px(SWATCH_SIZE),
		height: Val::Px(SWATCH_SIZE),
		flex_shrink: 0.,
		flex_grow: 0.,
		border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
		..default()
	}
}

/// Compact horizontal control strip (`<` value `>`, `-` value `+`).
pub fn compact_control_row() -> Node {
	Node {
		flex_direction: FlexDirection::Row,
		column_gap: Val::Px(4.0),
		align_items: AlignItems::Center,
		justify_content: JustifyContent::FlexStart,
		..default()
	}
}

pub fn color_from_hex(hex: &str) -> Color {
	let hex = hex.strip_prefix('#').unwrap_or(hex);
	if hex.len() != 6 {
		return INACTIVE;
	}
	let Ok(red) = u8::from_str_radix(&hex[0..2], 16) else {
		return INACTIVE;
	};
	let Ok(green) = u8::from_str_radix(&hex[2..4], 16) else {
		return INACTIVE;
	};
	let Ok(blue) = u8::from_str_radix(&hex[4..6], 16) else {
		return INACTIVE;
	};
	Color::srgb(red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0)
}
