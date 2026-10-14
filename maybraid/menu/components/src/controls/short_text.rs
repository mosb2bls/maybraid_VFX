//! Short-text button plus a modal line and in-game keypad.
//!
//! Toggle opens the modal. Devices without a system OSK get an A–Z / 0–9
//! keypad. iOS and Android hide that keypad and enable IME so the system
//! keyboard types into the same visible line. Submit emits [`ShortTextChange`].

use bevy::ecs::event::EntityEvent;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::text::{Justify, LineBreak, LineHeight, TextSpan};
use bevy::window::{Ime, PrimaryWindow};

use crate::icons::AnimatedIcon;
use crate::modal::{MenuModal, MenuModalDismiss, MenuModalId, MenuModalLayer, MenuModalStack};
use crate::single_select::{KeyboardMenuNav, TextCursorSlot, TextMenuInputLock};
use crate::theme::{
	HEADER_FONT_SIZE, PANEL_CHIP_GAP, PANEL_CURSOR_ICON_GAP, PANEL_HEADER_CURSOR_ICON_SIZE,
	PANEL_HEADER_FONT_SIZE, PANEL_ITEM_FONT_SIZE, PANEL_ROW_GAP, PANEL_VALUE_FONT_SIZE,
	TEXT_YELLOW, TEXT_YELLOW_FAINT, TEXT_YELLOW_HOVER,
};
use maybraid_input::{MenuNav, MenuNavImpulse, PadButton, VirtualPad};

use super::button::spawn_text_button;
use super::display::menu_display_name;
use super::hud_menu::{HudMenu, HudMenuIgnoresLock, HudMenuItem, HudOverlayMenu};
use super::text::{spawn_cursor_slot_sized, spawn_hud_text};
use super::HudFonts;

/// IR / host key for a short-text field.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortTextKey(pub &'static str);

/// Committed value on the HUD row.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct ShortTextField {
	pub value: String,
	pub max_len: usize,
	pub editing: bool,
}

/// Marker on the HUD row's value `TextSpan`.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextValue;

/// Which field's modal is open.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct ActiveShortText(pub Option<&'static str>);

/// Open edit session. The modal is the only text owner while this is `Some`.
#[derive(Resource, Debug, Default, Clone, PartialEq, Eq)]
pub struct ShortTextModal {
	pub session: Option<ShortTextSession>,
	/// Set on submit/cancel for the rest of the frame so the same Enter/Start
	/// Select cannot reopen the line, and so Escape/B does not also leave the
	/// screen.
	pub dismissed: bool,
}

impl ShortTextModal {
	pub fn is_open(&self) -> bool {
		self.session.is_some()
	}
}

pub fn clear_short_text_dismissed(mut modal: ResMut<ShortTextModal>) {
	modal.dismissed = false;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortTextSession {
	pub key: &'static str,
	pub source: Entity,
	pub value: String,
	pub original: String,
	pub max_len: usize,
	pub shift: bool,
}

/// Root of the fullscreen text modal.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextModalRoot;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextModalValue;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextSubmit;

#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextCancel;

/// In-game keypad root; absent on devices that already have a system OSK.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct ShortTextPad;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortTextPadKey {
	Letter(char),
	Digit(char),
	Space,
	Backspace,
	Shift,
}

#[derive(Component, Debug, Clone, Copy)]
pub(crate) struct ShortTextPadLetter(char);

/// True on platforms where winit can show a system soft keyboard.
pub fn system_osk_available() -> bool {
	cfg!(target_os = "ios") || cfg!(target_os = "android")
}

/// Start or stop editing. The button does not interpret the text.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(propagate, auto_propagate)]
pub struct ShortTextToggle {
	pub entity: Entity,
	pub key: &'static str,
	pub editing: bool,
}

/// Submitted value after confirm.
#[derive(EntityEvent, Debug, Clone)]
#[entity_event(propagate, auto_propagate)]
pub struct ShortTextChange {
	pub entity: Entity,
	pub key: &'static str,
	pub value: String,
}

/// Pickable name row. `extra` is typically [`ShortTextKey`] + [`ShortTextField`] + [`HudMenuItem`].
pub fn spawn_short_text_button(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &'static str,
	value: &str,
	editing: bool,
	justify: JustifyContent,
	extra: impl Bundle,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				width: Val::Percent(100.0),
				padding: UiRect::axes(Val::Px(0.0), Val::Px(4.0)),
				flex_direction: FlexDirection::Row,
				justify_content: justify,
				align_items: AlignItems::Center,
				column_gap: Val::Px(PANEL_CURSOR_ICON_GAP),
				..default()
			},
			BackgroundColor(Color::NONE),
		))
		.with_children(|row| {
			spawn_cursor_slot_sized(row, fonts, editing, PANEL_HEADER_CURSOR_ICON_SIZE);
			spawn_short_text_line(row, fonts, label, value);
		});
}

