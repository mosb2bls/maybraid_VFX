//! Start / Back confirmations on the character editor main screen.

use bevy::prelude::*;
use bevy::text::Justify;
use character_persist::SaveRoot;
use maybraid_character_ui_menu_renderer::OverlaySelectRoot;
use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad, PadButton, VirtualPad};
use menu_components::theme::{HEADER_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW};
use menu_components::{
	clear_menu_back_consumed, emit_short_text_submit_on_confirm, spawn_hud_action, spawn_hud_text,
	ActiveOverlayKey, HudFonts, HudMenu, HudMenuItem, HudOverlayMenu, MenuBackConsumed,
	ScreenBackPressed, ShortTextModal, ShortTextModalRoot, TextMenuInputLock, TextMenuSystems,
};

use crate::character::{CharacterMenuState, CharacterScreen};
use crate::session::{
	leave_character_editor, save_editing_character, set_active_character, CharacterEditorReturn,
	EditingCharacter,
};

/// Save-and-exit or discard-and-exit while the editor HUD is up.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterLeaveKind {
	Save,
	/// The last Save confirm could not write; confirming retries.
	SaveFailed,
	Discard,
}

impl CharacterLeaveKind {
	pub fn saves(self) -> bool {
		matches!(self, Self::Save | Self::SaveFailed)
	}
}

/// Open confirm card on the character editor. Empty while the HUD is idle.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CharacterLeavePrompt {
	pub kind: Option<CharacterLeaveKind>,
	/// Ignore the Start/Back that opened the card so it cannot confirm or leave.
	pub opened_this_frame: bool,
}

/// Character create/edit is covered by a name modal, catalog, or leave card.
/// Start/Back that went down in that mode do not bubble onto the HUD after
/// the popup closes — the same press must not open a leave prompt.
#[derive(Resource, Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct CharacterModalMode {
	pub active: bool,
	start_from_modal: bool,
	back_from_modal: bool,
}

impl CharacterModalMode {
	pub fn blocks_leave(&self) -> bool {
		self.active || self.start_from_modal || self.back_from_modal
	}

	pub fn tick(&mut self, active: bool, start_down: bool, back_down: bool) {
		self.active = active;
		self.start_from_modal = latch_modal_button(self.start_from_modal, active, start_down);
		self.back_from_modal = latch_modal_button(self.back_from_modal, active, back_down);
	}
}

/// Keep a modal's Start/Back until release so a late `just_pressed` cannot
/// land on the HUD after submit closes the popup.
pub fn latch_modal_button(armed: bool, modal: bool, down: bool) -> bool {
	if !down {
		false
	} else if modal {
		true
	} else {
		armed
	}
}

impl CharacterLeavePrompt {
	pub fn is_open(&self) -> bool {
		self.kind.is_some()
	}

	pub fn open(&mut self, kind: CharacterLeaveKind) {
		self.kind = Some(kind);
		self.opened_this_frame = true;
	}
}

#[derive(Component, Debug, Default, Clone, Copy)]
struct CharacterLeavePromptRoot;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
struct CharacterLeavePromptKind(CharacterLeaveKind);

#[derive(Component, Debug, Default, Clone, Copy)]
struct CharacterLeavePromptMenu;

#[derive(Component, Debug, Default, Clone, Copy)]
struct LeavePromptConfirm;

#[derive(Component, Debug, Default, Clone, Copy)]
struct LeavePromptCancel;

pub struct CharacterLeavePromptPlugin;

impl Plugin for CharacterLeavePromptPlugin {
	fn build(&self, app: &mut App) {
		app.init_resource::<CharacterLeavePrompt>()
			.init_resource::<CharacterModalMode>()
			.add_observer(on_leave_prompt_click)
			.add_observer(on_leave_prompt_nav)
			.add_systems(
				Update,
				(refresh_character_modal_mode, intercept_character_leave_edges)
					.chain()
					.in_set(TextMenuSystems::InputLock)
					.after(clear_menu_back_consumed)
					.after(emit_short_text_submit_on_confirm),
			)
			.add_systems(
				Update,
				(close_leave_prompt_without_character, sync_character_leave_prompt)
					.chain()
					.after(TextMenuSystems::Navigate),
			);
	}
}

