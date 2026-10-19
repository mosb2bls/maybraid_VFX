//! Reusable Maybraid HUD widgets.
//!
//! Widgets expose [`Scene`] constructors (`bsn!`). Screens stamp an API-specific
//! choice component onto each pickable row. Selection triggers [`MenuFocus`]
//! and click / Enter trigger [`MenuActivate`], both on the menu. The screen
//! observes those events; [`republish_menu_activate`] copies activate out as a
//! [`Message`].

pub mod controls;
pub mod icons;
pub mod info;
pub mod loading;
pub mod modal;
pub mod single_select;
pub mod spin_reveal;
pub mod theme;

pub use controls::{
	apply_hud_menu_nav, clear_short_text_dismissed, color_from_hex,
	emit_short_text_submit_on_confirm, menu_display_name, navigate_hud_menus, on_hud_scroll,
	restore_short_text_editing, scroll_hud_selection_into_view, scroll_hud_viewport_on_nav,
	select_hud_item_on_over, send_hud_scroll_events, spawn_asset_tile, spawn_block_label,
	spawn_corner_action, spawn_focus_tag, spawn_grid_catalog_tile, spawn_group_label,
	spawn_header_line, spawn_hud_action, spawn_hud_plain, spawn_hud_text,
	spawn_inventory_action_prompts, spawn_labeled_row, spawn_panel_title, spawn_scroll_pane,
	spawn_section_header, spawn_short_text_button, spawn_stepper, spawn_swatch, spawn_swatch_row,
	spawn_text_button, spawn_tile_grid, spawn_tile_grid_columns, sync_hover_tiles,
	sync_hud_focus_tags, sync_hud_item_focus, sync_hud_scrollbars, sync_input_prompt_device,
	sync_inventory_prompt_glyphs, sync_tile_wrap, ActiveOverlayKey, ActiveShortText, CursorRow,
	FocusTag, HoverTile, HudFonts, HudMenu, HudMenuIgnoresLock, HudMenuItem, HudOverlayMenu,
	HudScroll, HudScrollThumb, HudScrollTrack, HudScrollViewport, InputPromptDevice,
	InventoryActionPrompts, InventoryPromptGlyph, InventoryPromptKind, OverlayHeader,
	OverlayHeaderKey, ShortTextChange, ShortTextField, ShortTextKey, ShortTextModal,
	ShortTextModalRoot, ShortTextPad, ShortTextToggle, ShortTextValue, SlotRank,
};
pub use icons::{spin_icons, Icon, SpinningIcon};
pub use info::{
	set_brand_mode_title, set_description_for_menu, set_hint_for_menu, set_info_box_for_menu,
	spawn_hud_text_card, spawn_hud_text_card_label, spawn_menu_objective, BrandModeCorner,
	BrandModeLine, BrandModeTitle, HudTextCard, MenuInfoBox, MenuInfoBoxLabel, MenuObjective,
	TextMenuDescription, TextMenuHint, TextMenuHintLabel, BRAND_NAME, HUD_TEXT_CARD_FACE_PX,
};
pub use loading::{
	set_loading_explainer, set_loading_progress, sync_loading_bar_fill, LoadingBarFill,
	LoadingExplainer, LoadingPanel, LoadingStack,
};
pub use modal::{
	begin_menu_modal_frame, consume_menu_modal_back, consume_screen_back, entity_under_nav_owner,
	hud_menu_under_modal, sync_menu_modal_stack, tick_menu_modal_holds, MenuModal,
	MenuModalDismiss, MenuModalId, MenuModalLayer, MenuModalStack, MenuModalSystems,
};
pub use single_select::{
	apply_text_menu_nav, emit_menu_activate_on_click, emit_menu_activate_on_enter,
	emit_menu_activate_on_nav, emit_menu_focus, emit_menu_step, emit_screen_back_on_click,
	emit_screen_edit_on_click, navigate_text_menus, republish_menu_activate, screen_back_scene,
	screen_edit_scene, scroll_text_cursor_selection_into_view, select_text_menu_item_on_over,
	sync_text_menu_item_colors, ButtonWithSubtext, KeyboardMenuNav, MenuActivate, MenuFocus,
	MenuItemLocked, MenuObjectiveKind, MenuObjectiveMarker, MenuStep, MenuStepRow, MenuStepper,
	ScreenBack, ScreenBackPressed, ScreenEdit, ScreenEditPressed, TextColumnAlign,
	TextColumnAnchor, TextCursorColumn, TextCursorMenu, TextCursorRow, TextCursorScroll, TextMenu,
	TextMenuColumn, TextMenuHeader, TextMenuInputLock, TextMenuItem, TextMenuItemLabel,
};
pub use spin_reveal::{
	SpinRevealCover, SpinRevealFace, SpinRevealPayload, SpinRevealSlot, SpinRevealViewport,
	SPIN_REVEAL_SECS, SPIN_REVEAL_SLOT_SIZE, SPIN_REVEAL_TILE_HEIGHT, SPIN_REVEAL_TILE_WIDTH,
};
pub use theme::{
	character_screen_grid_node, menu_list_panel_node, menu_region_wash, BARLOW_BLACK,
	BARLOW_REGULAR, BARLOW_SEMIBOLD, BONES_ICON, CATALOG_THUMB_PX, CHARACTER_OVERLAY_GAP,
	CHARACTER_PANEL_INSET, CHARACTER_PANEL_WIDTH, DESCRIPTION_PANE_LEFT_PERCENT, HEADER_FONT_SIZE,
	INVENTORY_THUMB_PX, INVENTORY_TILE_COLUMNS, ITEM_FONT_SIZE, LOADING_ICON_SIZE, MAP_ARROW_ICON,
	MAP_GROVE_ICON, MAP_HOUSE_ICON, MAP_MOUNTAIN_ICON, MAP_TOWN_ICON, MAP_TREE_ICON,
	MAP_WATER_ICON, MENU_CLEAR, MENU_LIST_PAD, MENU_LIST_RADIUS, MENU_REGION_WASH_ALPHA,
	NOTO_SANS_REGULAR, OBJECTIVE_MARKER_FONT_SIZE, PANEL_BLOCK_FONT_SIZE, PANEL_GROUP_FONT_SIZE,
	PANEL_HEADER_FONT_SIZE, PANEL_ITEM_FONT_SIZE, PANEL_LABEL_FONT_SIZE, PANEL_ROW_GAP,
	PANEL_TILE_COLUMNS, TEXT_DEEP_BLUE, TEXT_LIGHT_BLUE, TEXT_LIME, TEXT_PURPLE, TEXT_SALMON,
	TEXT_YELLOW, TEXT_YELLOW_FAINT, TEXT_YELLOW_FAINT_FOCUS, TEXT_YELLOW_HOVER,
};