fn spawn_short_text_line(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	value: &str,
) {
	parent
		.spawn((
			Text::new(menu_display_name(label)),
			fonts.header(PANEL_HEADER_FONT_SIZE),
			TextColor(TEXT_YELLOW),
			TextLayout::new(Justify::Left, LineBreak::NoWrap),
			LineHeight::RelativeToFont(1.0),
			Pickable::IGNORE,
		))
		.with_children(|text| {
			text.spawn((
				ShortTextValue,
				TextSpan::new(row_value_display(value)),
				fonts.body(PANEL_HEADER_FONT_SIZE),
				TextColor(TEXT_YELLOW_FAINT),
				LineHeight::RelativeToFont(1.0),
			));
		});
}

fn row_value_display(value: &str) -> String {
	if value.is_empty() {
		String::from("  —")
	} else {
		format!("  {value}")
	}
}

fn modal_value_display(value: &str) -> String {
	format!("{value}_")
}

pub fn open_short_text_modal(
	entity: Entity,
	key: &'static str,
	field: &mut ShortTextField,
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	stack: &mut MenuModalStack,
	commands: &mut Commands,
) {
	field.editing = true;
	active.0 = Some(key);
	modal.session = Some(ShortTextSession {
		key,
		source: entity,
		value: field.value.clone(),
		original: field.value.clone(),
		max_len: field.max_len,
		shift: false,
	});
	stack.open(MenuModalLayer::SHORT_TEXT);
	commands.trigger(ShortTextToggle { entity, key, editing: true });
}

pub fn cancel_short_text_modal(
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	fields: &mut Query<&mut ShortTextField>,
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<ShortTextModalRoot>>,
	commands: &mut Commands,
) {
	let Some(session) = modal.session.take() else {
		return;
	};
	active.0 = None;
	modal.dismissed = true;
	stack.close(MenuModalId::SHORT_TEXT);
	stack.mark_consumed();
	if let Ok(mut field) = fields.get_mut(session.source) {
		field.editing = false;
	}
	despawn_short_text_roots(roots, commands);
	commands.trigger(ShortTextToggle { entity: session.source, key: session.key, editing: false });
}

pub fn submit_short_text_modal(
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	fields: &mut Query<&mut ShortTextField>,
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<ShortTextModalRoot>>,
	commands: &mut Commands,
) {
	let Some(session) = modal.session.take() else {
		return;
	};
	active.0 = None;
	modal.dismissed = true;
	stack.close(MenuModalId::SHORT_TEXT);
	stack.mark_consumed();
	if let Ok(mut field) = fields.get_mut(session.source) {
		field.value.clone_from(&session.value);
		field.editing = false;
	}
	despawn_short_text_roots(roots, commands);
	commands.trigger(ShortTextChange {
		entity: session.source,
		key: session.key,
		value: session.value,
	});
	commands.trigger(ShortTextToggle { entity: session.source, key: session.key, editing: false });
}

fn despawn_short_text_roots(
	roots: &Query<Entity, With<ShortTextModalRoot>>,
	commands: &mut Commands,
) {
	for entity in roots {
		commands.entity(entity).try_despawn();
	}
}

/// Enter / Start commit while the modal is open, before HUD nav can reuse the edge.
pub fn emit_short_text_submit_on_confirm(
	keyboard: Res<ButtonInput<KeyCode>>,
	pad: Option<Res<VirtualPad>>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if !modal.is_open() {
		return;
	}
	if !keyboard.just_pressed(KeyCode::Enter) && !pad_start_submits(pad.as_deref()) {
		return;
	}
	submit_short_text_modal(
		&mut active,
		&mut modal,
		&mut fields,
		&mut stack,
		&roots,
		&mut commands,
	);
}

pub fn observe_short_text_submit_button(add: On<Add, ShortTextSubmit>, mut commands: Commands) {
	commands.entity(add.entity).observe(on_short_text_submit_click);
}

fn on_short_text_submit_click(
	_: On<Pointer<Click>>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	submit_short_text_modal(
		&mut active,
		&mut modal,
		&mut fields,
		&mut stack,
		&roots,
		&mut commands,
	);
}

pub fn emit_short_text_toggle_on_click(
	click: On<Pointer<Click>>,
	lock: Res<TextMenuInputLock>,
	keys: Query<&ShortTextKey>,
	child_of: Query<&ChildOf>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	let target = pointer_target(&click, &child_of, |entity| keys.contains(entity));
	if modal.is_open() {
		if target.is_some_and(|entity| fields.get(entity).is_ok_and(|field| field.editing)) {
			cancel_short_text_modal(
				&mut active,
				&mut modal,
				&mut fields,
				&mut stack,
				&roots,
				&mut commands,
			);
		}
		return;
	}
	if lock.0 || modal.dismissed {
		return;
	}
	let Some(entity) = target else {
		return;
	};
	let Ok(key) = keys.get(entity) else {
		return;
	};
	let Ok(mut field) = fields.get_mut(entity) else {
		return;
	};
	open_short_text_modal(
		entity,
		key.0,
		&mut field,
		&mut active,
		&mut modal,
		&mut stack,
		&mut commands,
	);
}

