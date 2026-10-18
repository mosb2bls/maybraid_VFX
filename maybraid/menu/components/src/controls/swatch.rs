//! Color chips.

use bevy::prelude::*;

use crate::theme::{
	PANEL_CHIP_GAP, PANEL_SWATCH_SIZE, TEXT_YELLOW, TEXT_YELLOW_FAINT, TILE_FOCUS_PAD,
};

/// Parse `#RRGGBB`; malformed input falls back to faint yellow.
pub fn color_from_hex(hex: &str) -> Color {
	let hex = hex.strip_prefix('#').unwrap_or(hex);
	if hex.len() != 6 {
		return TEXT_YELLOW_FAINT;
	}
	let Ok(red) = u8::from_str_radix(&hex[0..2], 16) else {
		return TEXT_YELLOW_FAINT;
	};
	let Ok(green) = u8::from_str_radix(&hex[2..4], 16) else {
		return TEXT_YELLOW_FAINT;
	};
	let Ok(blue) = u8::from_str_radix(&hex[4..6], 16) else {
		return TEXT_YELLOW_FAINT;
	};
	Color::srgb(red as f32 / 255.0, green as f32 / 255.0, blue as f32 / 255.0)
}

/// One pickable color chip. `extra` is typically `MenuButton<E>`.
pub fn spawn_swatch(
	parent: &mut ChildSpawnerCommands,
	hex: &str,
	selected: bool,
	extra: impl Bundle,
) {
	parent.spawn((
		Button,
		crate::controls::tile::HoverTile {
			equipped: selected,
			preserve_fill: true,
			face: TEXT_YELLOW,
		},
		extra,
		Node {
			width: Val::Px(PANEL_SWATCH_SIZE),
			height: Val::Px(PANEL_SWATCH_SIZE),
			flex_shrink: 0.0,
			border: UiRect::all(Val::Px(if selected { 2.0 } else { 1.0 })),
			..default()
		},
		BorderColor::all(if selected { TEXT_YELLOW } else { TEXT_YELLOW_FAINT }),
		BackgroundColor(color_from_hex(hex)),
		Outline::new(Val::Px(2.0), Val::Px(2.0), Color::NONE),
	));
}

pub fn spawn_swatch_row(
	parent: &mut ChildSpawnerCommands,
	justify: JustifyContent,
	children: impl FnOnce(&mut ChildSpawnerCommands),
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				flex_wrap: FlexWrap::Wrap,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				row_gap: Val::Px(PANEL_CHIP_GAP),
				align_items: AlignItems::Center,
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
	use super::color_from_hex;
	use crate::theme::TEXT_YELLOW_FAINT;

	#[test]
	fn parses_rrggbb() {
		let color = color_from_hex("#ff0000").to_srgba();
		assert!((color.red - 1.0).abs() < f32::EPSILON);
		assert!(color.green.abs() < f32::EPSILON);
	}

	#[test]
	fn malformed_falls_back() {
		assert_eq!(color_from_hex("zzz"), TEXT_YELLOW_FAINT);
	}
}
