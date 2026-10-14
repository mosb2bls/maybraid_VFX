//! Shared command-line lock for text-menu screens.

use bevy::prelude::*;
use game_commands::command::{TextEntryBlocked, TextEntryFocus};
use menu_components::{
	MenuModalSystems, ShortTextField, ShortTextModal, TextMenuInputLock, TextMenuSystems,
};

/// Registers the lock that keeps `/` and HUD arrows off each other.
///
/// Screen plugins add this once via [`add_menu_input`].
pub struct MenuInputPlugin;

impl Plugin for MenuInputPlugin {
	fn build(&self, app: &mut App) {
		app.add_systems(
			Update,
			sync_text_menu_input_lock
				.in_set(TextMenuSystems::InputLock)
				.after(MenuModalSystems::Apply),
		);
	}
}

pub fn add_menu_input(app: &mut App) {
	if !app.is_plugin_added::<MenuInputPlugin>() {
		app.add_plugins(MenuInputPlugin);
	}
}

fn sync_text_menu_input_lock(
	mut focus: Option<ResMut<TextEntryFocus>>,
	mut blocked: Option<ResMut<TextEntryBlocked>>,
	modal: Res<ShortTextModal>,
	fields: Query<&ShortTextField>,
	mut lock: ResMut<TextMenuInputLock>,
) {
	let short = modal.is_open() || fields.iter().any(|field| field.editing);
	if let Some(blocked) = blocked.as_mut() {
		blocked.0 = short;
	}
	if short {
		if let Some(focus) = focus.as_mut() {
			focus.0 = false;
		}
		lock.0 = true;
		return;
	}
	lock.0 = focus.as_ref().is_some_and(|focus| focus.0);
}