#[allow(clippy::too_many_arguments)]
pub fn emit_short_text_toggle_on_enter(
	keyboard: Res<ButtonInput<KeyCode>>,
	keyboard_nav: Res<KeyboardMenuNav>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<Entity, With<super::hud_menu::HudOverlayMenu>>,
	menus: Query<&HudMenu>,
	items: Query<(Entity, &HudMenuItem, &ShortTextKey)>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if !keyboard_nav.is_enabled()
		|| modal.is_open()
		|| modal.dismissed
		|| !keyboard.just_pressed(KeyCode::Enter)
		|| lock.0
		|| !overlay_menus.is_empty()
	{
		return;
	}
	open_selected_short_text(
		&menus,
		&items,
		&mut fields,
		&mut active,
		&mut modal,
		&mut stack,
		&mut commands,
	);
}

#[allow(clippy::too_many_arguments)]
pub fn emit_short_text_toggle_on_nav(
	impulse: On<MenuNavImpulse>,
	lock: Res<TextMenuInputLock>,
	overlay_menus: Query<(), With<super::hud_menu::HudOverlayMenu>>,
	menus: Query<&HudMenu>,
	items: Query<(Entity, &HudMenuItem, &ShortTextKey)>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if lock.0
		|| modal.is_open()
		|| modal.dismissed
		|| impulse.event().nav != MenuNav::Select
		|| !overlay_menus.is_empty()
	{
		return;
	}
	open_selected_short_text(
		&menus,
		&items,
		&mut fields,
		&mut active,
		&mut modal,
		&mut stack,
		&mut commands,
	);
}

fn open_selected_short_text(
	menus: &Query<&HudMenu>,
	items: &Query<(Entity, &HudMenuItem, &ShortTextKey)>,
	fields: &mut Query<&mut ShortTextField>,
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	stack: &mut MenuModalStack,
	commands: &mut Commands,
) {
	for (entity, item, key) in items.iter() {
		let Ok(menu) = menus.get(item.menu) else {
			continue;
		};
		if item.index != menu.selected {
			continue;
		}
		let Ok(mut field) = fields.get_mut(entity) else {
			continue;
		};
		open_short_text_modal(entity, key.0, &mut field, active, modal, stack, commands);
		return;
	}
}

pub fn emit_short_text_submit_on_click(
	mut click: On<Pointer<Click>>,
	submits: Query<(), With<ShortTextSubmit>>,
	child_of: Query<&ChildOf>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if pointer_target(&click, &child_of, |entity| submits.contains(entity)).is_none() {
		return;
	}
	click.propagate(false);
	submit_short_text_modal(
		&mut active,
		&mut modal,
		&mut fields,
		&mut stack,
		&roots,
		&mut commands,
	);
}

pub fn emit_short_text_pad_on_click(
	click: On<Pointer<Click>>,
	keys: Query<&ShortTextPadKey>,
	child_of: Query<&ChildOf>,
	mut modal: ResMut<ShortTextModal>,
) {
	let Some(entity) = pointer_target(&click, &child_of, |entity| keys.contains(entity)) else {
		return;
	};
	let Ok(key) = keys.get(entity) else {
		return;
	};
	apply_short_text_pad_key(*key, &mut modal);
}

pub fn emit_short_text_pad_on_nav(
	mut impulse: On<MenuNavImpulse>,
	pad_state: Option<Res<VirtualPad>>,
	pads: Query<(Entity, &HudMenu), With<ShortTextPad>>,
	keys: Query<(Entity, &HudMenuItem, Option<&ShortTextPadKey>, Option<&ShortTextSubmit>)>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	let Ok((pad, menu)) = pads.get(impulse.entity) else {
		return;
	};
	if impulse.event().nav != MenuNav::Select {
		return;
	}
	impulse.propagate(false);
	if pad_start_submits(pad_state.as_deref()) {
		submit_short_text_modal(
			&mut active,
			&mut modal,
			&mut fields,
			&mut stack,
			&roots,
			&mut commands,
		);
	} else {
		activate_selected_pad_item(
			pad,
			menu,
			&keys,
			&mut fields,
			&mut active,
			&mut modal,
			&mut stack,
			&roots,
			&mut commands,
		);
	}
}

/// Left-stick click toggles caps while the in-game keypad is open.
pub fn emit_short_text_pad_shortcuts(
	pad: Option<Res<VirtualPad>>,
	mut modal: ResMut<ShortTextModal>,
) {
	let Some(pad) = pad else {
		return;
	};
	if !modal.is_open() || !pad.just_pressed(PadButton::StickClickMove) {
		return;
	}
	apply_short_text_pad_key(ShortTextPadKey::Shift, &mut modal);
}