/// Main-screen Start saves and leaves; Back discards. Any catalog, name
/// modal, input lock, or already-open prompt keeps its own edges.
pub fn character_leave_edge(
	on_main_screen: bool,
	popup_hold: bool,
	prompt_open: bool,
	start: bool,
	back: bool,
) -> Option<CharacterLeaveKind> {
	if !on_main_screen || popup_hold || prompt_open {
		return None;
	}
	if start {
		return Some(CharacterLeaveKind::Save);
	}
	if back {
		return Some(CharacterLeaveKind::Discard);
	}
	None
}

fn character_screen_modal(
	lock: bool,
	overlay: bool,
	modal_open: bool,
	modal_dismissed: bool,
	short_text_root: bool,
	catalog: bool,
	overlay_menu: bool,
	leave_prompt: bool,
) -> bool {
	lock || overlay
		|| modal_open
		|| modal_dismissed
		|| short_text_root
		|| catalog
		|| overlay_menu
		|| leave_prompt
}

fn refresh_character_modal_mode(
	mut mode: ResMut<CharacterModalMode>,
	prompt: Res<CharacterLeavePrompt>,
	pad: Option<Res<VirtualPad>>,
	nav: Res<MenuNavPad>,
	lock: Res<TextMenuInputLock>,
	overlay: Res<ActiveOverlayKey>,
	modal: Res<ShortTextModal>,
	screens: Query<(), With<CharacterScreen>>,
	short_text_roots: Query<(), With<ShortTextModalRoot>>,
	catalogs: Query<(), With<OverlaySelectRoot>>,
	overlay_menus: Query<(), (With<HudOverlayMenu>, Without<CharacterLeavePromptMenu>)>,
) {
	let active = !screens.is_empty()
		&& character_screen_modal(
			lock.0,
			overlay.0.is_some(),
			modal.is_open(),
			modal.dismissed,
			!short_text_roots.is_empty(),
			!catalogs.is_empty(),
			!overlay_menus.is_empty(),
			prompt.is_open(),
		);
	let start_down = pad.as_deref().is_some_and(|pad| pad.pressed(PadButton::Start));
	let back_down = pad.as_deref().is_some_and(|pad| pad.pressed(PadButton::B))
		|| nav.just_pressed(MenuNav::Back);
	mode.tick(active, start_down, back_down);
}

fn intercept_character_leave_edges(
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut consumed: ResMut<MenuBackConsumed>,
	mut nav: ResMut<MenuNavPad>,
	pad: Option<Res<VirtualPad>>,
	mode: Res<CharacterModalMode>,
	screens: Query<(), With<CharacterScreen>>,
	mut backs: MessageReader<ScreenBackPressed>,
) {
	prompt.opened_this_frame = false;
	let back_click = backs.read().next().is_some();
	let Some(kind) = character_leave_edge(
		!screens.is_empty(),
		mode.blocks_leave(),
		false,
		pad.is_some_and(|pad| pad.just_pressed(PadButton::Start)),
		back_click || nav.just_pressed(MenuNav::Back),
	) else {
		return;
	};
	prompt.open(kind);
	match kind {
		CharacterLeaveKind::Save | CharacterLeaveKind::SaveFailed => {
			nav.events.retain(|event| *event != MenuNav::Select);
		}
		CharacterLeaveKind::Discard => {
			consumed.0 = true;
			nav.events.retain(|event| *event != MenuNav::Back);
		}
	}
}

fn close_leave_prompt_without_character(
	screens: Query<(), With<CharacterScreen>>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	roots: Query<Entity, With<CharacterLeavePromptRoot>>,
	mut commands: Commands,
) {
	if screens.is_empty() && prompt.is_open() {
		close_leave_prompt(&mut prompt, &roots, &mut commands);
	}
}

fn sync_character_leave_prompt(
	prompt: Res<CharacterLeavePrompt>,
	asset_server: Res<AssetServer>,
	screens: Query<Entity, With<CharacterScreen>>,
	roots: Query<(Entity, &CharacterLeavePromptKind), With<CharacterLeavePromptRoot>>,
	mut commands: Commands,
) {
	let have = roots.iter().next().map(|(_, kind)| kind.0);
	if prompt.kind == have {
		return;
	}
	for (entity, _) in &roots {
		commands.entity(entity).try_despawn();
	}
	let Some(kind) = prompt.kind else {
		return;
	};
	let Ok(screen) = screens.single() else {
		return;
	};
	commands.entity(screen).with_children(|parent| {
		spawn_leave_prompt(parent, &HudFonts::load(asset_server.as_ref()), kind);
	});
}

