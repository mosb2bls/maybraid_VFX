use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::character::CharacterConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "character playground - / cmd - WASD - up/down history - PgUp/PgDn scroll".into(),
		empty_console_text: "Console: (errors & `help` output) - wheel or PgUp/PgDn".into(),
		root_background: Color::srgba(0.12, 0.14, 0.18, 0.82),
		controls_hint: "help - Enter - up/down history - PgUp/PgDn - Shift+up/down scroll".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<CharacterConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = config.status_label();
}