fn pad_start_submits(pad: Option<&VirtualPad>) -> bool {
	pad.is_some_and(|pad| pad.just_pressed(PadButton::Start))
}

fn apply_short_text_pad_key(key: ShortTextPadKey, modal: &mut ShortTextModal) {
	let Some(session) = modal.session.as_mut() else {
		return;
	};
	match key {
		ShortTextPadKey::Shift => session.shift = !session.shift,
		ShortTextPadKey::Backspace => {
			session.value.pop();
		}
		ShortTextPadKey::Space => {
			push_short_text_char(&mut session.value, session.max_len, ' ');
		}
		ShortTextPadKey::Digit(ch) => {
			push_short_text_char(&mut session.value, session.max_len, ch);
		}
		ShortTextPadKey::Letter(ch) => {
			let ch = if session.shift { ch.to_ascii_uppercase() } else { ch.to_ascii_lowercase() };
			push_short_text_char(&mut session.value, session.max_len, ch);
		}
	}
}

fn activate_selected_pad_item(
	pad: Entity,
	menu: &HudMenu,
	keys: &Query<(Entity, &HudMenuItem, Option<&ShortTextPadKey>, Option<&ShortTextSubmit>)>,
	fields: &mut Query<&mut ShortTextField>,
	active: &mut ActiveShortText,
	modal: &mut ShortTextModal,
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<ShortTextModalRoot>>,
	commands: &mut Commands,
) {
	for (_, item, key, submit) in keys.iter() {
		if item.menu != pad || item.index != menu.selected {
			continue;
		}
		if let Some(key) = key {
			apply_short_text_pad_key(*key, modal);
		}
		if submit.is_some() {
			submit_short_text_modal(active, modal, fields, stack, roots, commands);
		}
		return;
	}
}

pub fn emit_short_text_cancel_on_click(
	mut click: On<Pointer<Click>>,
	cancels: Query<(), With<ShortTextCancel>>,
	child_of: Query<&ChildOf>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if pointer_target(&click, &child_of, |entity| cancels.contains(entity)).is_none() {
		return;
	}
	click.propagate(false);
	cancel_short_text_modal(
		&mut active,
		&mut modal,
		&mut fields,
		&mut stack,
		&roots,
		&mut commands,
	);
}

pub fn capture_short_text_input(
	mut reader: MessageReader<KeyboardInput>,
	mut ime: MessageReader<Ime>,
	keyboard: Res<ButtonInput<KeyCode>>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if !modal.is_open() {
		reader.clear();
		ime.clear();
		return;
	}

	if keyboard.just_pressed(KeyCode::Escape) {
		cancel_short_text_modal(
			&mut active,
			&mut modal,
			&mut fields,
			&mut stack,
			&roots,
			&mut commands,
		);
		reader.clear();
		ime.clear();
		return;
	}
	if keyboard.just_pressed(KeyCode::Enter) {
		submit_short_text_modal(
			&mut active,
			&mut modal,
			&mut fields,
			&mut stack,
			&roots,
			&mut commands,
		);
		reader.clear();
		ime.clear();
		return;
	}

	let Some(session) = modal.session.as_mut() else {
		return;
	};
	if keyboard.just_pressed(KeyCode::Backspace) {
		session.value.pop();
	}
	for ev in reader.read() {
		if ev.state != ButtonState::Pressed || ev.repeat {
			continue;
		}
		let Some(text) = ev.text.as_ref() else {
			continue;
		};
		for ch in text.chars() {
			if ch == '\r' || ch == '\n' {
				continue;
			}
			push_short_text_char(&mut session.value, session.max_len, ch);
		}
	}
	for ev in ime.read() {
		if let Ime::Commit { value, .. } = ev {
			for ch in value.chars() {
				if ch == '\r' || ch == '\n' {
					continue;
				}
				push_short_text_char(&mut session.value, session.max_len, ch);
			}
		}
	}
}

pub fn close_short_text_on_modal_dismiss(
	mut dismiss: MessageReader<MenuModalDismiss>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut fields: Query<&mut ShortTextField>,
	mut active: ResMut<ActiveShortText>,
	mut modal: ResMut<ShortTextModal>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if !dismiss.read().any(|event| event.is(MenuModalId::SHORT_TEXT)) {
		return;
	}
	cancel_short_text_modal(
		&mut active,
		&mut modal,
		&mut fields,
		&mut stack,
		&roots,
		&mut commands,
	);
}

