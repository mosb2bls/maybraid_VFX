//! Hint strip: static mark plus faint copy.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::icons::maybraid::Icon;
use crate::theme::{
	BARLOW_REGULAR, COLUMN_INSET, HINT_BOTTOM, HINT_FONT_SIZE, HINT_ICON_GAP, HINT_ICON_SIZE,
	TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

/// Row that holds the hint mark and the hint [`Text`].
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuHint;

/// Marker on the hint copy [`Text`]. Screens write the string.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct TextMenuHintLabel;

impl TextMenuHint {
	pub fn scene(initial: impl Into<String>) -> impl Scene + 'static {
		let initial = initial.into();
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(Icon::maybraid(HINT_ICON_SIZE, TEXT_YELLOW).scene()),
			Box::new(hint_label_scene(initial)),
		];
		bsn! {
			TextMenuHint
			Node {
				position_type: PositionType::Absolute,
				left: px(COLUMN_INSET),
				right: px(COLUMN_INSET),
				bottom: px(HINT_BOTTOM),
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::Center,
				column_gap: px(HINT_ICON_GAP),
			}
			Pickable::IGNORE
			Children [ {children} ]
		}
	}
}

fn hint_label_scene(initial: String) -> impl Scene {
	bsn! {
		TextMenuHintLabel
		template_value(Text::new(initial))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_REGULAR),
			font_size: px(HINT_FONT_SIZE),
		}
		TextColor(TEXT_YELLOW_FAINT)
		TextLayout::new(Justify::Left, LineBreak::WordBoundary)
		Pickable::IGNORE
	}
}

/// Write `value` onto the [`TextMenuHintLabel`] under `root` (the screen).
pub fn set_hint_for_menu(
	root: Entity,
	value: impl Into<String>,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<TextMenuHintLabel>>,
) {
	if let Ok(mut text) = lines.get_mut(root) {
		text.0 = value.into();
		return;
	}
	let Ok(root_children) = children.get(root) else {
		return;
	};
	let value = value.into();
	for child in root_children {
		if let Ok(mut text) = lines.get_mut(*child) {
			text.0 = value;
			return;
		}
		let Ok(nested) = children.get(*child) else {
			continue;
		};
		for nested_child in nested {
			if let Ok(mut text) = lines.get_mut(*nested_child) {
				text.0 = value;
				return;
			}
		}
	}
}
