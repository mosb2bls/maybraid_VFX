//! Pause / live-play projection. Pause must not uninstall play.

use bevy::prelude::*;
use maybraid_character_controller::CharacterIntent;
use maybraid_controllers_core::{InputRouter, InputRouting};

use crate::SessionCapabilities;

/// When `false`, world movement / POV intents are ignored (menus, pause overlay).
/// Pause must not be treated as a play-session teardown.
///
/// This is the live projection of installed gameplay minus the pause
/// [`InputRouter`] suspension. Capture and the map sheet are consulted
/// separately so they do not halt NPCs.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGameplayEnabled(pub bool);

impl Default for WorldGameplayEnabled {
	fn default() -> Self {
		Self(true)
	}
}

impl WorldGameplayEnabled {
	pub fn from_session(session: SessionCapabilities, router: &InputRouter) -> Self {
		Self(session.gameplay_enabled(router))
	}

	/// Live play with no modal that steals character intents (console, world map).
	pub fn accepts_character_input(&self, text_focus: bool, map_open: bool) -> bool {
		self.0 && !text_focus && !map_open
	}

	/// Combat vitals stay visible during live play when the map sheet is closed.
	pub fn combat_hud_visible(&self, map_open: bool) -> bool {
		self.0 && !map_open
	}
}

/// Honor Start the same frame Bind published [`CharacterIntent::InGameMenu`].
/// Production uses the game-shell pause state; stack tests register this in
/// [`crate::SessionCapabilitySystems`].
pub fn apply_in_game_menu_pause(
	mut router: InputRouting,
	mut gameplay: ResMut<WorldGameplayEnabled>,
	session: Res<SessionCapabilities>,
	mut intents: MessageReader<CharacterIntent>,
) {
	if intents.read().any(|intent| matches!(intent, CharacterIntent::InGameMenu)) {
		router.set_paused(true);
	}
	*gameplay = WorldGameplayEnabled::from_session(*session, &router);
}

#[cfg(test)]
mod tests {
	use super::WorldGameplayEnabled;
	use crate::SessionCapabilities;
	use maybraid_controllers_core::InputRouter;

	#[test]
	fn accepts_character_input_requires_live_play_without_modals() {
		assert!(WorldGameplayEnabled(true).accepts_character_input(false, false));
		assert!(!WorldGameplayEnabled(false).accepts_character_input(false, false));
		assert!(!WorldGameplayEnabled(true).accepts_character_input(true, false));
		assert!(!WorldGameplayEnabled(true).accepts_character_input(false, true));
	}

	#[test]
	fn combat_hud_stays_up_while_typing_but_not_on_the_map() {
		assert!(WorldGameplayEnabled(true).combat_hud_visible(false));
		assert!(!WorldGameplayEnabled(false).combat_hud_visible(false));
		assert!(!WorldGameplayEnabled(true).combat_hud_visible(true));
	}

	#[test]
	fn projects_session_gameplay() {
		let mut session = SessionCapabilities::default();
		let mut router = InputRouter::default();
		assert!(!WorldGameplayEnabled::from_session(session, &router).0);
		session.install_play(true);
		assert!(WorldGameplayEnabled::from_session(session, &router).0);
		router.set_paused(true);
		assert!(!WorldGameplayEnabled::from_session(session, &router).0);
		assert!(session.gameplay().is_installed());
	}
}