pub fn sync_short_text_modal(
	mut commands: Commands,
	asset_server: Res<AssetServer>,
	modal: Res<ShortTextModal>,
	roots: Query<Entity, With<ShortTextModalRoot>>,
	mut values: Query<&mut Text, With<ShortTextModalValue>>,
) {
	if !modal.is_open() {
		despawn_short_text_roots(&roots, &mut commands);
		return;
	}
	let session = modal.session.as_ref().expect("open");
	if roots.is_empty() {
		spawn_short_text_modal(&mut commands, &HudFonts::load(asset_server.as_ref()), session);
		return;
	}
	let display = modal_value_display(&session.value);
	for mut text in &mut values {
		if text.0 != display {
			text.0 = display.clone();
		}
	}
}

pub(crate) fn sync_short_text_pad_shift(
	modal: Res<ShortTextModal>,
	mut letters: Query<(&ShortTextPadLetter, &mut Text)>,
) {
	let shift = modal.session.as_ref().is_some_and(|session| session.shift);
	for (letter, mut text) in &mut letters {
		let display = pad_letter_label(letter.0, shift);
		if text.0 != display {
			text.0 = display;
		}
	}
}

fn spawn_short_text_modal(commands: &mut Commands, fonts: &HudFonts, session: &ShortTextSession) {
	let show_pad = !system_osk_available();
	commands
		.spawn((
			ShortTextModalRoot,
			MenuModal::SHORT_TEXT,
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				padding: UiRect::bottom(Val::Percent(if show_pad { 8.0 } else { 22.0 })),
				..default()
			},
			GlobalZIndex(100),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Button,
				ShortTextCancel,
				Node {
					position_type: PositionType::Absolute,
					left: Val::Px(0.0),
					top: Val::Px(0.0),
					width: Val::Percent(100.0),
					height: Val::Percent(100.0),
					..default()
				},
				BackgroundColor(Color::srgba(0.04, 0.05, 0.07, 0.72)),
			));
			root.spawn((
				Node {
					width: Val::Px(720.0),
					max_width: Val::Percent(92.0),
					flex_direction: FlexDirection::Column,
					align_items: AlignItems::FlexStart,
					row_gap: Val::Px(PANEL_ROW_GAP),
					..default()
				},
				Pickable::default(),
			))
			.with_children(|card| {
				spawn_hud_text(
					card,
					fonts.header(HEADER_FONT_SIZE * 0.42),
					session.key,
					TEXT_YELLOW,
					Justify::Left,
				);
				card.spawn((
					ShortTextModalValue,
					Text::new(modal_value_display(&session.value)),
					fonts.header(HEADER_FONT_SIZE * 0.55),
					TextColor(TEXT_YELLOW),
					TextLayout::new(Justify::Left, LineBreak::NoWrap),
					LineHeight::RelativeToFont(1.0),
					Pickable::IGNORE,
				));
				if show_pad {
					spawn_short_text_pad(card, fonts, session.shift);
				} else {
					spawn_text_button(card, fonts, "submit", ShortTextSubmit);
					spawn_hud_text(
						card,
						fonts.body(PANEL_ITEM_FONT_SIZE),
						"type to enter",
						TEXT_YELLOW_FAINT,
						Justify::Left,
					);
				}
			});
		});
}

const PAD_LETTERS: [&str; 3] = ["QWERTYUIOP", "ASDFGHJKL", "ZXCVBNM"];
const PAD_DIGITS: &str = "1234567890";
const PAD_KEY_SIZE: f32 = 40.0;
const PAD_WIDE_KEY: f32 = 88.0;

fn spawn_short_text_pad(parent: &mut ChildSpawnerCommands, fonts: &HudFonts, shift: bool) {
	let mut index = 0;
	let mut pad = parent.spawn((
		ShortTextPad,
		HudOverlayMenu,
		HudMenuIgnoresLock,
		HudMenu::new(0),
		Node {
			width: Val::Percent(100.0),
			flex_direction: FlexDirection::Column,
			align_items: AlignItems::FlexStart,
			row_gap: Val::Px(PANEL_CHIP_GAP),
			margin: UiRect::top(Val::Px(PANEL_ROW_GAP)),
			..default()
		},
		Pickable::IGNORE,
	));
	let pad_id = pad.id();
	pad.with_children(|pad| {
		for row in PAD_LETTERS {
			spawn_pad_letter_row(pad, fonts, row, shift, pad_id, &mut index);
		}
		spawn_pad_digit_row(pad, fonts, pad_id, &mut index);
		spawn_pad_action_row(pad, fonts, pad_id, &mut index);
	});
	pad.insert(HudMenu::new(index));
}

fn spawn_pad_letter_row(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	letters: &str,
	shift: bool,
	pad: Entity,
	index: &mut usize,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				flex_wrap: FlexWrap::Wrap,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for ch in letters.chars() {
				spawn_pad_key(
					row,
					fonts,
					&pad_letter_label(ch, shift),
					PAD_KEY_SIZE,
					(ShortTextPadKey::Letter(ch), HudMenuItem { index: *index, menu: pad }),
					Some(ch),
				);
				*index += 1;
			}
		});
}

