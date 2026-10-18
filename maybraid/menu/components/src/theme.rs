//! Placeholder type tokens for Maybraid HUD menus.
//!
//! Faces and sizes are a first cut so screens can iterate by eye.

use bevy::prelude::*;

/// Barlow Semi Condensed Black, under `maybraid/assets`.
pub const BARLOW_BLACK: &str = "fonts/barlow/BarlowSemiCondensed-Black.ttf";

/// Barlow Semi Condensed SemiBold, under `maybraid/assets`.
pub const BARLOW_SEMIBOLD: &str = "fonts/barlow/BarlowSemiCondensed-SemiBold.ttf";

/// Barlow Semi Condensed Regular, under `maybraid/assets`.
pub const BARLOW_REGULAR: &str = "fonts/barlow/BarlowSemiCondensed-Regular.ttf";

/// Noto Sans Regular — IPA coverage for generated place names.
pub const NOTO_SANS_REGULAR: &str = "fonts/noto/NotoSans-Regular.ttf";

/// Largest face on a text menu (title / header).
pub const HEADER_FONT_SIZE: f32 = 96.0;

/// Large option labels, still smaller than [`HEADER_FONT_SIZE`].
pub const ITEM_FONT_SIZE: f32 = 48.0;

/// Full-screen menu wash (home and other title screens).
pub const MENU_CLEAR: Color = Color::srgb(0.08, 0.10, 0.14);

/// Deep navy: inverted-tag text on [`TEXT_YELLOW`].
pub const TEXT_DEEP_BLUE: Color = Color::srgb(0.07, 0.14, 0.40);

/// Alpha of the in-game modal dimmer. RGB is [`MENU_CLEAR`].
pub const MENU_REGION_WASH_ALPHA: f32 = 0.72;

/// Semi-transparent home-screen blue behind in-game list panels and modal dimmers.
///
/// Full-screen shells that already clear to [`MENU_CLEAR`] should not use this.
pub fn menu_region_wash() -> Color {
	MENU_CLEAR.with_alpha(MENU_REGION_WASH_ALPHA)
}

/// Corner radius on in-game menu-item lists and modal cards.
pub const MENU_LIST_RADIUS: f32 = 16.0;

/// Padding inside a rounded menu-item list so focus tags sit off the corners.
pub const MENU_LIST_PAD: f32 = 16.0;

/// Right-justified character HUD panel width.
pub const CHARACTER_PANEL_WIDTH: f32 = 480.0;

/// Inset of the character HUD from the window top and right.
pub const CHARACTER_PANEL_INSET: f32 = 10.0;

/// Gap between an overlay picker and the character HUD.
pub const CHARACTER_OVERLAY_GAP: f32 = 16.0;

/// Two-column character screen: leftover preview / picker (`1fr`) | HUD (480px).
///
/// Overlay and HUD are cells of this grid so they cannot overlap. Corner chrome
/// (Back / Save) stays absolutely positioned on the same root.
pub fn character_screen_grid_node() -> Node {
	Node {
		width: Val::Percent(100.0),
		height: Val::Percent(100.0),
		display: Display::Grid,
		grid_template_columns: vec![GridTrack::flex(1.0), GridTrack::px(CHARACTER_PANEL_WIDTH)],
		column_gap: Val::Px(CHARACTER_OVERLAY_GAP),
		padding: UiRect {
			left: Val::Px(CHARACTER_PANEL_INSET),
			right: Val::Px(CHARACTER_PANEL_INSET),
			top: Val::Px(CHARACTER_PANEL_INSET),
			bottom: Val::Px(COLUMN_BOTTOM),
			..default()
		},
		..default()
	}
}

/// Rounded list / modal-card chrome. Pair with [`menu_region_wash`].
pub fn menu_list_panel_node(align_items: AlignItems) -> Node {
	Node {
		flex_direction: FlexDirection::Column,
		align_items,
		padding: UiRect::all(Val::Px(MENU_LIST_PAD)),
		border_radius: BorderRadius::all(Val::Px(MENU_LIST_RADIUS)),
		overflow: Overflow::clip(),
		..default()
	}
}

/// Idle menu yellow.
pub const TEXT_YELLOW: Color = Color::srgb(1.0, 0.86, 0.22);

/// Hover / keyboard-focus yellow (slightly darker).
pub const TEXT_YELLOW_HOVER: Color = Color::srgb(0.82, 0.68, 0.12);

/// Description strip: same yellow, lower alpha. Read-only stats use this too.
pub const TEXT_YELLOW_FAINT: Color = Color::srgba(1.0, 0.86, 0.22, 0.42);

