//! Publish menu snapshots for the agent plugin. World stays free of menu types.

use std::collections::HashSet;

use bevy::prelude::*;
use maybraid_menu_controller::MenuController;
use maybraid_world::AgentMenuState;
use menu_components::{
	ActiveOverlayKey, HudMenu, HudMenuItem, ShortTextModal, TextMenu, TextMenuItem,
	TextMenuItemLabel,
};
use menu_playground::{CharacterEditorReturn, CharacterLeavePrompt, CharacterScreen};
use menu_screens::{
	GalleryScreen, HomeScreen, InGameScreen, InGameSettingsScreen, LoadingScreen, MenuScreen,
	RequestShowCreateCharacter, RequestShowGallery, RequestShowHome, RequestShowInGame,
	RequestShowInGameSettings, RequestShowLoading, RequestShowSpinReveal, SpinRevealScreen,
};

use crate::{GameFlow, WorldPause};

#[derive(Resource, Default)]
pub(crate) struct AgentShowRequestMemory(HashSet<Entity>);

pub(crate) fn publish_agent_menu_shell(
	mut state: ResMut<AgentMenuState>,
	mut memory: ResMut<AgentShowRequestMemory>,
	flow: Res<State<GameFlow>>,
	pause: Option<Res<State<WorldPause>>>,
	screens: Query<
		(
			Option<&HomeScreen>,
			Option<&GalleryScreen>,
			Option<&SpinRevealScreen>,
			Option<&CharacterScreen>,
			Option<&InGameScreen>,
			Option<&InGameSettingsScreen>,
			Option<&LoadingScreen>,
		),
		With<MenuScreen>,
	>,
	prompt: Option<Res<CharacterLeavePrompt>>,
	editor_return: Option<Res<CharacterEditorReturn>>,
	requests: Query<
		Entity,
		Or<(
			With<RequestShowHome>,
			With<RequestShowGallery>,
			With<RequestShowInGame>,
			With<RequestShowInGameSettings>,
			With<RequestShowLoading>,
			With<RequestShowSpinReveal>,
			With<RequestShowCreateCharacter>,
			With<menu_playground::RequestShowCharacter>,
		)>,
	>,
	show_character: Query<(), With<menu_playground::RequestShowCharacter>>,
) {
	let screen_count = screens.iter().len();
	let mut screen = "none";
	let mut character_alive = false;
	for (home, gallery, spin, character, in_game, settings, loading) in &screens {
		if character.is_some() {
			character_alive = true;
		}
		screen = if home.is_some() {
			"home"
		} else if gallery.is_some() {
			"gallery"
		} else if spin.is_some() {
			"spin_reveal"
		} else if character.is_some() {
			"character"
		} else if in_game.is_some() {
			"in_game"
		} else if settings.is_some() {
			"settings"
		} else if loading.is_some() {
			"loading"
		} else if screen_count > 0 {
			"unknown"
		} else {
			screen
		};
	}

	let request_ids: HashSet<Entity> = requests.iter().collect();
	let leftover_request = !request_ids.is_disjoint(&memory.0);
	memory.0 = request_ids;

	let flow_world = matches!(flow.get(), GameFlow::World);
	let pause_value = pause.as_deref().map(|state| *state.get()).unwrap_or(WorldPause::Playing);
	let pause_playing = !flow_world || matches!(pause_value, WorldPause::Playing);

	state.observation.screen = screen.into();
	state.observation.flow = flow_name(*flow.get()).into();
	state.observation.pause = pause_name(pause_value).into();
	state.observation.menu_ready = screen_count > 0;
	state.observation.invariants = menu_invariants(
		screen_count,
		flow_world,
		pause_playing,
		leftover_request,
		editor_return.is_some(),
		character_alive,
		!show_character.is_empty(),
		prompt.is_some_and(|prompt| prompt.is_open()),
	);
}

pub(crate) fn publish_agent_menu_focus(
	mut state: ResMut<AgentMenuState>,
	screens: Query<(Entity, Option<&MenuController>), With<MenuScreen>>,
	text_menus: Query<&TextMenu>,
	hud_menus: Query<&HudMenu>,
	text_items: Query<(Entity, &TextMenuItem)>,
	hud_items: Query<(Entity, &HudMenuItem)>,
	children: Query<&Children>,
	labels: Query<&Text, With<TextMenuItemLabel>>,
	texts: Query<&Text>,
	prompt: Option<Res<CharacterLeavePrompt>>,
	short_text: Option<Res<ShortTextModal>>,
	overlay: Option<Res<ActiveOverlayKey>>,
) {
	let (focus_row, focus_label) = focused_row(
		&screens,
		&text_menus,
		&hud_menus,
		&text_items,
		&hud_items,
		&children,
		&labels,
		&texts,
	);
	state.observation.modal =
		current_modal(prompt.as_deref(), short_text.as_deref(), overlay.as_deref());
	state.observation.focus_row = focus_row;
	state.observation.focus_label = focus_label;
}