fn spawn_pad_digit_row(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	pad: Entity,
	index: &mut usize,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				flex_wrap: FlexWrap::Wrap,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			for ch in PAD_DIGITS.chars() {
				spawn_pad_key(
					row,
					fonts,
					&ch.to_string(),
					PAD_KEY_SIZE,
					(ShortTextPadKey::Digit(ch), HudMenuItem { index: *index, menu: pad }),
					None,
				);
				*index += 1;
			}
		});
}

fn spawn_pad_action_row(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	pad: Entity,
	index: &mut usize,
) {
	parent
		.spawn((
			Node {
				flex_direction: FlexDirection::Row,
				column_gap: Val::Px(PANEL_CHIP_GAP),
				flex_wrap: FlexWrap::Wrap,
				align_items: AlignItems::Center,
				..default()
			},
			Pickable::IGNORE,
		))
		.with_children(|row| {
			spawn_pad_key(
				row,
				fonts,
				"shift",
				PAD_WIDE_KEY,
				(ShortTextPadKey::Shift, HudMenuItem { index: *index, menu: pad }),
				None,
			);
			*index += 1;
			spawn_pad_key(
				row,
				fonts,
				"space",
				PAD_WIDE_KEY * 2.0,
				(ShortTextPadKey::Space, HudMenuItem { index: *index, menu: pad }),
				None,
			);
			*index += 1;
			spawn_pad_key(
				row,
				fonts,
				"back",
				PAD_WIDE_KEY,
				(ShortTextPadKey::Backspace, HudMenuItem { index: *index, menu: pad }),
				None,
			);
			*index += 1;
			spawn_pad_key(
				row,
				fonts,
				"submit",
				PAD_WIDE_KEY,
				(ShortTextSubmit, HudMenuItem { index: *index, menu: pad }),
				None,
			);
			*index += 1;
		});
}

fn spawn_pad_key(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	label: &str,
	min_width: f32,
	extra: impl Bundle,
	letter: Option<char>,
) {
	parent
		.spawn((
			Button,
			extra,
			Node {
				min_width: Val::Px(min_width),
				min_height: Val::Px(PAD_KEY_SIZE),
				padding: UiRect::axes(Val::Px(6.0), Val::Px(4.0)),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				..default()
			},
			BackgroundColor(Color::NONE),
			Outline::new(Val::Px(2.0), Val::Px(1.0), Color::NONE),
		))
		.with_children(|button| {
			let mut text = button.spawn((
				Text::new(label),
				fonts.item(PANEL_VALUE_FONT_SIZE),
				TextColor(TEXT_YELLOW),
				TextLayout::new(Justify::Center, LineBreak::NoWrap),
				LineHeight::RelativeToFont(1.0),
				Pickable::IGNORE,
			));
			if let Some(ch) = letter {
				text.insert(ShortTextPadLetter(ch));
			}
		});
}

fn pad_letter_label(letter: char, shift: bool) -> String {
	if shift { letter.to_ascii_uppercase() } else { letter.to_ascii_lowercase() }.to_string()
}

fn pointer_target(
	click: &On<Pointer<Click>>,
	child_of: &Query<&ChildOf>,
	matches: impl Fn(Entity) -> bool,
) -> Option<Entity> {
	let mut current = Some(click.original_event_target());
	while let Some(entity) = current {
		if matches(entity) {
			return Some(entity);
		}
		current = child_of.get(entity).ok().map(ChildOf::parent);
	}
	None
}

pub fn sync_short_text_display(
	fields: Query<(Entity, &ShortTextField), Changed<ShortTextField>>,
	children: Query<&Children>,
	mut spans: Query<&mut TextSpan, With<ShortTextValue>>,
) {
	for (entity, field) in &fields {
		refresh_row_span(entity, &field.value, &children, &mut spans);
	}
}

fn push_short_text_char(value: &mut String, max_len: usize, ch: char) -> bool {
	if !is_short_text_char(ch) {
		return false;
	}
	if value.chars().count() >= max_len {
		return false;
	}
	value.push(ch);
	true
}

fn is_short_text_char(ch: char) -> bool {
	ch.is_ascii_alphabetic() || ch.is_ascii_digit() || ch == ' '
}

fn refresh_row_span(
	entity: Entity,
	value: &str,
	children: &Query<&Children>,
	spans: &mut Query<&mut TextSpan, With<ShortTextValue>>,
) {
	let Ok(row_children) = children.get(entity) else {
		return;
	};
	for child in row_children {
		if let Ok(mut span) = spans.get_mut(*child) {
			span.0 = row_value_display(value);
			return;
		}
		let Ok(nested) = children.get(*child) else {
			continue;
		};
		for grandchild in nested {
			if let Ok(mut span) = spans.get_mut(*grandchild) {
				span.0 = row_value_display(value);
				return;
			}
		}
	}
}

