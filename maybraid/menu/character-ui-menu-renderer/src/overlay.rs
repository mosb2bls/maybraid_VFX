//! Overlay routing. The IR stays `MenuNode`; this crate decides what opens
//! a picker and what paints inside it.

use bevy::prelude::*;
use character_ui_menu::{MenuNode, SelectGroup};

use crate::sink::{MaybraidMenuSink, MenuSink, MenuThumbnailContext, RenderContext};
use crate::widgets::CloseOverlaySelect;
use crate::MenuJustify;
use menu_components::{
	spawn_header_line, spawn_text_button, HudFonts, PANEL_HEADER_FONT_SIZE, PANEL_ROW_GAP,
};

/// Catalog nodes that can show a selected name on a header.
pub fn is_select_node<E>(node: &MenuNode<E>) -> bool {
	matches!(
		node,
		MenuNode::SectionSelect { .. }
			| MenuNode::BlockAsset { .. }
			| MenuNode::ItemMultiSelect { .. }
			| MenuNode::GridCatalog { .. }
	)
}

/// Flatten fragments so a section's real children can be inspected.
pub fn flatten_nodes<E>(nodes: &[MenuNode<E>]) -> Vec<&MenuNode<E>> {
	let mut out = Vec::new();
	flatten_into(nodes, &mut out);
	out
}

fn flatten_into<'a, E>(nodes: &'a [MenuNode<E>], out: &mut Vec<&'a MenuNode<E>>) {
	for node in nodes {
		match node {
			MenuNode::Fragment(children) => flatten_into(children, out),
			other => out.push(other),
		}
	}
}

/// The lone catalog under a section, if there is exactly one.
pub fn primary_select<'a, E>(children: &'a [MenuNode<E>]) -> Option<&'a MenuNode<E>> {
	let selects: Vec<_> = flatten_nodes(children)
		.into_iter()
		.filter(|node| is_select_node(node))
		.collect();
	(selects.len() == 1).then(|| selects[0])
}

/// True when the overlay is only a single catalog (no sliders/swatches).
pub fn is_picker_only<E>(node: &MenuNode<E>) -> bool {
	match node {
		MenuNode::SectionSelect { .. } | MenuNode::BlockAsset { .. } => true,
		MenuNode::ItemMultiSelect { .. } | MenuNode::GridCatalog { .. } => false,
		MenuNode::Section { children, .. } => {
			let flat = flatten_nodes(children);
			flat.len() == 1 && is_select_node(flat[0]) && is_picker_only(flat[0])
		}
		MenuNode::ShortText { .. }
		| MenuNode::Action { .. }
		| MenuNode::LabeledValue { .. }
		| MenuNode::StatGrid { .. } => false,
		_ => false,
	}
}

pub fn overlay_closes_on_pick<E>(node: &MenuNode<E>) -> bool {
	is_picker_only(node)
}

/// Walk a normalized tree for the node whose label matches `key`.
pub fn find_overlay_node<'a, E>(nodes: &'a [MenuNode<E>], key: &str) -> Option<&'a MenuNode<E>> {
	for node in nodes {
		if let Some(found) = find_in_node(node, key) {
			return Some(found);
		}
	}
	None
}

fn find_in_node<'a, E>(node: &'a MenuNode<E>, key: &str) -> Option<&'a MenuNode<E>> {
	match node {
		MenuNode::Fragment(children) => find_overlay_node(children, key),
		MenuNode::Section { label, children } => {
			if *label == key {
				Some(node)
			} else {
				find_overlay_node(children, key)
			}
		}
		MenuNode::SectionSelect { label, children, .. } => {
			if *label == key {
				Some(node)
			} else {
				find_overlay_node(children, key)
			}
		}
		MenuNode::BlockAsset { label, .. }
		| MenuNode::ItemMultiSelect { label, .. }
		| MenuNode::GridCatalog { label, .. }
			if *label == key =>
		{
			Some(node)
		}
		MenuNode::ShortText { .. }
		| MenuNode::Action { .. }
		| MenuNode::LabeledValue { .. }
		| MenuNode::StatGrid { .. }
		| MenuNode::LabeledCycle { .. }
		| MenuNode::LabeledSlider { .. }
		| MenuNode::LabeledSwatch { .. }
		| MenuNode::BlockAsset { .. }
		| MenuNode::ItemMultiSelect { .. }
		| MenuNode::GridCatalog { .. } => None,
	}
}

