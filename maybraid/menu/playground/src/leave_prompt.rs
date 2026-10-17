//! Start / Back confirmations on the character editor main screen.

use bevy::prelude::*;
use bevy::text::Justify;
use character_persist::SaveRoot;
use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad, PadButton, VirtualPad};
use menu_components::theme::{
	menu_list_panel_node, menu_region_wash, HEADER_FONT_SIZE, PANEL_ROW_GAP, TEXT_YELLOW,
};
use menu_components::{
	emit_short_text_submit_on_confirm, spawn_hud_action, spawn_hud_text, HudFonts, HudMenu,
	HudMenuItem, HudOverlayMenu, MenuModal, MenuModalDismiss, MenuModalId, MenuModalLayer,
	MenuModalStack, MenuModalSystems, ScreenBackPressed, TextMenuSystems,
};

use crate::character::{save_chrome, CharacterEditBaseline, CharacterMenuState, CharacterScreen};
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

impl CharacterLeavePrompt {
	pub fn is_open(&self) -> bool {
		self.kind.is_some()
	}

	pub fn open(&mut self, kind: CharacterLeaveKind, stack: &mut MenuModalStack) {
		self.kind = Some(kind);
		self.opened_this_frame = true;
		stack.open(MenuModalLayer::CHARACTER_LEAVE);
		stack.mark_consumed();
	}

