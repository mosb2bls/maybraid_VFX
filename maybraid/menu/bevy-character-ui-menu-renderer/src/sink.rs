//! Forward contract from the [`MenuNode`] IR to Bevy UI widgets.
//!
//! [`BevyMenuSink`] is the only place that decides how each IR variant paints.
//! It contains no species- or menu-specific knowledge; everything it needs is
//! carried on the nodes.

use bevy::prelude::*;
use character_ui_menu::{
	AssetChoice, AssetThumbnailDisplay, GridCatalogChoice, ItemRow, MenuNode, PreviewColor,
	SectionOpen, SelectChoice, StatCard, StatTone, SwatchChoice, ThumbnailRequest,
};

use crate::widgets::{
	color_from_hex, compact_control_row, inline_chip_row, labeled_row, render_asset_button,
	render_button, select_tile_node, swatch_node, text, tile_text, ToggleSectionKey, ACTIVE,
	INACTIVE, MENU_VERTICAL_GAP, MUTED,
};

/// Renderer-owned thumbnail bridge. The playground adapts this to its cache.
pub trait MenuThumbnailContext {
	fn image_for_asset(
		&mut self,
		label: &'static str,
		asset_path: &'static str,
		color: Color,
		camera: character_ui_menu::ThumbnailCamera,
	) -> Option<Handle<Image>>;

	fn image_for_key(&mut self, _key: u64) -> Option<Handle<Image>> {
		None
	}
}

/// Per-rebuild rendering state shared across the whole node tree.
pub struct RenderContext<'a, T> {
	pub sections: &'a dyn SectionOpen,
	pub thumbnails: &'a mut T,
	pub asset_thumbnails: AssetThumbnailDisplay,
	pub prewarm: &'a mut Vec<ThumbnailRequest>,
}

/// Forward dispatcher over [`MenuNode`] trees.
pub trait MenuSink<E: Copy + Send + Sync + 'static> {
	fn render_node<C: MenuThumbnailContext>(
		&self,
		node: &MenuNode<E>,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	);

	fn render_nodes<C: MenuThumbnailContext>(
		&self,
		nodes: &[MenuNode<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		for node in nodes {
			self.render_node(node, parent, context);
		}
	}
}

/// Bevy UI implementation of the menu forward contract.
#[derive(Default)]
pub struct BevyMenuSink;

impl<E: Copy + Send + Sync + 'static> MenuSink<E> for BevyMenuSink {
	fn render_node<C: MenuThumbnailContext>(
		&self,
		node: &MenuNode<E>,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		match node {
			MenuNode::Fragment(children) => self.render_nodes(children, parent, context),
			MenuNode::Section { label, children } => self.section(label, children, parent, context),
			MenuNode::SectionSelect { label, groups, children } => {
				block_label(parent, label);
				for group in groups {
					if let Some(group_label) = group.label {
						text(parent, group_label, 10.0, MUTED);
					}
					parent.spawn((inline_chip_row(), Pickable::IGNORE)).with_children(|row| {
						for choice in &group.choices {
							row.spawn((
								Button,
								select_tile_node(),
								BackgroundColor(if choice.selected { ACTIVE } else { INACTIVE }),
								crate::widgets::MenuButton(choice.event),
							))
							.with_children(|button| {
								tile_text(button, choice.label, 9.0, Color::WHITE)
							});
						}
					});
				}
				self.render_nodes(children, parent, context);
			}
			MenuNode::LabeledCycle { label, value, minus, plus } => {
				inline_label_row(parent, label, |row| {
					row.spawn((compact_control_row(), Pickable::IGNORE)).with_children(
						|controls| {
							render_button(controls, "<", *minus, false);
							text(controls, value, 11.0, VALUE_COLOR);
							render_button(controls, ">", *plus, false);
						},
					);
				});
			}
			MenuNode::LabeledSlider { label, value, decrease, increase } => {
				inline_label_row(parent, label, |row| {
					row.spawn((compact_control_row(), Pickable::IGNORE)).with_children(
						|controls| {
							render_button(controls, "-", *decrease, false);
							text(controls, &format!("{value:.2}"), 11.0, VALUE_COLOR);
							render_button(controls, "+", *increase, false);
						},
					);
				});
			}
			MenuNode::LabeledSwatch { label, choices } => {
				inline_label_row(parent, label, |row| swatch_row(row, choices));
			}
			MenuNode::BlockAsset { label, preview, choices } => {
				block_label(parent, label);
				let preview = bevy_color(*preview);
				parent
					.spawn((
						Node {
							width: Val::Percent(100.0),
							flex_direction: FlexDirection::Row,
							flex_wrap: FlexWrap::Wrap,
							column_gap: Val::Px(8.0),
							row_gap: Val::Px(MENU_VERTICAL_GAP),
							..default()
						},
						Pickable::IGNORE,
					))
					.with_children(|grid| {
						for choice in choices {
							let thumbnail = asset_thumbnail(choice, preview, context);
							render_asset_button(
								grid,
								choice.label,
								choice.event,
								choice.selected,
								thumbnail,
							);
						}
					});
			}
			MenuNode::ItemMultiSelect { label, rows } => {
				block_label(parent, label);
				for row in rows {
					self.item_row(row, parent, context);
				}
			}
			MenuNode::GridCatalog { label, choices, .. } => {
				block_label(parent, label);
				self.grid_catalog(choices, parent, context);
			}
			MenuNode::ShortText { label, value, .. } => {
				inline_label_row(parent, label, |row| {
					text(row, value, 11.0, VALUE_COLOR);
				});
			}
			MenuNode::Action { label, event } => {
				render_button(parent, label, *event, false);
			}
			MenuNode::LabeledValue { label, value } => {
				inline_label_row(parent, label, |row| {
					text(row, value, 11.0, playground_tone_color(StatTone::from_text(value)));
				});
			}
			MenuNode::StatGrid { cards } => self.stat_grid(cards, parent),
		}
	}
}