pub fn overlay_summary_value<E>(node: &MenuNode<E>) -> String {
	match node {
		MenuNode::Section { children, .. } => primary_select(children)
			.map(overlay_summary_value)
			.unwrap_or_else(|| labeled_values_preview(children)),
		MenuNode::SectionSelect { groups, .. } => {
			selected_select_label(groups).unwrap_or("—").to_string()
		}
		MenuNode::BlockAsset { choices, .. } => choices
			.iter()
			.find(|choice| choice.selected)
			.map(|choice| choice.label.to_string())
			.unwrap_or_else(|| "—".into()),
		MenuNode::ItemMultiSelect { rows, .. } => {
			let worn = rows.iter().filter(|row| row.asset.selected).count();
			format!("{worn} worn")
		}
		MenuNode::GridCatalog { label, choices, .. } => {
			let count = choices.iter().filter(|choice| choice.selected).count();
			if *label == "Weapons" || *label == "Skill Maps" {
				format!("{count} queued")
			} else {
				format!("{count} worn")
			}
		}
		MenuNode::LabeledValue { value, .. } => value.clone(),
		MenuNode::StatGrid { cards } => stat_grid_preview(cards),
		_ => String::new(),
	}
}

pub(crate) fn labeled_values_preview<E>(children: &[MenuNode<E>]) -> String {
	let flat = flatten_nodes(children);
	if let Some(preview) = flat.iter().find_map(|node| match node {
		MenuNode::StatGrid { cards } => Some(stat_grid_preview(cards)),
		_ => None,
	}) {
		if !preview.is_empty() {
			return preview;
		}
	}
	let rows: Vec<_> = flat
		.into_iter()
		.filter_map(|node| match node {
			MenuNode::LabeledValue { label, value } => Some((label.as_str(), value.as_str())),
			_ => None,
		})
		.collect();
	let health = rows.iter().find(|(label, _)| *label == "Health").map(|(_, value)| *value);
	let weight = rows.iter().find(|(label, _)| *label == "Weight").map(|(_, value)| *value);
	match (health, weight) {
		(Some(health), Some(weight)) => format!("{health} HP · {weight} wt"),
		(Some(health), None) => format!("{health} HP"),
		_ => rows.first().map(|(_, value)| (*value).to_string()).unwrap_or_default(),
	}
}

fn stat_grid_preview(cards: &[character_ui_menu::StatCard]) -> String {
	let total = cards.iter().find(|card| card.title == "Total Stats");
	let health = total.and_then(|card| {
		card.rows.iter().find(|row| row.label == "Health").map(|row| row.value.as_str())
	});
	let weight = total.and_then(|card| {
		card.rows
			.iter()
			.find(|row| row.label == "Added Weight")
			.map(|row| row.value.as_str())
	});
	match (health, weight) {
		(Some(health), Some(weight)) => format!("{health} HP · {weight} wt"),
		(Some(health), None) => format!("{health} HP"),
		_ => cards.first().map(|card| card.title.clone()).unwrap_or_default(),
	}
}

fn selected_select_label<E>(groups: &[SelectGroup<E>]) -> Option<&'static str> {
	groups
		.iter()
		.flat_map(|group| group.choices.iter())
		.find(|choice| choice.selected)
		.map(|choice| choice.label)
}