fn spawn_leave_prompt(
	parent: &mut ChildSpawnerCommands,
	fonts: &HudFonts,
	kind: CharacterLeaveKind,
) {
	let (title, confirm) = prompt_copy(kind);
	parent
		.spawn((
			CharacterLeavePromptRoot,
			CharacterLeavePromptKind(kind),
			Node {
				position_type: PositionType::Absolute,
				left: Val::Px(0.0),
				top: Val::Px(0.0),
				width: Val::Percent(100.0),
				height: Val::Percent(100.0),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
				padding: UiRect::bottom(Val::Percent(22.0)),
				..default()
			},
			GlobalZIndex(100),
			Pickable::IGNORE,
		))
		.with_children(|root| {
			root.spawn((
				Button,
				LeavePromptCancel,
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
					title,
					TEXT_YELLOW,
					Justify::Left,
				);
				let mut list = card.spawn((
					CharacterLeavePromptMenu,
					HudOverlayMenu,
					HudMenu::new(2),
					Node {
						width: Val::Percent(100.0),
						flex_direction: FlexDirection::Column,
						align_items: AlignItems::FlexStart,
						row_gap: Val::Px(PANEL_ROW_GAP),
						..default()
					},
					Pickable::IGNORE,
				));
				let menu = list.id();
				list.with_children(|list| {
					spawn_hud_action(
						list,
						fonts,
						confirm,
						JustifyContent::FlexStart,
						(LeavePromptConfirm, HudMenuItem { index: 0, menu }),
					);
					spawn_hud_action(
						list,
						fonts,
						"Cancel",
						JustifyContent::FlexStart,
						(LeavePromptCancel, HudMenuItem { index: 1, menu }),
					);
				});
			});
		});
}

fn prompt_copy(kind: CharacterLeaveKind) -> (&'static str, &'static str) {
	match kind {
		CharacterLeaveKind::Save => ("Save and exit?", "Save"),
		CharacterLeaveKind::SaveFailed => ("Couldn't save. Try again?", "Retry"),
		CharacterLeaveKind::Discard => ("Exit without saving?", "Exit"),
	}
}

fn on_leave_prompt_click(
	mut click: On<Pointer<Click>>,
	confirms: Query<(), With<LeavePromptConfirm>>,
	cancels: Query<(), With<LeavePromptCancel>>,
	child_of: Query<&ChildOf>,
	roots: Query<Entity, With<CharacterLeavePromptRoot>>,
	save_root: Res<SaveRoot>,
	editing: Option<Res<EditingCharacter>>,
	menu_state: Res<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut commands: Commands,
) {
	if pointer_target(&click, &child_of, |entity| confirms.contains(entity)).is_some() {
		click.propagate(false);
		apply_leave_prompt(
			LeavePromptAction::Confirm,
			&mut prompt,
			&roots,
			&save_root,
			editing.as_deref(),
			&menu_state,
			return_to.as_deref().copied(),
			&mut commands,
		);
		return;
	}
	if pointer_target(&click, &child_of, |entity| cancels.contains(entity)).is_some() {
		click.propagate(false);
		apply_leave_prompt(
			LeavePromptAction::Dismiss,
			&mut prompt,
			&roots,
			&save_root,
			editing.as_deref(),
			&menu_state,
			return_to.as_deref().copied(),
			&mut commands,
		);
	}
}