/// Focused locked-row yellow: still dampened, a bit stronger than idle faint.
pub const TEXT_YELLOW_FAINT_FOCUS: Color = Color::srgba(1.0, 0.86, 0.22, 0.68);

/// Lime-adjacent violet for “Coming Soon” objective badges.
pub const TEXT_PURPLE: Color = Color::srgb(0.72, 0.48, 0.92);

/// Positive modifier (`+6`).
pub const TEXT_LIME: Color = Color::srgb(0.55, 0.92, 0.38);

/// Negative modifier (`-4`).
pub const TEXT_SALMON: Color = Color::srgb(0.95, 0.45, 0.42);

/// Derived formula (pace equation).
pub const TEXT_LIGHT_BLUE: Color = Color::srgb(0.62, 0.86, 1.0);

/// Badge beside a text-cursor title. Smaller than the adjacent [`ITEM_FONT_SIZE`]
/// line so the tag reads as a chip, not a second title.
pub const OBJECTIVE_MARKER_FONT_SIZE: f32 = 24.0;

/// Horizontal padding inside an objective tag.
pub const OBJECTIVE_MARKER_PAD_X: f32 = 8.0;

/// Vertical padding inside an objective tag.
pub const OBJECTIVE_MARKER_PAD_Y: f32 = 1.0;

/// Stroke on an objective tag.
pub const OBJECTIVE_MARKER_BORDER: f32 = 1.5;

/// Corner radius on an objective tag.
pub const OBJECTIVE_MARKER_RADIUS: f32 = 8.0;

/// Optical drop so the chip sits on the title’s ink, not the top of its line box.
pub const OBJECTIVE_MARKER_OFFSET_Y: f32 = 3.0;

/// Description line under the menu column.
pub const DESCRIPTION_FONT_SIZE: f32 = 22.0;

/// Hint line uses the same face size as the description.
pub const HINT_FONT_SIZE: f32 = DESCRIPTION_FONT_SIZE;

/// Display size of the hint mark (matches the type optically).
pub const HINT_ICON_SIZE: f32 = 24.0;

/// Gap between the hint mark and the hint copy.
pub const HINT_ICON_GAP: f32 = 10.0;

/// Horizontal padding inside a focused-row tag. The chip hugs the label.
pub const FOCUS_TAG_PAD_X: f32 = 10.0;

/// Vertical padding inside a focused-row tag.
pub const FOCUS_TAG_PAD_Y: f32 = 2.0;

/// Corner radius on a focused-row tag.
pub const FOCUS_TAG_RADIUS: f32 = 6.0;

/// Gap between a focused-row tag and a sibling objective chip.
pub const FOCUS_TAG_GAP: f32 = 10.0;

/// Maybraid mark, under `maybraid/assets`. Author file: `art/iconography/maybraid_logo_icon.blend`.
pub const MAYBRAID_LOGO: &str = "iconography/maybraid_logo_icon.png";

/// Death / last-body mark. Author file: `art/iconography/bones_icon.blend`.
pub const BONES_ICON: &str = "iconography/bones_icon.png";

/// Kenney cartography pine. A single tree.
pub const MAP_TREE_ICON: &str = "iconography/kenney/cartography/tree_pine.png";

/// Kenney cartography pine cluster. A grove or forest.
pub const MAP_GROVE_ICON: &str = "iconography/kenney/cartography/tree_pines.png";

/// Kenney cartography house. Discoverable places.
pub const MAP_HOUSE_ICON: &str = "iconography/kenney/cartography/house.png";

/// Kenney cartography houses. Urban cells.
pub const MAP_TOWN_ICON: &str = "iconography/kenney/cartography/houses.png";

/// Kenney cartography mountain. Landform features.
pub const MAP_MOUNTAIN_ICON: &str = "iconography/kenney/cartography/mountain.png";

/// Kenney cartography lake. Water features.
pub const MAP_WATER_ICON: &str = "iconography/kenney/cartography/lake.png";

/// Kenney game-icons white arrow. Rotated toward off-screen anchors.
pub const MAP_ARROW_ICON: &str = "iconography/kenney/game-icons/arrow_up.png";

/// Inset from the bottom of the window for the description pane.
pub const DESCRIPTION_BOTTOM: f32 = 48.0;

/// Reserved height of the description band. Copy is centered inside it so
/// one- and two-line strings share the same vertical slot at the bottom of
/// the remainder.
pub const DESCRIPTION_BAND_HEIGHT: f32 = 80.0;

/// Left edge of the right-hand info pane, as a percent of the window.
/// The menu occupies the left; this remainder is for description (and later display).
pub const DESCRIPTION_PANE_LEFT_PERCENT: f32 = 40.0;