fn current_modal(
	prompt: Option<&CharacterLeavePrompt>,
	short_text: Option<&ShortTextModal>,
	overlay: Option<&ActiveOverlayKey>,
) -> Option<String> {
	if let Some(name) = prompt.and_then(|prompt| prompt.agent_kind_name()) {
		return Some(name.into());
	}
	if short_text.is_some_and(|modal| modal.is_open()) {
		return Some("name".into());
	}
	overlay.and_then(|key| key.0).map(str::to_string)
}

fn focused_row(
	screens: &Query<(Entity, Option<&MenuController>), With<MenuScreen>>,
	text_menus: &Query<&TextMenu>,
	hud_menus: &Query<&HudMenu>,
	text_items: &Query<(Entity, &TextMenuItem)>,
	hud_items: &Query<(Entity, &HudMenuItem)>,
	children: &Query<&Children>,
	labels: &Query<&Text, With<TextMenuItemLabel>>,
	texts: &Query<&Text>,
) -> (Option<u32>, Option<String>) {
	let focus = screens.iter().find_map(|(_, controller)| controller.and_then(|c| c.focus));
	let Some(focus) = focus else {
		return (None, None);
	};
	if let Ok(menu) = text_menus.get(focus) {
		let row = menu.selected as u32;
		return (Some(row), text_row_label(menu.selected, text_items, children, labels));
	}
	if let Ok(menu) = hud_menus.get(focus) {
		let row = menu.selected as u32;
		return (Some(row), hud_row_label(focus, menu.selected, hud_items, children, texts));
	}
	(None, None)
}

fn text_row_label(
	selected: usize,
	items: &Query<(Entity, &TextMenuItem)>,
	children: &Query<&Children>,
	labels: &Query<&Text, With<TextMenuItemLabel>>,
) -> Option<String> {
	items.iter().find_map(|(entity, item)| {
		if item.index != selected {
			return None;
		}
		TextMenuItem::label_under(entity, children, labels)
	})
}

fn hud_row_label(
	menu: Entity,
	selected: usize,
	items: &Query<(Entity, &HudMenuItem)>,
	children: &Query<&Children>,
	texts: &Query<&Text>,
) -> Option<String> {
	for (entity, item) in items {
		if item.menu != menu || item.index != selected {
			continue;
		}
		if let Ok(text) = texts.get(entity) {
			if !text.0.is_empty() {
				return Some(text.0.clone());
			}
		}
		if let Ok(childs) = children.get(entity) {
			for child in childs {
				if let Ok(text) = texts.get(*child) {
					if !text.0.is_empty() {
						return Some(text.0.clone());
					}
				}
			}
		}
	}
	None
}

pub(crate) fn flow_name(flow: GameFlow) -> &'static str {
	match flow {
		GameFlow::Home => "home",
		GameFlow::Characters => "characters",
		GameFlow::LoadingWorld => "loading_world",
		GameFlow::World => "world",
	}
}

pub(crate) fn pause_name(pause: WorldPause) -> &'static str {
	match pause {
		WorldPause::Playing => "playing",
		WorldPause::Menu => "menu",
	}
}

pub(crate) fn menu_invariants(
	screen_count: usize,
	flow_world: bool,
	pause_playing: bool,
	leftover_request: bool,
	editor_return: bool,
	character_screen: bool,
	pending_character_show: bool,
	leave_prompt: bool,
) -> Vec<String> {
	let mut breaks = Vec::new();
	if screen_count > 1 {
		breaks.push(format!("multiple MenuScreen ({screen_count})"));
	}
	if flow_world && pause_playing && screen_count > 0 {
		breaks.push("MenuScreen while WorldPause::Playing".into());
	}
	if leftover_request {
		breaks.push("MenuShowRequest survived the frame it was queued in".into());
	}
	if editor_return && !character_screen && !pending_character_show {
		breaks.push("CharacterEditorReturn without CharacterScreen".into());
	}
	if leave_prompt && !character_screen {
		breaks.push("CharacterLeavePrompt without CharacterScreen".into());
	}
	breaks
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn flow_and_pause_names() {
		assert_eq!(flow_name(GameFlow::Home), "home");
		assert_eq!(flow_name(GameFlow::LoadingWorld), "loading_world");
		assert_eq!(pause_name(WorldPause::Menu), "menu");
	}

	#[test]
	fn healthy_home_has_no_invariants() {
		assert!(menu_invariants(1, false, true, false, false, false, false, false).is_empty());
	}

	#[test]
	fn playing_world_rejects_a_menu_screen() {
		let breaks = menu_invariants(1, true, true, false, false, false, false, false);
		assert_eq!(breaks, vec!["MenuScreen while WorldPause::Playing".to_string()]);
	}

	#[test]
	fn editor_return_is_ok_while_opening() {
		assert!(menu_invariants(0, false, true, false, true, false, true, false).is_empty());
		let breaks = menu_invariants(0, false, true, false, true, false, false, false);
		assert_eq!(breaks, vec!["CharacterEditorReturn without CharacterScreen".to_string()]);
	}

	#[test]
	fn leave_prompt_needs_character_screen() {
		let breaks = menu_invariants(1, false, true, false, false, false, false, true);
		assert_eq!(breaks, vec!["CharacterLeavePrompt without CharacterScreen".to_string()]);
	}
}
