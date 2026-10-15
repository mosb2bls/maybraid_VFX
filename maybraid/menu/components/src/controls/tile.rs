//! Asset tile: optional thumbnail plus a yellow label.

use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, LineHeight, TextBounds, TextSpan};
use bevy::ui::widget::ViewportNode;

use crate::theme::{
	MENU_CLEAR, OBJECTIVE_MARKER_BORDER, OBJECTIVE_MARKER_RADIUS, PANEL_CHIP_GAP,
	PANEL_GROUP_FONT_SIZE, PANEL_ITEM_FONT_SIZE, PANEL_TILE_COLUMNS, PANEL_TILE_MIN_HEIGHT,
	TEXT_LIME, TEXT_SALMON, TEXT_YELLOW, TEXT_YELLOW_FAINT, TEXT_YELLOW_HOVER, TILE_FOCUS_PAD,
};

use super::display::menu_display_name;
use super::hud_menu::{HudMenu, HudMenuItem};
use super::HudFonts;

const TILE_PAD_X: f32 = 8.0;
const TILE_PAD_Y: f32 = 6.0;
const TILE_BORDER: f32 = 2.0;

/// Title-case the catalog id and keep the full name so the tile can wrap it.
pub fn tile_caption(label: &str) -> String {
	menu_display_name(label)
}

fn tile_grid_columns() -> Vec<RepeatedGridTrack> {
	RepeatedGridTrack::flex(PANEL_TILE_COLUMNS, 1.0)
}

fn tile_node() -> Node {
	Node {
		width: Val::Percent(100.0),
		min_width: Val::Px(0.0),
		min_height: Val::Px(PANEL_TILE_MIN_HEIGHT),
		padding: UiRect::axes(Val::Px(TILE_PAD_X), Val::Px(TILE_PAD_Y)),
		flex_direction: FlexDirection::Column,
		justify_content: JustifyContent::Center,
		align_items: AlignItems::Center,
		row_gap: Val::Px(PANEL_CHIP_GAP),
		border: UiRect::all(Val::Px(TILE_BORDER)),
		overflow: Overflow::clip_x(),
		..default()
	}
}

/// Caption or detail line that reflows to the live cell width.
#[derive(Component)]
pub struct TileWrap;

fn tile_caption_text(caption: impl Into<String>, face: Color, fonts: &HudFonts) -> impl Bundle {
	(
		TileWrap,
		Text::new(caption.into()),
		fonts.item(PANEL_ITEM_FONT_SIZE),
		TextColor(face),
		TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
		TextBounds::UNBOUNDED,
		Node { width: Val::Percent(100.0), ..default() },
		Pickable::IGNORE,
	)
}

/// Wrap tile copy to the laid-out cell instead of a fixed pixel width.
pub fn sync_tile_wrap(mut wraps: Query<(&ComputedNode, &mut TextBounds), With<TileWrap>>) {
	for (computed, mut bounds) in &mut wraps {
		let width = (computed.size().x * computed.inverse_scale_factor()).max(12.0);
		if bounds.width != Some(width) {
			*bounds = TextBounds::new_horizontal(width);
		}
	}
}

/// Pickable asset cell. `extra` is typically `MenuButton<E>`.
pub fn spawn_asset_tile(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	selected: bool,
	thumbnail: Option<Handle<Image>>,
	muted: bool,
	extra: impl Bundle,
) {
	let face = tile_face(selected, muted);
	parent
		.spawn((
			Button,
			HoverTile { equipped: selected, preserve_fill: false },
			extra,
			tile_node(),
			BorderColor::all(if selected { face } else { Color::NONE }),
			BackgroundColor(Color::NONE),
			Outline::new(Val::Px(2.0), Val::Px(2.0), Color::NONE),
		))
		.with_children(|button| {
			if let Some(thumbnail) = thumbnail {
				button.spawn((
					ImageNode::new(thumbnail),
					Node { width: Val::Px(48.0), height: Val::Px(48.0), ..default() },
					Pickable::IGNORE,
				));
			}
			button.spawn(tile_caption_text(tile_caption(label), face, fonts));
		});
}

/// 1-based rank in a typed inventory slot. Derived from bag selection order.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SlotRank(pub u8);