/// Hint strip sits above the description so wrapped copy does not collide.
pub const HINT_BOTTOM: f32 = 100.0;

/// Extra space under a header before the first option.
pub const HEADER_MARGIN_BOTTOM: f32 = 16.0;

/// Gap between stacked options.
pub const ITEM_ROW_GAP: f32 = 6.0;

/// Inset from the left of the window.
pub const COLUMN_INSET: f32 = 48.0;

/// Tight inset for a corner action (spin-reveal Next — no description strip).
pub const CORNER_INSET: f32 = 16.0;

/// Upper-corner brand / mode line (`Maybraid - Discovery`).
pub const BRAND_MODE_FONT_SIZE: f32 = 36.0;

/// Menu column sits above the footer chrome (Back / Edit).
pub const COLUMN_BOTTOM: f32 = 120.0;

/// Corner action (Back / Edit / Save) inset from the window bottom.
pub const CORNER_BOTTOM: f32 = 40.0;

/// Gap between the last menu row and explainer copy stacked under the column.
pub const DESCRIPTION_UNDER_COLUMN_GAP: f32 = 24.0;

/// Wrap width for explainer copy under a top-left menu.
pub const DESCRIPTION_UNDER_COLUMN_MAX_WIDTH: f32 = 440.0;

/// Info-box body width under a centered column.
pub const INFO_BOX_MAX_WIDTH: f32 = DESCRIPTION_UNDER_COLUMN_MAX_WIDTH;

/// Horizontal padding inside an info box.
pub const INFO_BOX_PAD_X: f32 = 16.0;

/// Vertical padding inside an info box.
pub const INFO_BOX_PAD_Y: f32 = 14.0;

/// Corner radius on the info-box border.
pub const INFO_BOX_RADIUS: f32 = 12.0;

/// Stroke on the info-box border.
pub const INFO_BOX_BORDER: f32 = 1.5;

/// Diameter of the circled `i` mark.
pub const INFO_BOX_MARK_SIZE: f32 = 28.0;

/// Radius of the circled `i` mark (half of [`INFO_BOX_MARK_SIZE`]).
pub const INFO_BOX_MARK_RADIUS: f32 = 14.0;

/// Gap between the circled `i` and the body copy.
pub const INFO_BOX_MARK_GAP: f32 = 12.0;

/// Face size of the circled `i`.
pub const INFO_BOX_MARK_FONT_SIZE: f32 = 18.0;

/// Wash inside the info box (menu yellow, very low alpha).
pub const INFO_BOX_FILL: Color = Color::srgba(1.0, 0.86, 0.22, 0.06);

/// Wash inside the circled `i`.
pub const INFO_BOX_MARK_FILL: Color = Color::srgba(1.0, 0.86, 0.22, 0.14);

/// Loading-page Maybraid mark.
pub const LOADING_ICON_SIZE: f32 = 192.0;

/// Seconds per revolution of the loading mark.
pub const LOADING_SPIN_SECS: f32 = 2.5;

/// Width of the loading track.
pub const LOADING_BAR_WIDTH: f32 = 400.0;

/// Thickness of the loading track.
pub const LOADING_BAR_HEIGHT: f32 = 4.0;

/// Gap between logo, bar, and explainer on the loading page.
pub const LOADING_STACK_GAP: f32 = 28.0;

/// Explainer under the loading bar.
pub const LOADING_EXPLAINER_FONT_SIZE: f32 = DESCRIPTION_FONT_SIZE;

/// Collapsible section titles and overlay picker titles.
pub const PANEL_HEADER_FONT_SIZE: f32 = 36.0;

/// Block titles above a field group (`Eyes`, `Hair`) and overlay summary names.
pub const PANEL_BLOCK_FONT_SIZE: f32 = 28.0;

/// Inline field name beside a stepper or swatch row.
pub const PANEL_LABEL_FONT_SIZE: f32 = 22.0;

/// Option rows, tiles, and stepper values — smaller than headers.
pub const PANEL_ITEM_FONT_SIZE: f32 = 20.0;

/// Numeric or cycled value in a stepper.
pub const PANEL_VALUE_FONT_SIZE: f32 = PANEL_ITEM_FONT_SIZE;

/// Subheading above a grouped select list.
pub const PANEL_GROUP_FONT_SIZE: f32 = 16.0;

/// Color chip size in a panel swatch row.
pub const PANEL_SWATCH_SIZE: f32 = 22.0;

/// Floor width so a grid cell does not collapse under long wrap.
pub const PANEL_TILE_MIN_WIDTH: f32 = 72.0;