fn on_leave_prompt_nav(
	impulse: On<MenuNavImpulse>,
	menus: Query<&HudMenu, With<CharacterLeavePromptMenu>>,
	roots: Query<Entity, With<CharacterLeavePromptRoot>>,
	save_root: Res<SaveRoot>,
	editing: Option<Res<EditingCharacter>>,
	menu_state: Res<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut consumed: ResMut<MenuBackConsumed>,
	mut commands: Commands,
) {
	let Ok(menu) = menus.get(impulse.entity) else {
		return;
	};
	match impulse.event().nav {
		MenuNav::Select => {
			if prompt.opened_this_frame {
				return;
			}
			let action = if menu.selected == 0 {
				LeavePromptAction::Confirm
			} else {
				LeavePromptAction::Dismiss
			};
			if action == LeavePromptAction::Dismiss {
				consumed.0 = true;
			}
			apply_leave_prompt(
				action,
				&mut prompt,
				&roots,
				&save_root,
				editing.as_deref(),
				&menu_state,
				return_to.as_deref().copied(),
				&mut commands,
			);
		}
		MenuNav::Back => {
			if prompt.opened_this_frame {
				return;
			}
			consumed.0 = true;
			apply_leave_prompt(
				LeavePromptAction::Dismiss,
				&mut prompt,
				&roots,
				&save_root,
				editing.as_deref(),
				&menu_state,
				return_to.as_deref().copied(),
				&mut commands,
			);
		}
		_ => {}
	}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LeavePromptAction {
	Confirm,
	Dismiss,
}

#[allow(clippy::too_many_arguments)]
fn apply_leave_prompt(
	action: LeavePromptAction,
	prompt: &mut CharacterLeavePrompt,
	roots: &Query<Entity, With<CharacterLeavePromptRoot>>,
	save_root: &SaveRoot,
	editing: Option<&EditingCharacter>,
	menu_state: &CharacterMenuState,
	return_to: Option<CharacterEditorReturn>,
	commands: &mut Commands,
) {
	if action == LeavePromptAction::Dismiss {
		close_leave_prompt(prompt, roots, commands);
		return;
	}
	let Some(kind) = prompt.kind else {
		return;
	};
	if kind.saves() {
		let Some(editing) = editing else {
			warn!("save character: no editing id");
			prompt.kind = Some(CharacterLeaveKind::SaveFailed);
			return;
		};
		if let Err(error) = save_editing_character(save_root, editing.id, &menu_state.0) {
			warn!(
				"failed to save character {} under {}: {error}",
				editing.id.to_hex(),
				save_root.path.display()
			);
			prompt.kind = Some(CharacterLeaveKind::SaveFailed);
			return;
		}
		set_active_character(commands, save_root, editing.id);
	}
	close_leave_prompt(prompt, roots, commands);
	leave_character_editor(commands, return_to);
}

fn close_leave_prompt(
	prompt: &mut CharacterLeavePrompt,
	roots: &Query<Entity, With<CharacterLeavePromptRoot>>,
	commands: &mut Commands,
) {
	prompt.kind = None;
	prompt.opened_this_frame = false;
	for entity in roots.iter() {
		commands.entity(entity).try_despawn();
	}
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

#[cfg(test)]
mod tests {
	use super::{
		character_leave_edge, character_screen_modal, latch_modal_button, CharacterLeaveKind,
		CharacterModalMode,
	};

	#[test]
	fn start_on_the_main_screen_asks_to_save() {
		assert_eq!(
			character_leave_edge(true, false, false, true, false),
			Some(CharacterLeaveKind::Save)
		);
	}

	#[test]
	fn back_on_the_main_screen_asks_to_discard() {
		assert_eq!(
			character_leave_edge(true, false, false, false, true),
			Some(CharacterLeaveKind::Discard)
		);
	}

	#[test]
	fn overlay_modal_or_open_prompt_keep_their_edges() {
		assert_eq!(character_leave_edge(true, true, false, true, true), None);
		assert_eq!(character_leave_edge(true, false, true, true, true), None);
		assert_eq!(character_leave_edge(false, false, false, true, true), None);
	}

	#[test]
	fn start_wins_when_both_edges_land() {
		assert_eq!(
			character_leave_edge(true, false, false, true, true),
			Some(CharacterLeaveKind::Save)
		);
	}

	#[test]
	fn name_modal_or_catalog_is_screen_modal_mode() {
		assert!(character_screen_modal(false, false, true, false, false, false, false, false));
		assert!(character_screen_modal(false, false, false, true, false, false, false, false));
		assert!(character_screen_modal(false, false, false, false, true, false, false, false));
		assert!(character_screen_modal(false, true, false, false, false, false, false, false));
		assert!(character_screen_modal(false, false, false, false, false, true, false, false));
		assert!(!character_screen_modal(false, false, false, false, false, false, false, false));
	}

	#[test]
	fn start_from_a_modal_does_not_bubble_after_submit() {
		assert!(latch_modal_button(false, true, true));
		assert!(latch_modal_button(true, false, true));
		assert!(!latch_modal_button(true, false, false));
		assert!(!latch_modal_button(false, false, true));

		let mut mode = CharacterModalMode::default();
		mode.tick(true, true, false);
		assert!(mode.blocks_leave());
		mode.tick(false, true, false);
		assert!(mode.blocks_leave());
		assert_eq!(character_leave_edge(true, mode.blocks_leave(), false, true, false), None);
		mode.tick(false, false, false);
		assert!(!mode.blocks_leave());
		assert_eq!(
			character_leave_edge(true, mode.blocks_leave(), false, true, false),
			Some(CharacterLeaveKind::Save)
		);
	}
}
