//! Pause / live-play flag for world character input and combat HUD.

use bevy::prelude::*;

/// When `false`, world movement / POV intents are ignored (menus, pause overlay).
/// Pause must not be treated as a play-session teardown.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct WorldGameplayEnabled(pub bool);

impl Default for WorldGameplayEnabled {
	fn default() -> Self {
		Self(true)
	}
}

impl WorldGameplayEnabled {
	/// Live play with no modal that steals character intents (console, world map).
	pub fn accepts_character_input(&self, text_focus: bool, map_open: bool) -> bool {
		self.0 && !text_focus && !map_open
	}

	/// Combat vitals stay visible during live play when the map sheet is closed.
	pub fn combat_hud_visible(&self, map_open: bool) -> bool {
		self.0 && !map_open
	}
}

#[cfg(test)]
mod tests {
	use super::WorldGameplayEnabled;

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
}