/// Catalog / select tiles sit three to a row.
pub const PANEL_TILE_COLUMNS: u16 = 3;

/// Shared inventory (Starter Kit + modal) sits four to a row.
pub const INVENTORY_TILE_COLUMNS: u16 = 4;

/// Inventory catalog thumbnail. Smaller than the appearance 72px tiles.
pub const INVENTORY_THUMB_PX: f32 = 52.0;

/// Appearance / picker catalog thumbnail.
pub const CATALOG_THUMB_PX: f32 = 72.0;

/// Kenney Double Xbox **A** — [`maybraid_input::MENU_SELECT_PAD`].
pub const PROMPT_PAD_SELECT: &str = "iconography/kenney/input-prompts/xbox_button_a_outline.png";

/// Kenney Double keyboard **Enter** — [`maybraid_input::MENU_SELECT_KEYS`].
pub const PROMPT_KEY_SELECT: &str = "iconography/kenney/input-prompts/keyboard_enter_outline.png";

/// Kenney Double Xbox **X** — [`maybraid_input::INVENTORY_DROP_PAD`].
pub const PROMPT_PAD_DROP: &str = "iconography/kenney/input-prompts/xbox_button_x_outline.png";

/// Kenney Double keyboard **E** — first of [`maybraid_input::INVENTORY_DROP_KEYS`].
pub const PROMPT_KEY_DROP: &str = "iconography/kenney/input-prompts/keyboard_e_outline.png";

/// Minimum asset-tile height when thumbnails are off.
pub const PANEL_TILE_MIN_HEIGHT: f32 = 44.0;

/// Vertical rhythm inside a panel.
pub const PANEL_ROW_GAP: f32 = 10.0;

/// Extra air above and below a `− / +` stepper so stacked sliders do not touch.
pub const PANEL_STEPPER_PAD_Y: f32 = 10.0;

/// Gap between swatches or tiles in a wrapping row.
pub const PANEL_CHIP_GAP: f32 = 8.0;

/// Inset so tile / swatch focus outlines are not clipped by a scroll pane.
pub const TILE_FOCUS_PAD: f32 = 6.0;

/// Visible scrollbar thickness when a HUD pane overflows.
pub const SCROLLBAR_WIDTH: f32 = 6.0;

/// Track behind the scrollbar thumb.
pub const SCROLLBAR_TRACK: Color = Color::srgba(1.0, 0.86, 0.22, 0.16);

/// Thumb on a HUD scrollbar.
pub const SCROLLBAR_THUMB: Color = Color::srgba(1.0, 0.86, 0.22, 0.72);

#[cfg(test)]
mod tests {
	use bevy::prelude::*;

	use super::{
		character_screen_grid_node, menu_list_panel_node, menu_region_wash, CHARACTER_OVERLAY_GAP,
		CHARACTER_PANEL_WIDTH, MENU_CLEAR, MENU_LIST_RADIUS, MENU_REGION_WASH_ALPHA,
		TEXT_DEEP_BLUE, TEXT_YELLOW,
	};

	#[test]
	fn wash_reuses_home_screen_rgb() {
		let home = MENU_CLEAR.to_srgba();
		let wash = menu_region_wash().to_srgba();
		assert!((home.red - wash.red).abs() < f32::EPSILON);
		assert!((home.green - wash.green).abs() < f32::EPSILON);
		assert!((home.blue - wash.blue).abs() < f32::EPSILON);
		assert!((wash.alpha - MENU_REGION_WASH_ALPHA).abs() < f32::EPSILON);
	}

	#[test]
	fn list_panel_is_rounded_home_blue() {
		let panel = menu_list_panel_node(AlignItems::Center);
		assert_eq!(panel.border_radius, BorderRadius::all(Val::Px(MENU_LIST_RADIUS)));
		assert_eq!(panel.overflow, Overflow::clip());
	}

	#[test]
	fn character_screen_is_a_two_column_grid() {
		let grid = character_screen_grid_node();
		assert_eq!(grid.display, Display::Grid);
		assert_eq!(grid.grid_template_columns.len(), 2);
		assert_eq!(grid.column_gap, Val::Px(CHARACTER_OVERLAY_GAP));
		assert_eq!(grid.grid_template_columns[1], GridTrack::px(CHARACTER_PANEL_WIDTH));
	}

	#[test]
	fn inverted_tag_is_navy_on_yellow() {
		assert_ne!(TEXT_DEEP_BLUE, TEXT_YELLOW);
		assert!(TEXT_DEEP_BLUE.to_srgba().blue > TEXT_DEEP_BLUE.to_srgba().red);
	}
}