/// Pickable catalog cell. Clothing selection uses the Maybraid son; weapons
/// selection uses the 1-based queue rank. `detail` is a compact stat line.
pub fn spawn_grid_catalog_tile(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	detail: &str,
	selected: bool,
	rank: Option<u8>,
	thumbnail: Option<Handle<Image>>,
	thumbnail_tint: Color,
	viewport: Option<Entity>,
	swatch: Option<Color>,
	muted: bool,
	extra: impl Bundle,
) {
	let face = tile_face(selected, muted);
	let mark = if muted { TEXT_YELLOW_FAINT } else { TEXT_YELLOW };
	let pictured = thumbnail.is_some() || viewport.is_some();
	const THUMB_PX: f32 = 72.0;
	let mut tile = parent.spawn((
		Button,
		HoverTile { equipped: selected, preserve_fill: false },
		extra,
		tile_node(),
		BorderColor::all(Color::NONE),
		BackgroundColor(Color::NONE),
		Outline::new(Val::Px(2.0), Val::Px(2.0), Color::NONE),
	));
	if let Some(rank) = rank {
		tile.insert(SlotRank(rank));
	}
	tile.with_children(|button| {
		button
			.spawn((
				Node {
					width: if pictured { Val::Percent(100.0) } else { Val::Px(48.0) },
					height: if pictured { Val::Auto } else { Val::Px(48.0) },
					aspect_ratio: pictured.then_some(1.0),
					min_height: Val::Px(if pictured { THUMB_PX } else { 48.0 }),
					position_type: PositionType::Relative,
					justify_content: JustifyContent::Center,
					align_items: AlignItems::Center,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|slot| {
				if !pictured {
					if let Some(fill) = swatch {
						let accent = Color::srgb(
							(fill.to_srgba().red * 0.35 + 0.65).min(1.0),
							(fill.to_srgba().green * 0.35 + 0.65).min(1.0),
							(fill.to_srgba().blue * 0.35 + 0.65).min(1.0),
						);
						slot.spawn((
							Node {
								position_type: PositionType::Absolute,
								width: Val::Px(48.0),
								height: Val::Px(48.0),
								justify_content: JustifyContent::Center,
								align_items: AlignItems::Center,
								border_radius: BorderRadius::all(Val::Px(6.0)),
								..default()
							},
							BackgroundColor(fill),
							Pickable::IGNORE,
						))
						.with_children(|plate| {
							plate.spawn((
								Node {
									width: Val::Px(22.0),
									height: Val::Px(22.0),
									border_radius: BorderRadius::all(Val::Px(3.0)),
									..default()
								},
								BackgroundColor(accent),
								Pickable::IGNORE,
							));
						});
					}
				}
				if let Some(camera) = viewport {
					slot.spawn((
						ViewportNode::new(camera),
						Node {
							width: Val::Percent(100.0),
							height: Val::Percent(100.0),
							..default()
						},
						Pickable::IGNORE,
					));
				} else if let Some(thumbnail) = thumbnail {
					slot.spawn((
						ImageNode { image: thumbnail, color: thumbnail_tint, ..default() },
						Node { width: Val::Percent(100.0), aspect_ratio: Some(1.0), ..default() },
						Pickable::IGNORE,
					));
				}
				if let Some(rank) = rank {
					spawn_rank_tag(slot, fonts, rank, mark);
				} else if selected && !pictured {
					// Clothing without a thumbnail uses the Maybraid son.
					// A pictured tile is just the icon — no select overlay.
					crate::icons::Icon::maybraid(22.0, mark).spawn(
						slot,
						fonts.logo.clone(),
						Visibility::Inherited,
					);
				}
			});
		button.spawn(tile_caption_text(tile_caption(label), face, fonts));
		if !detail.is_empty() {
			spawn_detail_line(button, fonts, detail);
		}
	});
}

fn spawn_rank_tag(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, rank: u8, face: Color) {
	parent
		.spawn((
			Node {
				position_type: PositionType::Absolute,
				top: Val::Px(2.0),
				left: Val::Px(2.0),
				padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
				border: UiRect::all(Val::Px(OBJECTIVE_MARKER_BORDER)),
				border_radius: BorderRadius::all(Val::Px(OBJECTIVE_MARKER_RADIUS)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(MENU_CLEAR),
			BorderColor::all(face),
			ZIndex(1),
			Pickable::IGNORE,
		))
		.with_children(|tag| {
			tag.spawn((
				Text::new(rank.to_string()),
				fonts.item(PANEL_ITEM_FONT_SIZE),
				TextColor(face),
				Pickable::IGNORE,
			));
		});
}

fn spawn_detail_line(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, detail: &str) {
	let mut parts = detail.split(" · ");
	let Some(first) = parts.next() else {
		return;
	};
	parent
		.spawn((
			TileWrap,
			Text::new(first),
			fonts.item(PANEL_GROUP_FONT_SIZE),
			TextColor(detail_chip_color(first)),
			TextLayout::new(Justify::Center, LineBreak::WordOrCharacter),
			TextBounds::UNBOUNDED,
			LineHeight::RelativeToFont(1.0),
			Node { width: Val::Percent(100.0), ..default() },
			Pickable::IGNORE,
		))
		.with_children(|text| {
			for part in parts {
				text.spawn((
					TextSpan::new(" · "),
					fonts.item(PANEL_GROUP_FONT_SIZE),
					TextColor(TEXT_YELLOW_FAINT),
					LineHeight::RelativeToFont(1.0),
				));
				text.spawn((
					TextSpan::new(part),
					fonts.item(PANEL_GROUP_FONT_SIZE),
					TextColor(detail_chip_color(part)),
					LineHeight::RelativeToFont(1.0),
				));
			}
		});
}

fn detail_chip_color(part: &str) -> Color {
	let trimmed = part.trim();
	if trimmed.starts_with('+') {
		TEXT_LIME
	} else if trimmed.starts_with('-') {
		TEXT_SALMON
	} else {
		TEXT_YELLOW_FAINT
	}
}

/// Inventory / catalog cell that pulses while focused or pointer-hovered.
#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub struct HoverTile {
	pub equipped: bool,
	/// Keep the authored fill (color chips). Tiles wash; swatches do not.
	pub preserve_fill: bool,
}

/// Pulse the focused or hovered tile so stick and mouse share the same cue.
pub fn sync_hover_tiles(
	time: Res<Time>,
	menus: Query<&HudMenu>,
	mut tiles: Query<(
		&HoverTile,
		Option<&HudMenuItem>,
		Option<&Interaction>,
		&mut Node,
		&mut BorderColor,
		&mut BackgroundColor,
	)>,
) {
	let pulse = 0.55 + 0.45 * (time.elapsed_secs() * 7.0).sin();
	for (tile, item, interaction, mut node, mut border, mut background) in &mut tiles {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let hovered = matches!(interaction, Some(Interaction::Hovered | Interaction::Pressed));
		if focused || hovered {
			node.border = UiRect::all(Val::Px(TILE_BORDER));
			*border = BorderColor::all(TEXT_YELLOW.with_alpha(0.4 + 0.6 * pulse));
			if !tile.preserve_fill {
				background.0 = Color::srgba(0.98, 0.86, 0.32, 0.06 + 0.12 * pulse);
			}
			continue;
		}
		if tile.equipped {
			node.border = UiRect::all(Val::Px(TILE_BORDER));
			*border = BorderColor::all(TEXT_YELLOW_HOVER);
			if !tile.preserve_fill {
				background.0 = Color::NONE;
			}
			continue;
		}
		node.border = UiRect::all(Val::Px(if tile.preserve_fill { 1.0 } else { TILE_BORDER }));
		*border =
			BorderColor::all(if tile.preserve_fill { TEXT_YELLOW_FAINT } else { Color::NONE });
		if !tile.preserve_fill {
			background.0 = Color::NONE;
		}
	}
}

fn tile_face(selected: bool, muted: bool) -> Color {
	if muted {
		TEXT_YELLOW_FAINT
	} else if selected {
		TEXT_YELLOW_HOVER
	} else {
		TEXT_YELLOW
	}
}

pub fn spawn_tile_grid(
	parent: &mut ChildSpawnerCommands,
	justify: JustifyContent,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			Node {
				display: Display::Grid,
				width: Val::Percent(100.0),
				grid_template_columns: tile_grid_columns(),
				column_gap: Val::Px(PANEL_CHIP_GAP),
				row_gap: Val::Px(PANEL_CHIP_GAP),
				align_items: AlignItems::Stretch,
				justify_content: justify,
				padding: UiRect::all(Val::Px(TILE_FOCUS_PAD)),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(children);
}

#[cfg(test)]
mod tests {
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;

	use super::{sync_hover_tiles, tile_caption, tile_grid_columns, tile_node, HoverTile};
	use crate::controls::hud_menu::{HudMenu, HudMenuItem};
	use crate::theme::PANEL_TILE_COLUMNS;

	#[test]
	fn short_caption_unchanged() {
		assert_eq!(tile_caption("Hair"), "Hair");
	}

	#[test]
	fn long_caption_keeps_every_word() {
		assert_eq!(tile_caption("celestial-red-tide-joggers"), "Celestial Red Tide Joggers");
	}

	#[test]
	fn tile_grid_is_three_columns() {
		assert_eq!(PANEL_TILE_COLUMNS, 3);
		assert_eq!(tile_grid_columns().len(), 1);
		assert_eq!(tile_node().width, Val::Percent(100.0));
	}

	#[test]
	fn focused_tile_gains_a_border() {
		let mut world = World::new();
		world.init_resource::<Time>();
		let menu = world.spawn(HudMenu { selected: 1, item_count: 2 }).id();
		let idle = world
			.spawn((
				HoverTile { equipped: false, preserve_fill: false },
				HudMenuItem { index: 0, menu },
				Node::default(),
				BorderColor::all(Color::NONE),
				BackgroundColor(Color::NONE),
			))
			.id();
		let focused = world
			.spawn((
				HoverTile { equipped: false, preserve_fill: false },
				HudMenuItem { index: 1, menu },
				Node::default(),
				BorderColor::all(Color::NONE),
				BackgroundColor(Color::NONE),
			))
			.id();
		world.run_system_once(sync_hover_tiles).expect("hover");
		let idle_border = world.get::<BorderColor>(idle).expect("idle").top;
		let live_border = world.get::<BorderColor>(focused).expect("focused").top;
		assert_eq!(idle_border, Color::NONE);
		assert_ne!(live_border, Color::NONE);
	}
}