pub fn restore_short_text_editing(
	active: Res<ActiveShortText>,
	modal: Res<ShortTextModal>,
	mut fields: Query<(&ShortTextKey, &mut ShortTextField)>,
) {
	for (key, mut field) in &mut fields {
		let editing = modal.is_open() && active.0 == Some(key.0);
		if field.editing != editing {
			field.editing = editing;
		}
	}
}

pub fn sync_short_text_pad_focus(
	pads: Query<(Entity, &HudMenu), With<ShortTextPad>>,
	items: Query<(Entity, &HudMenuItem)>,
	mut outlines: Query<&mut Outline>,
	mut colors: Query<&mut TextColor>,
	children: Query<&Children>,
) {
	for (pad, menu) in &pads {
		for (entity, item) in &items {
			if item.menu != pad {
				continue;
			}
			let focused = item.index == menu.selected;
			if let Ok(mut outline) = outlines.get_mut(entity) {
				outline.color = if focused { TEXT_YELLOW } else { Color::NONE };
			}
			let Ok(row_children) = children.get(entity) else {
				continue;
			};
			for child in row_children {
				if let Ok(mut color) = colors.get_mut(*child) {
					color.0 = if focused { TEXT_YELLOW_HOVER } else { TEXT_YELLOW };
				}
			}
		}
	}
}

pub fn sync_short_text_cursors(
	fields: Query<(&ShortTextField, Option<&HudMenuItem>, &Children)>,
	menus: Query<&HudMenu>,
	slots: Query<(), With<TextCursorSlot>>,
	children: Query<&Children>,
	mut icons: Query<&mut Visibility, With<AnimatedIcon>>,
) {
	for (field, item, row_children) in &fields {
		let focused = item
			.is_some_and(|item| menus.get(item.menu).is_ok_and(|menu| menu.selected == item.index));
		let show = focused || field.editing;
		for child in row_children {
			if slots.get(*child).is_err() {
				continue;
			}
			let Ok(slot_children) = children.get(*child) else {
				continue;
			};
			for icon_entity in slot_children {
				if let Ok(mut visibility) = icons.get_mut(*icon_entity) {
					*visibility = if show { Visibility::Inherited } else { Visibility::Hidden };
				}
			}
		}
	}
}

pub fn sync_short_text_ime(
	modal: Res<ShortTextModal>,
	mut windows: Query<&mut Window, With<PrimaryWindow>>,
) {
	let editing = modal.is_open() && system_osk_available();
	let Ok(mut window) = windows.single_mut() else {
		return;
	};
	if window.ime_enabled != editing {
		window.ime_enabled = editing;
	}
}

#[cfg(test)]
mod tests {
	use super::{
		apply_short_text_pad_key, cancel_short_text_modal, close_short_text_on_modal_dismiss,
		is_short_text_char, pad_letter_label, push_short_text_char, submit_short_text_modal,
		ActiveShortText, ShortTextField, ShortTextModal, ShortTextModalRoot, ShortTextPadKey,
		ShortTextSession,
	};
	use crate::modal::{
		consume_menu_modal_back, consume_screen_back, MenuModalDismiss, MenuModalId,
		MenuModalLayer, MenuModalStack,
	};
	use crate::single_select::text_cursor::ScreenBackPressed;
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use maybraid_input::{MenuNav, MenuNavPad};

	#[test]
	fn pad_letters_follow_shift() {
		assert_eq!(pad_letter_label('A', false), "a");
		assert_eq!(pad_letter_label('A', true), "A");
	}

	#[test]
	fn allows_letters_digits_space_and_caps_max_len() {
		let mut value = String::new();
		assert!(push_short_text_char(&mut value, 4, 'A'));
		assert!(push_short_text_char(&mut value, 4, 'b'));
		assert!(push_short_text_char(&mut value, 4, '3'));
		assert!(!push_short_text_char(&mut value, 4, '-'));
		assert!(!push_short_text_char(&mut value, 4, '\''));
		assert!(push_short_text_char(&mut value, 4, ' '));
		assert!(!push_short_text_char(&mut value, 4, 'x'));
		assert_eq!(value, "Ab3 ");
		assert!(is_short_text_char('Z'));
		assert!(is_short_text_char('0'));
		assert!(!is_short_text_char('!'));
	}

	#[test]
	fn pad_shift_toggles_caps() {
		let mut modal = ShortTextModal {
			session: Some(ShortTextSession {
				key: "name",
				source: Entity::PLACEHOLDER,
				value: String::new(),
				original: String::new(),
				max_len: 8,
				shift: false,
			}),
			dismissed: false,
		};
		apply_short_text_pad_key(ShortTextPadKey::Shift, &mut modal);
		assert!(modal.session.as_ref().is_some_and(|session| session.shift));
		apply_short_text_pad_key(ShortTextPadKey::Shift, &mut modal);
		assert!(modal.session.as_ref().is_some_and(|session| !session.shift));
	}

