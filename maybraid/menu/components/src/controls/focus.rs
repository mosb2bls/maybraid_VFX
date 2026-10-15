//! Focused-row tag: yellow fill, deep-blue text.

use bevy::prelude::*;

use crate::theme::{
	FOCUS_TAG_PAD_X, FOCUS_TAG_PAD_Y, FOCUS_TAG_RADIUS, TEXT_DEEP_BLUE, TEXT_YELLOW,
	TEXT_YELLOW_FAINT,
};

/// Shrink-wrapped label chip. Focus systems invert it while the row is live.
#[derive(Component, Debug, Clone, Copy)]
pub struct FocusTag {
	pub idle: Color,
}

impl Default for FocusTag {
	fn default() -> Self {
		Self { idle: TEXT_YELLOW }
	}
}

impl FocusTag {
	pub fn new(idle: Color) -> Self {
		Self { idle }
	}

	pub fn node() -> Node {
		Node {
			padding: UiRect::axes(Val::Px(FOCUS_TAG_PAD_X), Val::Px(FOCUS_TAG_PAD_Y)),
			border_radius: BorderRadius::all(Val::Px(FOCUS_TAG_RADIUS)),
			justify_content: JustifyContent::Center,
			align_items: AlignItems::Center,
			flex_shrink: 0.0,
			..default()
		}
	}

	pub fn fill(focused: bool, locked: bool) -> Color {
		if !focused {
			Color::NONE
		} else if locked {
			TEXT_YELLOW.with_alpha(0.45)
		} else {
			TEXT_YELLOW
		}
	}

	pub fn text(focused: bool, locked: bool, idle: Color) -> Color {
		if !focused {
			idle
		} else if locked {
			TEXT_DEEP_BLUE.with_alpha(0.72)
		} else {
			TEXT_DEEP_BLUE
		}
	}
}

/// Spawn a shrink-wrapped tag and run `children` inside it.
pub fn spawn_focus_tag(
	parent: &mut ChildSpawnerCommands,
	idle: Color,
	focused: bool,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			FocusTag::new(idle),
			FocusTag::node(),
			BackgroundColor(FocusTag::fill(focused, false)),
			Pickable::IGNORE,
		))
		.with_children(children);
}

/// Paint fill and descendant type. Unfocused muted spans stay faint.
pub fn paint_focus_tag(
	tag: Entity,
	focused: bool,
	locked: bool,
	idle: Color,
	children: &Query<&Children>,
	backgrounds: &mut Query<&mut BackgroundColor, With<FocusTag>>,
	colors: &mut Query<&mut TextColor>,
) {
	if let Ok(mut background) = backgrounds.get_mut(tag) {
		background.0 = FocusTag::fill(focused, locked);
	}
	let Ok(kids) = children.get(tag) else {
		return;
	};
	if focused {
		let face = FocusTag::text(true, locked, idle);
		for kid in kids {
			set_text_colors_under(*kid, face, children, colors);
		}
		return;
	}
	for kid in kids {
		if let Ok(mut color) = colors.get_mut(*kid) {
			color.0 = idle;
		}
		let Ok(nested) = children.get(*kid) else {
			continue;
		};
		for span in nested {
			if let Ok(mut color) = colors.get_mut(*span) {
				color.0 = TEXT_YELLOW_FAINT;
			}
		}
	}
}

fn set_text_colors_under(
	root: Entity,
	face: Color,
	children: &Query<&Children>,
	colors: &mut Query<&mut TextColor>,
) {
	if let Ok(mut color) = colors.get_mut(root) {
		color.0 = face;
	}
	let Ok(kids) = children.get(root) else {
		return;
	};
	for kid in kids {
		set_text_colors_under(*kid, face, children, colors);
	}
}

pub fn first_focus_tag(
	row_children: &Children,
	tags: &Query<(), With<FocusTag>>,
) -> Option<Entity> {
	row_children.iter().find(|child| tags.contains(*child))
}

#[cfg(test)]
mod tests {
	use super::FocusTag;
	use crate::theme::{TEXT_DEEP_BLUE, TEXT_YELLOW, TEXT_YELLOW_FAINT};
	use bevy::prelude::*;

	#[test]
	fn idle_tag_has_no_fill() {
		assert_eq!(FocusTag::fill(false, false), Color::NONE);
		assert_eq!(FocusTag::fill(false, true), Color::NONE);
		assert_eq!(FocusTag::text(false, false, TEXT_YELLOW_FAINT), TEXT_YELLOW_FAINT);
	}

	#[test]
	fn live_tag_is_navy_on_yellow() {
		assert_eq!(FocusTag::fill(true, false), TEXT_YELLOW);
		assert_eq!(FocusTag::text(true, false, TEXT_YELLOW), TEXT_DEEP_BLUE);
	}

	#[test]
	fn locked_live_tag_keeps_a_dampened_fill() {
		let fill = FocusTag::fill(true, true).to_srgba();
		assert!((fill.red - 1.0).abs() < f32::EPSILON);
		assert!(fill.alpha < 1.0);
		assert_ne!(FocusTag::text(true, true, TEXT_YELLOW_FAINT), TEXT_YELLOW_FAINT);
	}
}
