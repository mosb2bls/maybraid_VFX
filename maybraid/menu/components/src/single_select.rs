//! Single-choice HUD menus.

pub mod text_cursor;
pub mod text_menu;

pub use text_cursor::{
	clear_menu_back_consumed, consume_screen_back, emit_screen_back_on_click,
	emit_screen_edit_on_click, screen_back_scene, screen_edit_scene,
	scroll_text_cursor_selection_into_view, sync_screen_edit_cursor, sync_text_cursor_icons,
	ButtonWithSubtext, MenuBackConsumed, MenuObjectiveKind, MenuObjectiveMarker, ScreenBack,
	ScreenBackPressed, ScreenEdit, ScreenEditPressed, TextCursorColumn, TextCursorMenu,
	TextCursorRow, TextCursorScroll, TextCursorSlot,
};
pub use text_menu::{
	apply_text_menu_nav, emit_menu_activate_on_click, emit_menu_activate_on_enter,
	emit_menu_activate_on_nav, emit_menu_focus, emit_menu_step_on_nav, navigate_text_menus,
	republish_menu_activate, select_text_menu_item_on_over, sync_text_menu_item_colors,
	KeyboardMenuNav, MenuActivate, MenuFocus, MenuItemLocked, MenuStep, MenuStepper,
	TextColumnAlign, TextColumnAnchor, TextMenu, TextMenuColumn, TextMenuHeader, TextMenuInputLock,
	TextMenuItem, TextMenuItemLabel,
};