const VALUE_COLOR: Color = Color::srgb(0.85, 0.95, 1.0);
const PLUS_COLOR: Color = Color::srgb(0.55, 0.92, 0.38);
const MINUS_COLOR: Color = Color::srgb(0.95, 0.45, 0.42);

fn playground_tone_color(tone: StatTone) -> Color {
	match tone {
		StatTone::Plus => PLUS_COLOR,
		StatTone::Minus => MINUS_COLOR,
		StatTone::Formula => VALUE_COLOR,
		StatTone::Neutral => MUTED,
	}
}

impl BevyMenuSink {
	fn section<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		label: &'static str,
		children: &[MenuNode<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let open = context.sections.is_open(label);
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(MENU_VERTICAL_GAP),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|section| {
				section
					.spawn((
						Button,
						Node {
							min_width: Val::Px(28.0),
							height: Val::Px(crate::widgets::BUTTON_HEIGHT),
							padding: UiRect::axes(
								Val::Px(7.0),
								Val::Px(crate::widgets::MENU_BUTTON_PADDING_V),
							),
							justify_content: JustifyContent::Center,
							align_items: AlignItems::Center,
							..default()
						},
						BackgroundColor(if open { ACTIVE } else { INACTIVE }),
						ToggleSectionKey(label),
					))
					.with_children(|button| {
						text(
							button,
							&format!("{} {}", if open { "v" } else { ">" }, label),
							10.0,
							Color::WHITE,
						);
					});
				if open {
					self.render_nodes(children, section, context);
				}
			});
	}

	fn grid_catalog<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		choices: &[GridCatalogChoice<E>],
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					column_gap: Val::Px(8.0),
					row_gap: Val::Px(MENU_VERTICAL_GAP),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|grid| {
				for choice in choices {
					let thumbnail =
						grid_catalog_thumbnail(choice, bevy_color(choice.preview), context);
					let label = if choice.detail.is_empty() {
						choice.label.clone()
					} else {
						format!("{}\n{}", choice.label, choice.detail)
					};
					render_asset_button(grid, &label, choice.event, choice.selected, thumbnail);
				}
			});
	}

	fn stat_grid(&self, cards: &[StatCard], parent: &mut ChildSpawnerCommands) {
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Row,
					flex_wrap: FlexWrap::Wrap,
					column_gap: Val::Px(12.0),
					row_gap: Val::Px(20.0),
					align_items: AlignItems::FlexStart,
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|grid| {
				for card in cards {
					grid.spawn((
						Node {
							width: Val::Percent(31.0),
							min_width: Val::Px(140.0),
							flex_direction: FlexDirection::Column,
							row_gap: Val::Px(8.0),
							padding: UiRect::vertical(Val::Px(6.0)),
							..default()
						},
						Pickable::IGNORE,
					))
					.with_children(|column| {
						text(column, &card.title, 14.0, Color::WHITE);
						for row in &card.rows {
							inline_label_row(column, &row.label, |line| {
								text(line, &row.value, 10.0, playground_tone_color(row.tone));
							});
						}
					});
				}
			});
	}

	fn item_row<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
		&self,
		row: &ItemRow<E>,
		parent: &mut ChildSpawnerCommands,
		context: &mut RenderContext<'_, C>,
	) {
		let thumbnail = asset_thumbnail(&row.asset, bevy_color(row.preview), context);
		parent
			.spawn((
				Node {
					width: Val::Percent(100.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(MENU_VERTICAL_GAP),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|item| {
				item.spawn((
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Row,
						column_gap: Val::Px(6.0),
						row_gap: Val::Px(MENU_VERTICAL_GAP),
						align_items: AlignItems::Center,
						flex_wrap: FlexWrap::Wrap,
						..default()
					},
					Pickable::IGNORE,
				))
				.with_children(|top| {
					render_asset_button(
						top,
						row.asset.label,
						row.asset.event,
						row.asset.selected,
						thumbnail,
					);
					swatch_row(top, &row.colors);
				});
				if !row.materials.is_empty() {
					select_row(item, &row.materials);
				}
			});
	}
}

fn block_label(parent: &mut ChildSpawnerCommands, label: &str) {
	text(parent, label, 12.0, Color::srgb(0.78, 0.84, 0.92));
}

fn inline_label_row(
	parent: &mut ChildSpawnerCommands,
	label: &str,
	controls: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent.spawn((labeled_row(), Pickable::IGNORE)).with_children(|row| {
		text(row, label, 11.0, Color::WHITE);
		controls(row);
	});
}

fn swatch_row<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	choices: &[SwatchChoice<E>],
) {
	parent.spawn((inline_chip_row(), Pickable::IGNORE)).with_children(|row| {
		for choice in choices {
			row.spawn((
				Button,
				swatch_node(choice.selected),
				BorderColor::all(if choice.selected { Color::WHITE } else { MUTED }),
				BackgroundColor(color_from_hex(choice.color_hex)),
				crate::widgets::MenuButton(choice.event),
			));
		}
	});
}

