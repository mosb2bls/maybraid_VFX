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

/// Mark beside a text-cursor row (sits against [`ITEM_FONT_SIZE`] labels).
pub const CURSOR_ICON_SIZE: f32 = 36.0;

/// Gap between the cursor mark and the row label.
pub const CURSOR_ICON_GAP: f32 = 12.0;

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

/// Blink period for animated marks, in seconds.
pub const ICON_BLINK_SECS: f32 = 1.2;

/// Animated-mark alpha at the dim end of the blink.
pub const ICON_BLINK_ALPHA_MIN: f32 = 0.18;

/// Animated-mark alpha at the bright end of the blink.
pub const ICON_BLINK_ALPHA_MAX: f32 = 0.92;

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

/// Cursor mark beside a field or option row.
pub const PANEL_CURSOR_ICON_SIZE: f32 = 20.0;

/// Cursor mark beside a section or overlay-summary header.
pub const PANEL_HEADER_CURSOR_ICON_SIZE: f32 = 28.0;

/// Gap between the panel cursor mark and the row label.
pub const PANEL_CURSOR_ICON_GAP: f32 = 10.0;

/// Color chip size in a panel swatch row.
pub const PANEL_SWATCH_SIZE: f32 = 22.0;

/// Floor width so a grid cell does not collapse under long wrap.
pub const PANEL_TILE_MIN_WIDTH: f32 = 72.0;

/// Catalog / select tiles sit three to a row.
pub const PANEL_TILE_COLUMNS: u16 = 3;

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