use bevy::prelude::*;
use std::marker::PhantomData;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TextMenuSystems {
	InputLock,
	Navigate,
}

pub struct MenuComponentsPlugin;

impl Plugin for MenuComponentsPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<TextMenuInputLock>()
			.init_resource::<KeyboardMenuNav>()
			.init_resource::<ActiveOverlayKey>()
			.init_resource::<ActiveShortText>()
			.init_resource::<ShortTextModal>()
			.init_resource::<MenuModalStack>()
			.init_resource::<InputPromptDevice>()
			.add_message::<ScreenBackPressed>()
			.add_message::<ScreenEditPressed>()
			.add_message::<MenuModalDismiss>()
			.configure_sets(
				Update,
				(
					TextMenuSystems::InputLock.before(TextMenuSystems::Navigate),
					MenuModalSystems::Sync
						.in_set(TextMenuSystems::InputLock)
						.before(MenuModalSystems::Consume),
					MenuModalSystems::Consume
						.in_set(TextMenuSystems::InputLock)
						.before(MenuModalSystems::Apply),
					MenuModalSystems::Apply.in_set(TextMenuSystems::InputLock),
				),
			)
			.add_systems(
				Update,
				(begin_menu_modal_frame, sync_menu_modal_stack)
					.chain()
					.in_set(MenuModalSystems::Sync),
			)
			.add_systems(
				Update,
				(consume_menu_modal_back, tick_menu_modal_holds)
					.chain()
					.in_set(MenuModalSystems::Consume),
			)
			.add_systems(
				Update,
				controls::close_short_text_on_modal_dismiss.in_set(MenuModalSystems::Apply),
			)
			.add_systems(
				Update,
				controls::emit_short_text_submit_on_confirm
					.after(MenuModalSystems::Sync)
					.before(MenuModalSystems::Consume)
					.in_set(TextMenuSystems::InputLock),
			)
			.add_systems(Last, clear_short_text_dismissed)
			.add_observer(controls::observe_short_text_submit_button)
			.add_observer(select_text_menu_item_on_over)
			.add_observer(select_hud_item_on_over)
			.add_observer(apply_text_menu_nav)
			.add_observer(apply_hud_menu_nav)
			.add_observer(scroll_hud_viewport_on_nav)
			.add_observer(on_hud_scroll)
			.add_observer(emit_screen_back_on_click)
			.add_observer(emit_screen_edit_on_click)
			.add_observer(controls::emit_short_text_toggle_on_click)
			.add_observer(controls::emit_short_text_toggle_on_nav)
			.add_observer(controls::emit_short_text_submit_on_click)
			.add_observer(controls::emit_short_text_pad_on_click)
			.add_observer(controls::emit_short_text_pad_on_nav)
			.add_observer(controls::emit_short_text_cancel_on_click)
			.add_systems(
				Update,
				(
					spin_icons,
					sync_loading_bar_fill,
					send_hud_scroll_events,
					sync_hud_scrollbars,
					(scroll_hud_selection_into_view, scroll_text_cursor_selection_into_view)
						.after(TextMenuSystems::Navigate),
					sync_hud_focus_tags,
					sync_hud_item_focus,
					sync_hover_tiles,
					sync_tile_wrap,
					sync_input_prompt_device,
					sync_inventory_prompt_glyphs,
				),
			)
			.add_systems(
				Update,
				(
					controls::restore_short_text_editing,
					controls::sync_short_text_display,
					controls::sync_short_text_focus_tags,
					controls::sync_short_text_ime,
					controls::short_text::sync_short_text_pad_shift,
					controls::short_text::sync_short_text_pad_focus,
					controls::emit_short_text_pad_shortcuts,
					controls::emit_short_text_toggle_on_enter,
					controls::capture_short_text_input,
					controls::sync_short_text_modal,
				)
					.after(TextMenuSystems::Navigate),
			)
			.add_systems(
				Update,
				(navigate_text_menus, navigate_hud_menus, sync_text_menu_item_colors)
					.chain()
					.in_set(TextMenuSystems::Navigate),
			);
	}
}

/// Per-choice-type wiring: focus / activate emit, and [`Message<E>`] registration.
pub struct TextMenuPlugin<E>(PhantomData<fn() -> E>);

impl<E> Default for TextMenuPlugin<E> {
	fn default() -> Self {
		Self(PhantomData)
	}
}

impl<E> Plugin for TextMenuPlugin<E>
where
	E: Message + Component + Copy + Send + Sync + 'static,
{
	fn build(&self, app: &mut App) {
		if !app.is_plugin_added::<MenuComponentsPlugin>() {
			app.add_plugins(MenuComponentsPlugin);
		}
		app.add_message::<E>()
			.add_observer(emit_menu_activate_on_click::<E>)
			.add_observer(emit_menu_activate_on_nav::<E>)
			.add_observer(emit_menu_step::<E>)
			.add_systems(
				Update,
				(
					emit_menu_focus::<E>.after(TextMenuSystems::Navigate),
					emit_menu_activate_on_enter::<E>,
				),
			);
	}
}