fn select_row<E: Copy + Send + Sync + 'static>(
	parent: &mut ChildSpawnerCommands,
	choices: &[SelectChoice<E>],
) {
	parent.spawn((inline_chip_row(), Pickable::IGNORE)).with_children(|row| {
		for choice in choices {
			row.spawn((
				Button,
				select_tile_node(),
				BackgroundColor(if choice.selected { ACTIVE } else { INACTIVE }),
				crate::widgets::MenuButton(choice.event),
			))
			.with_children(|button| tile_text(button, choice.label, 9.0, Color::WHITE));
		}
	});
}

fn asset_thumbnail<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	choice: &AssetChoice<E>,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	thumbnail_image(choice.label, choice.path, choice.thumbnail_camera, preview, context)
}

fn grid_catalog_thumbnail<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	choice: &GridCatalogChoice<E>,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	if let Some(key) = choice.image_key {
		if let Some(image) = context.thumbnails.image_for_key(key) {
			return Some(image);
		}
	}
	thumbnail_image(choice.path, choice.path, choice.thumbnail_camera, preview, context)
}

fn thumbnail_image<C: MenuThumbnailContext>(
	label: &'static str,
	path: &'static str,
	camera: character_ui_menu::ThumbnailCamera,
	preview: Color,
	context: &mut RenderContext<'_, C>,
) -> Option<Handle<Image>> {
	if context.asset_thumbnails != AssetThumbnailDisplay::None && !path.is_empty() {
		context.prewarm.push(ThumbnailRequest::new(path, color_key(preview), camera));
	}
	match context.asset_thumbnails {
		AssetThumbnailDisplay::Inline => {
			context.thumbnails.image_for_asset(label, path, preview, camera)
		}
		_ => None,
	}
}

fn bevy_color(preview: PreviewColor) -> Color {
	Color::srgba(preview.red, preview.green, preview.blue, preview.alpha)
}

fn color_key(color: Color) -> [u8; 4] {
	let srgba = color.to_srgba();
	[
		(srgba.red * 255.0) as u8,
		(srgba.green * 255.0) as u8,
		(srgba.blue * 255.0) as u8,
		(srgba.alpha * 255.0) as u8,
	]
}
