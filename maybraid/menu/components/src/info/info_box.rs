//! Callout box: circled `i` in the upper left, rounded border, wrapping copy.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, template_value, Scene};
use bevy::text::{FontSourceTemplate, LineBreak};

use crate::theme::{
	BARLOW_REGULAR, BARLOW_SEMIBOLD, DESCRIPTION_FONT_SIZE, DESCRIPTION_UNDER_COLUMN_GAP,
	INFO_BOX_BORDER, INFO_BOX_FILL, INFO_BOX_MARK_FILL, INFO_BOX_MARK_FONT_SIZE, INFO_BOX_MARK_GAP,
	INFO_BOX_MARK_RADIUS, INFO_BOX_MARK_SIZE, INFO_BOX_MAX_WIDTH, INFO_BOX_PAD_X, INFO_BOX_PAD_Y,
	INFO_BOX_RADIUS, TEXT_YELLOW, TEXT_YELLOW_FAINT,
};

/// Root of an info callout.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuInfoBox;

/// Marker on the body [`Text`]. Screens write the string.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct MenuInfoBoxLabel;

impl MenuInfoBox {
	pub fn scene(text: impl Into<String>) -> impl Scene + 'static {
		let text = text.into();
		let children: Vec<Box<dyn Scene>> =
			vec![Box::new(info_mark_scene()), Box::new(info_label_scene(text))];
		let margin = UiRect { top: Val::Px(DESCRIPTION_UNDER_COLUMN_GAP), ..default() };
		bsn! {
			MenuInfoBox
			Node {
				margin: margin,
				max_width: px(INFO_BOX_MAX_WIDTH),
				padding: UiRect::axes(px(INFO_BOX_PAD_X), px(INFO_BOX_PAD_Y)),
				border: UiRect::all(px(INFO_BOX_BORDER)),
				border_radius: BorderRadius::all(px(INFO_BOX_RADIUS)),
				flex_direction: FlexDirection::Row,
				align_items: AlignItems::FlexStart,
				column_gap: px(INFO_BOX_MARK_GAP),
			}
			BorderColor::all(TEXT_YELLOW_FAINT)
			BackgroundColor(INFO_BOX_FILL)
			Pickable::IGNORE
			Children [ {children} ]
		}
	}
}

fn info_mark_scene() -> impl Scene {
	bsn! {
		Node {
			width: px(INFO_BOX_MARK_SIZE),
			height: px(INFO_BOX_MARK_SIZE),
			flex_shrink: 0.0,
			border: UiRect::all(px(INFO_BOX_BORDER)),
			border_radius: BorderRadius::all(px(INFO_BOX_MARK_RADIUS)),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
		}
		BorderColor::all(TEXT_YELLOW)
		BackgroundColor(INFO_BOX_MARK_FILL)
		Pickable::IGNORE
		Children [(
			template_value(Text::new("i"))
			TextFont {
				font: FontSourceTemplate::Handle(BARLOW_SEMIBOLD),
				font_size: px(INFO_BOX_MARK_FONT_SIZE),
			}
			TextColor(TEXT_YELLOW)
			TextLayout::new(Justify::Center, LineBreak::NoWrap)
			Pickable::IGNORE
		)]
	}
}

fn info_label_scene(text: String) -> impl Scene {
	bsn! {
		MenuInfoBoxLabel
		template_value(Text::new(text))
		TextFont {
			font: FontSourceTemplate::Handle(BARLOW_REGULAR),
			font_size: px(DESCRIPTION_FONT_SIZE),
		}
		TextColor(TEXT_YELLOW_FAINT)
		TextLayout::new(Justify::Left, LineBreak::WordBoundary)
		Pickable::IGNORE
	}
}

/// Write `value` onto the [`MenuInfoBoxLabel`] under `root` (the screen).
pub fn set_info_box_for_menu(
	root: Entity,
	value: impl Into<String>,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<MenuInfoBoxLabel>>,
) {
	let value = value.into();
	set_info_box_under(root, &value, children, lines);
}

fn set_info_box_under(
	entity: Entity,
	value: &str,
	children: &Query<&Children>,
	lines: &mut Query<&mut Text, With<MenuInfoBoxLabel>>,
) -> bool {
	if let Ok(mut text) = lines.get_mut(entity) {
		text.0 = value.to_string();
		return true;
	}
	let Ok(kids) = children.get(entity) else {
		return false;
	};
	for child in kids {
		if set_info_box_under(*child, value, children, lines) {
			return true;
		}
	}
	false
}

#[cfg(test)]
mod tests {
	use crate::theme::{
		INFO_BOX_BORDER, INFO_BOX_MARK_RADIUS, INFO_BOX_MARK_SIZE, INFO_BOX_MAX_WIDTH,
		INFO_BOX_RADIUS,
	};

	#[test]
	fn mark_is_a_circle() {
		assert!((INFO_BOX_MARK_RADIUS - INFO_BOX_MARK_SIZE * 0.5).abs() < f32::EPSILON);
		assert!(INFO_BOX_RADIUS > INFO_BOX_BORDER);
		assert!(INFO_BOX_MAX_WIDTH >= 400.0);
	}
}
