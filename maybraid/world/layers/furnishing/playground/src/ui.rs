use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::preview::PreviewConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "Furniture playground - / cmd - WASD/Space/Shift - mouse look".into(),
		empty_console_text: "Console: `show bed|chair|chest|counter|gallery`, `help`".into(),
		root_background: Color::srgba(0.12, 0.14, 0.18, 0.82),
		controls_hint: "help — show bed|chair|chest|counter|gallery — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	config: Res<PreviewConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	status.0 = config.status_label();
}