	fn open_name_session(world: &mut World) -> Entity {
		world.init_resource::<ActiveShortText>();
		world.init_resource::<ShortTextModal>();
		world.init_resource::<MenuModalStack>();
		let source = world
			.spawn(ShortTextField { value: String::from("Ada"), max_len: 16, editing: true })
			.id();
		world.resource_mut::<ShortTextModal>().session = Some(ShortTextSession {
			key: "Name",
			source,
			value: String::from("Mist"),
			original: String::from("Ada"),
			max_len: 16,
			shift: false,
		});
		world.resource_mut::<ActiveShortText>().0 = Some("Name");
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		source
	}

	#[test]
	fn submit_dismisses_and_keeps_the_committed_value() {
		let mut world = World::new();
		let source = open_name_session(&mut world);
		world
			.run_system_once(
				|mut active: ResMut<ActiveShortText>,
				 mut modal: ResMut<ShortTextModal>,
				 mut fields: Query<&mut ShortTextField>,
				 mut stack: ResMut<MenuModalStack>,
				 roots: Query<Entity, With<ShortTextModalRoot>>,
				 mut commands: Commands| {
					submit_short_text_modal(
						&mut active,
						&mut modal,
						&mut fields,
						&mut stack,
						&roots,
						&mut commands,
					);
				},
			)
			.expect("submit");
		let modal = world.resource::<ShortTextModal>();
		assert!(!modal.is_open());
		assert!(modal.dismissed);
		assert!(world.resource::<ActiveShortText>().0.is_none());
		assert!(!world.resource::<MenuModalStack>().contains(MenuModalId::SHORT_TEXT));
		assert!(world.resource::<MenuModalStack>().consumed());
		let field = world.get::<ShortTextField>(source).expect("field");
		assert_eq!(field.value, "Mist");
		assert!(!field.editing);
	}

	#[test]
	fn cancel_dismisses_and_consumes_back() {
		let mut world = World::new();
		let source = open_name_session(&mut world);
		world
			.run_system_once(
				|mut active: ResMut<ActiveShortText>,
				 mut modal: ResMut<ShortTextModal>,
				 mut fields: Query<&mut ShortTextField>,
				 mut stack: ResMut<MenuModalStack>,
				 roots: Query<Entity, With<ShortTextModalRoot>>,
				 mut commands: Commands| {
					cancel_short_text_modal(
						&mut active,
						&mut modal,
						&mut fields,
						&mut stack,
						&roots,
						&mut commands,
					);
				},
			)
			.expect("cancel");
		let modal = world.resource::<ShortTextModal>();
		assert!(!modal.is_open());
		assert!(modal.dismissed);
		assert!(world.resource::<MenuModalStack>().consumed());
		assert!(world.resource::<MenuModalStack>().blocks_screen_back());
		let field = world.get::<ShortTextField>(source).expect("field");
		assert_eq!(field.value, "Ada");
		assert!(!field.editing);
	}

	fn press_back_on_open_keyboard() -> World {
		let mut world = World::new();
		open_name_session(&mut world);
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.init_resource::<Messages<ScreenBackPressed>>();
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_short_text_on_modal_dismiss).expect("close");
		world
	}

	#[test]
	fn back_closes_the_keyboard_and_does_not_leave_the_screen() {
		let mut world = press_back_on_open_keyboard();
		assert!(!world.resource::<ShortTextModal>().is_open());
		assert!(world.resource::<MenuModalStack>().blocks_screen_back());
		let left = world
			.run_system_once(
				|nav: Res<MenuNavPad>,
				 stack: Res<MenuModalStack>,
				 mut backs: MessageReader<ScreenBackPressed>| {
					consume_screen_back(&nav, &stack, &mut backs)
				},
			)
			.expect("screen back");
		assert!(!left);
	}

	#[test]
	fn escape_closes_the_keyboard_without_a_keypad_entity() {
		let mut world = World::new();
		open_name_session(&mut world);
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.init_resource::<Messages<ScreenBackPressed>>();
		world.resource_mut::<ButtonInput<KeyCode>>().press(KeyCode::Escape);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_short_text_on_modal_dismiss).expect("close");
		assert!(!world.resource::<ShortTextModal>().is_open());
		assert!(world.resource::<MenuModalStack>().blocks_screen_back());
	}

	#[test]
	fn backspace_edits_and_does_not_close_the_keyboard() {
		let mut modal = ShortTextModal {
			session: Some(ShortTextSession {
				key: "Name",
				source: Entity::PLACEHOLDER,
				value: String::from("Mist"),
				original: String::from("Ada"),
				max_len: 16,
				shift: false,
			}),
			dismissed: false,
		};
		apply_short_text_pad_key(ShortTextPadKey::Backspace, &mut modal);
		assert_eq!(modal.session.as_ref().map(|session| session.value.as_str()), Some("Mis"));
		assert!(modal.is_open());
	}
}