/// Full-screen picker chrome. The host fills the returned viewport.
pub fn spawn_overlay_shell(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	title: &str,
	title_color: Color,
) -> Entity {
	let mut viewport = Entity::PLACEHOLDER;
	parent
		.spawn((
			crate::widgets::OverlaySelectRoot,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Button,
				CloseOverlaySelect,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.88)),
			));
			root.spawn((
				Node {
					position_type: PositionType::Absolute,
					left: Val::Percent(6.0),
					top: Val::Percent(8.0),
					width: Val::Percent(62.0),
					height: Val::Percent(84.0),
					flex_direction: FlexDirection::Column,
					row_gap: Val::Px(PANEL_ROW_GAP),
					padding: UiRect::all(Val::Px(8.0)),
					..default()
				},
				Pickable::IGNORE,
			))
			.with_children(|panel| {
				panel
					.spawn((
						Node {
							width: Val::Percent(100.0),
							flex_direction: FlexDirection::Row,
							justify_content: JustifyContent::SpaceBetween,
							align_items: AlignItems::Center,
							..default()
						},
						Pickable::IGNORE,
					))
					.with_children(|header| {
						spawn_header_line(
							header,
							fonts,
							title,
							None,
							PANEL_HEADER_FONT_SIZE,
							title_color,
						);
						spawn_text_button(header, fonts, "back", CloseOverlaySelect);
					});
				viewport = menu_components::spawn_scroll_pane(
					panel,
					crate::widgets::OverlaySelectViewport,
					AlignItems::FlexStart,
					PANEL_ROW_GAP,
				);
			});
		});
	viewport
}

/// Overlay interiors are the sink with catalogs painted inline.
pub fn render_overlay_body<E: Copy + Send + Sync + 'static, C: MenuThumbnailContext>(
	node: &MenuNode<E>,
	parent: &mut ChildSpawnerCommands,
	context: &mut RenderContext<'_, C>,
	justify: MenuJustify,
) {
	MaybraidMenuSink::overlay(justify).render_node(node, parent, context);
}

#[cfg(test)]
mod tests {
	use super::{
		find_overlay_node, is_picker_only, overlay_closes_on_pick, overlay_summary_value,
		primary_select,
	};
	use character_ui_menu::{MenuNode, PreviewColor, SelectChoice, SelectGroup};

	fn clip() -> MenuNode<u8> {
		MenuNode::BlockAsset { label: "Clip", preview: PreviewColor::WHITE, choices: vec![] }
	}

	#[test]
	fn animation_section_is_a_picker() {
		let section = MenuNode::<u8>::Section { label: "Animation", children: vec![clip()] };
		assert!(is_picker_only(&section));
		assert!(overlay_closes_on_pick(&section));
	}

	#[test]
	fn mixed_section_stays_open() {
		let section = MenuNode::<u8>::Section {
			label: "Hair",
			children: vec![
				clip(),
				MenuNode::LabeledSwatch { label: "Hair Color", choices: vec![] },
			],
		};
		assert!(!is_picker_only(&section));
		assert!(!overlay_closes_on_pick(&section));
		assert!(primary_select(match &section {
			MenuNode::Section { children, .. } => children,
			_ => unreachable!(),
		})
		.is_some());
	}

	#[test]
	fn species_header_shows_selected_name() {
		let node = MenuNode::<u8>::SectionSelect {
			label: "Species",
			groups: vec![SelectGroup::unlabeled(vec![SelectChoice {
				label: "braidman",
				selected: true,
				event: 1,
			}])],
			children: vec![],
		};
		assert_eq!(overlay_summary_value(&node), "braidman");
	}

	#[test]
	fn empty_section_has_no_primary_select() {
		assert!(primary_select::<u8>(&[]).is_none());
		assert!(!is_picker_only(&MenuNode::<u8>::Section { label: "Head", children: vec![] }));
	}

	#[test]
	fn finds_section() {
		let tree = [MenuNode::<u8>::Section { label: "Head", children: vec![] }];
		assert!(matches!(
			find_overlay_node(&tree, "Head"),
			Some(MenuNode::Section { label: "Head", .. })
		));
	}
}