	pub fn close(&mut self, stack: &mut MenuModalStack) {
		self.kind = None;
		self.opened_this_frame = false;
		stack.close(MenuModalId::CHARACTER_LEAVE);
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
			.add_observer(on_leave_prompt_click)
			.add_observer(on_leave_prompt_nav)
			.add_systems(Update, close_leave_on_modal_dismiss.in_set(MenuModalSystems::Apply))
			.add_systems(
				Update,
				intercept_character_leave_edges
					.in_set(TextMenuSystems::InputLock)
					.after(MenuModalSystems::Apply)
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

/// Confirm only when the Save chrome is up (create, or a dirty saved edit).
/// A clean saved character — including one just written by Save Changes —
/// leaves immediately.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CharacterLeaveDecision {
	Prompt(CharacterLeaveKind),
	Leave,
}

pub fn character_leave_decision(
	kind: CharacterLeaveKind,
	needs_save: bool,
) -> CharacterLeaveDecision {
	if needs_save {
		CharacterLeaveDecision::Prompt(kind)
	} else {
		CharacterLeaveDecision::Leave
	}
}

fn intercept_character_leave_edges(
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut stack: ResMut<MenuModalStack>,
	mut nav: ResMut<MenuNavPad>,
	pad: Option<Res<VirtualPad>>,
	menu_state: Option<Res<CharacterMenuState>>,
	baseline: Option<Res<CharacterEditBaseline>>,
	return_to: Option<Res<CharacterEditorReturn>>,
	screens: Query<(), With<CharacterScreen>>,
	mut backs: MessageReader<ScreenBackPressed>,
	mut commands: Commands,
) {
	prompt.opened_this_frame = false;
	let back_click = backs.read().next().is_some();
	let start = pad.is_some_and(|pad| pad.just_pressed(PadButton::Start));
	let back = back_click || nav.just_pressed(MenuNav::Back);
	let hold = if start { stack.blocks_screen_select() } else { stack.blocks_screen_back() };
	let Some(kind) = character_leave_edge(!screens.is_empty(), hold, false, start, back) else {
		return;
	};
	match kind {
		CharacterLeaveKind::Save | CharacterLeaveKind::SaveFailed => {
			nav.events.retain(|event| *event != MenuNav::Select);
		}
		CharacterLeaveKind::Discard => {
			nav.events.retain(|event| *event != MenuNav::Back);
		}
	}
	let needs_save = menu_state
		.as_ref()
		.is_none_or(|menu| save_chrome(&menu.0, baseline.as_deref()).is_some());
	match character_leave_decision(kind, needs_save) {
		CharacterLeaveDecision::Prompt(kind) => prompt.open(kind, &mut stack),
		CharacterLeaveDecision::Leave => {
			stack.mark_consumed();
			leave_character_editor(&mut commands, return_to.as_deref().copied());
		}
	}
}

fn close_leave_prompt_without_character(
	screens: Query<(), With<CharacterScreen>>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut stack: ResMut<MenuModalStack>,
	roots: Query<Entity, With<CharacterLeavePromptRoot>>,
	mut commands: Commands,
) {
	if screens.is_empty() && prompt.is_open() {
		close_leave_prompt(&mut prompt, &mut stack, &roots, &mut commands);
	}
}

fn close_leave_on_modal_dismiss(
	mut dismiss: MessageReader<MenuModalDismiss>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut stack: ResMut<MenuModalStack>,
	roots: Query<Entity, With<CharacterLeavePromptRoot>>,
	mut commands: Commands,
) {
	if prompt.opened_this_frame {
		return;
	}
	if !dismiss.read().any(|event| event.is(MenuModalId::CHARACTER_LEAVE)) {
		return;
	}
	close_leave_prompt(&mut prompt, &mut stack, &roots, &mut commands);
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
	let Ok(mut screen) = commands.get_entity(screen) else {
		return;
	};
	screen.with_children(|parent| {
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
			MenuModal::CHARACTER_LEAVE,
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
				BackgroundColor(menu_region_wash()),
			));
			let mut card = menu_list_panel_node(AlignItems::FlexStart);
			card.width = Val::Px(720.0);
			card.max_width = Val::Percent(92.0);
			card.row_gap = Val::Px(PANEL_ROW_GAP);
			root.spawn((card, BackgroundColor(menu_region_wash()), Pickable::default()))
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
	baseline: Option<Res<CharacterEditBaseline>>,
	mut menu_state: ResMut<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut stack: ResMut<MenuModalStack>,
	mut commands: Commands,
) {
	if pointer_target(&click, &child_of, |entity| confirms.contains(entity)).is_some() {
		click.propagate(false);
		apply_leave_prompt(
			LeavePromptAction::Confirm,
			&mut prompt,
			&mut stack,
			&roots,
			&save_root,
			editing.as_deref(),
			baseline.as_deref(),
			&mut menu_state,
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
			&mut stack,
			&roots,
			&save_root,
			editing.as_deref(),
			baseline.as_deref(),
			&mut menu_state,
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
	baseline: Option<Res<CharacterEditBaseline>>,
	mut menu_state: ResMut<CharacterMenuState>,
	return_to: Option<Res<CharacterEditorReturn>>,
	mut prompt: ResMut<CharacterLeavePrompt>,
	mut stack: ResMut<MenuModalStack>,
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
				stack.mark_consumed();
			}
			apply_leave_prompt(
				action,
				&mut prompt,
				&mut stack,
				&roots,
				&save_root,
				editing.as_deref(),
				baseline.as_deref(),
				&mut menu_state,
				return_to.as_deref().copied(),
				&mut commands,
			);
		}
		MenuNav::Back => {
			if prompt.opened_this_frame {
				return;
			}
			stack.mark_consumed();
			apply_leave_prompt(
				LeavePromptAction::Dismiss,
				&mut prompt,
				&mut stack,
				&roots,
				&save_root,
				editing.as_deref(),
				baseline.as_deref(),
				&mut menu_state,
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
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<CharacterLeavePromptRoot>>,
	save_root: &SaveRoot,
	editing: Option<&EditingCharacter>,
	baseline: Option<&CharacterEditBaseline>,
	menu_state: &mut CharacterMenuState,
	return_to: Option<CharacterEditorReturn>,
	commands: &mut Commands,
) {
	if action == LeavePromptAction::Dismiss {
		close_leave_prompt(prompt, stack, roots, commands);
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
	} else if let Some(baseline) = baseline {
		baseline.restore_into(&mut menu_state.0);
	}
	close_leave_prompt(prompt, stack, roots, commands);
	leave_character_editor(commands, return_to);
}

fn close_leave_prompt(
	prompt: &mut CharacterLeavePrompt,
	stack: &mut MenuModalStack,
	roots: &Query<Entity, With<CharacterLeavePromptRoot>>,
	commands: &mut Commands,
) {
	prompt.close(stack);
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
		character_leave_decision, character_leave_edge, close_leave_on_modal_dismiss,
		intercept_character_leave_edges, CharacterLeaveDecision, CharacterLeaveKind,
		CharacterLeavePrompt, CharacterLeavePromptMenu,
	};
	use crate::character::{CharacterEditBaseline, CharacterMenuState, CharacterScreen};
	use crate::session::{
		finish_character_editor_exit, CharacterEditorExit, CharacterEditorReturn, EditingCharacter,
	};
	use bevy::ecs::system::RunSystemOnce;
	use bevy::prelude::*;
	use character_items::Inventory;
	use character_persist::{CharacterId, SaveRoot};
	use maybraid_input::{MenuNav, MenuNavImpulse, MenuNavPad};
	use menu_components::{
		consume_menu_modal_back, consume_screen_back, HudMenu, MenuModalDismiss, MenuModalId,
		MenuModalLayer, MenuModalStack, ScreenBackPressed,
	};
	use menu_screens::{InGameScreen, MenuScreen, RequestShowInGame};

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
	fn close_drops_the_leave_layer() {
		let mut prompt = CharacterLeavePrompt::default();
		let mut stack = MenuModalStack::default();
		prompt.open(CharacterLeaveKind::Discard, &mut stack);
		assert!(stack.contains(MenuModalId::CHARACTER_LEAVE));
		prompt.close(&mut stack);
		assert!(!prompt.is_open());
		assert!(!stack.contains(MenuModalId::CHARACTER_LEAVE));
	}

	#[test]
	fn clean_editor_leaves_without_a_prompt() {
		assert_eq!(
			character_leave_decision(CharacterLeaveKind::Discard, false),
			CharacterLeaveDecision::Leave
		);
		assert_eq!(
			character_leave_decision(CharacterLeaveKind::Save, false),
			CharacterLeaveDecision::Leave
		);
	}

	#[test]
	fn dirty_or_create_editor_still_confirms_leave() {
		assert_eq!(
			character_leave_decision(CharacterLeaveKind::Discard, true),
			CharacterLeaveDecision::Prompt(CharacterLeaveKind::Discard)
		);
		assert_eq!(
			character_leave_decision(CharacterLeaveKind::Save, true),
			CharacterLeaveDecision::Prompt(CharacterLeaveKind::Save)
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
	fn start_from_a_modal_does_not_bubble_after_submit() {
		let mut stack = MenuModalStack::default();
		stack.open(MenuModalLayer::SHORT_TEXT);
		stack.tick_buttons(false, true);
		assert!(stack.blocks_screen_select());
		stack.close(MenuModalId::SHORT_TEXT);
		stack.begin_frame();
		stack.tick_buttons(false, true);
		assert!(stack.blocks_screen_select());
		assert_eq!(
			character_leave_edge(true, stack.blocks_screen_select(), false, true, false),
			None
		);
		stack.tick_buttons(false, false);
		assert!(!stack.blocks_screen_select());
		assert_eq!(
			character_leave_edge(true, stack.blocks_screen_select(), false, true, false),
			Some(CharacterLeaveKind::Save)
		);
	}

	fn leave_world(open_keyboard: bool) -> World {
		let mut world = World::new();
		world.init_resource::<CharacterLeavePrompt>();
		world.init_resource::<MenuModalStack>();
		world.init_resource::<MenuNavPad>();
		world.init_resource::<ButtonInput<KeyCode>>();
		world.init_resource::<Messages<MenuModalDismiss>>();
		world.init_resource::<Messages<ScreenBackPressed>>();
		world.spawn(CharacterScreen);
		if open_keyboard {
			world.resource_mut::<MenuModalStack>().open(MenuModalLayer::SHORT_TEXT);
		}
		world.resource_mut::<MenuNavPad>().events.push(MenuNav::Back);
		world
	}

	#[test]
	fn keyboard_back_does_not_open_the_leave_prompt() {
		let mut world = leave_world(true);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(intercept_character_leave_edges).expect("intercept");
		assert!(!world.resource::<CharacterLeavePrompt>().is_open());
		assert_eq!(
			world
				.run_system_once(|mut dismiss: MessageReader<MenuModalDismiss>| {
					dismiss.read().next().map(|event| event.id)
				})
				.expect("dismiss"),
			Some(MenuModalId::SHORT_TEXT)
		);
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
	fn leave_prompt_back_closes_the_card_and_does_not_exit() {
		let mut world = leave_world(false);
		world.resource_mut::<CharacterLeavePrompt>().kind = Some(CharacterLeaveKind::Discard);
		world.resource_mut::<MenuModalStack>().open(MenuModalLayer::CHARACTER_LEAVE);
		world.run_system_once(consume_menu_modal_back).expect("consume");
		world.run_system_once(close_leave_on_modal_dismiss).expect("close");
		world.run_system_once(intercept_character_leave_edges).expect("intercept");
		assert!(!world.resource::<CharacterLeavePrompt>().is_open());
		assert!(!world.resource::<MenuModalStack>().contains(MenuModalId::CHARACTER_LEAVE));
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

	fn scratch_root() -> SaveRoot {
		SaveRoot::at(
			std::env::temp_dir().join(format!("maybraid-leave-{}", CharacterId::new().to_hex())),
		)
	}

	/// `sync_preview` / pause look used to spawn a gallery mesh over the live
	/// world player when `CharacterEditorReturn` vanished while the HUD was up.
	fn panic_if_in_game_editor_lost_return(
		screens: Query<(), With<CharacterScreen>>,
		return_to: Option<Res<CharacterEditorReturn>>,
		exit: Option<Res<CharacterEditorExit>>,
	) {
		if !screens.is_empty() && exit.is_some() && return_to.is_none() {
			panic!(
				"CharacterEditorReturn removed while CharacterScreen is still live \
				 (in-game editor teardown)"
			);
		}
	}

	fn apply_show_in_game_request(
		mut commands: Commands,
		requests: Query<Entity, With<RequestShowInGame>>,
		screens: Query<Entity, With<MenuScreen>>,
	) {
		if requests.is_empty() {
			return;
		}
		for entity in &requests {
			commands.entity(entity).try_despawn();
		}
		for entity in &screens {
			commands.entity(entity).try_despawn();
		}
		commands.spawn((InGameScreen, MenuScreen));
	}

	fn editor_leave_app(dirty_name: &str) -> App {
		let mut menu = CharacterMenuState::default();
		menu.0.name = String::from(dirty_name);
		let baseline =
			CharacterEditBaseline { name: String::from("Roamer"), inventory: Inventory::default() };
		let mut app = App::new();
		app.add_plugins(MinimalPlugins)
			.insert_resource(scratch_root())
			.insert_resource(menu)
			.insert_resource(baseline)
			.insert_resource(EditingCharacter { id: CharacterId(11) })
			.insert_resource(CharacterEditorReturn::InGame)
			.init_resource::<CharacterLeavePrompt>()
			.init_resource::<MenuModalStack>()
			.add_observer(super::on_leave_prompt_nav)
			.add_systems(
				Update,
				(
					apply_show_in_game_request,
					finish_character_editor_exit,
					panic_if_in_game_editor_lost_return,
				),
			);
		app.world_mut().spawn((CharacterScreen, MenuScreen));
		app
	}

	fn open_discard_prompt(app: &mut App) -> Entity {
		{
			let world = app.world_mut();
			let mut prompt = world.resource_mut::<CharacterLeavePrompt>();
			prompt.kind = Some(CharacterLeaveKind::Discard);
			prompt.opened_this_frame = false;
			world.resource_mut::<MenuModalStack>().open(MenuModalLayer::CHARACTER_LEAVE);
		}
		app.world_mut().spawn((CharacterLeavePromptMenu, HudMenu::new(2))).id()
	}

	fn has_component<T: Component>(app: &mut App) -> bool {
		let world = app.world_mut();
		world.query::<&T>().iter(world).next().is_some()
	}

	#[test]
	fn canceling_the_in_game_leave_prompt_keeps_the_editor() {
		let mut app = editor_leave_app("Dirty");
		let menu = open_discard_prompt(&mut app);
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Back));
		app.update();
		app.update();

		assert!(app
			.world()
			.get_resource::<CharacterLeavePrompt>()
			.is_some_and(|prompt| !prompt.is_open()));
		assert!(has_component::<CharacterScreen>(&mut app));
		assert_eq!(
			app.world().get_resource::<CharacterEditorReturn>().copied(),
			Some(CharacterEditorReturn::InGame)
		);
		assert_eq!(app.world().resource::<CharacterMenuState>().0.name, "Dirty");
		assert!(app.world().get_resource::<CharacterEditorExit>().is_none());
		assert!(!has_component::<InGameScreen>(&mut app));
	}

	#[test]
	fn discarding_the_in_game_editor_restores_baseline_without_dropping_return() {
		let mut app = editor_leave_app("Dirty");
		let menu = open_discard_prompt(&mut app);
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		app.update();
		app.update();

		assert!(!has_component::<CharacterScreen>(&mut app));
		assert!(has_component::<InGameScreen>(&mut app));
		assert!(app.world().get_resource::<CharacterEditorReturn>().is_none());
		assert!(app.world().get_resource::<CharacterEditorExit>().is_none());
		assert_eq!(app.world().resource::<CharacterMenuState>().0.name, "Roamer");
		assert!(app
			.world()
			.get_resource::<CharacterLeavePrompt>()
			.is_some_and(|prompt| !prompt.is_open()));
	}

	#[test]
	fn selecting_cancel_on_the_leave_prompt_stays_in_the_editor() {
		let mut app = editor_leave_app("Dirty");
		let menu = open_discard_prompt(&mut app);
		app.world_mut().entity_mut(menu).insert(HudMenu { selected: 1, item_count: 2 });
		app.world_mut().trigger(MenuNavImpulse::new(menu, MenuNav::Select));
		app.update();
		app.update();

		assert!(has_component::<CharacterScreen>(&mut app));
		assert_eq!(
			app.world().get_resource::<CharacterEditorReturn>().copied(),
			Some(CharacterEditorReturn::InGame)
		);
		assert_eq!(app.world().resource::<CharacterMenuState>().0.name, "Dirty");
	}
}
