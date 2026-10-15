//! Mid-size HUD controls for inspector-style panels.
//!
//! These are imperative spawn helpers for dynamic trees (character menus).
//! Full-screen home / loading screens keep using BSN [`Scene`]s.

pub mod button;
pub mod display;
pub mod focus;
pub mod fonts;
pub mod hud_menu;
pub mod row;
pub mod scroll;
pub mod section;
pub mod short_text;
pub mod stepper;
pub mod swatch;
pub mod text;
pub mod tile;

pub use button::{spawn_corner_action, spawn_hud_action, spawn_text_button};
pub use display::menu_display_name;
pub use focus::{paint_focus_tag, spawn_focus_tag, FocusTag};
pub use fonts::HudFonts;
pub use hud_menu::{
	apply_hud_menu_nav, navigate_hud_menus, select_hud_item_on_over, sync_hud_item_focus, HudMenu,
	HudMenuIgnoresLock, HudMenuItem, HudOverlayMenu,
};
pub use row::spawn_labeled_row;
pub use scroll::{
	on_hud_scroll, scroll_hud_selection_into_view, scroll_hud_viewport_on_nav,
	send_hud_scroll_events, spawn_scroll_pane, sync_hud_scrollbars, HudScroll, HudScrollThumb,
	HudScrollTrack, HudScrollViewport,
};
pub use section::{
	spawn_section_header, sync_hud_focus_tags, ActiveOverlayKey, CursorRow, OverlayHeader,
	OverlayHeaderKey,
};
pub use short_text::{
	capture_short_text_input, clear_short_text_dismissed, close_short_text_on_modal_dismiss,
	emit_short_text_cancel_on_click, emit_short_text_pad_on_click, emit_short_text_pad_on_nav,
	emit_short_text_pad_shortcuts, emit_short_text_submit_on_click,
	emit_short_text_submit_on_confirm, emit_short_text_toggle_on_click,
	emit_short_text_toggle_on_enter, emit_short_text_toggle_on_nav,
	observe_short_text_submit_button, restore_short_text_editing, spawn_short_text_button,
	sync_short_text_display, sync_short_text_focus_tags, sync_short_text_ime,
	sync_short_text_modal, sync_short_text_pad_focus, ActiveShortText, ShortTextChange,
	ShortTextField, ShortTextKey, ShortTextModal, ShortTextModalRoot, ShortTextPad,
	ShortTextToggle, ShortTextValue,
};
pub use stepper::spawn_stepper;
pub use swatch::{color_from_hex, spawn_swatch, spawn_swatch_row};
pub use text::{
	spawn_block_label, spawn_group_label, spawn_header_line, spawn_hud_plain, spawn_hud_text,
	spawn_panel_title,
};
pub use tile::{
	spawn_asset_tile, spawn_grid_catalog_tile, spawn_tile_grid, sync_hover_tiles, sync_tile_wrap,
	HoverTile, SlotRank,
};
