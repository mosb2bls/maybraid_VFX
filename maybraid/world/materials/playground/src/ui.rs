use bevy::prelude::*;
use game_commands::ui::{GameCommandStatusText, GameCommandUiConfig};

use crate::preview::PreviewConfig;
use crate::shoot::ShootConfig;

pub fn ui_config() -> GameCommandUiConfig {
	GameCommandUiConfig {
		title: "World materials — / cmd — WASD/Space/Shift — mouse look".into(),
		empty_console_text:
			"Console: `show pulse`, `mesh blast|ring|shell|core|cards`, `shoot`, `help`".into(),
		root_background: Color::srgba(0.08, 0.09, 0.12, 0.86),
		controls_hint: "help — mesh blast — show pulse — shoot — Enter — history".into(),
	}
}

pub(crate) fn sync_command_status_text(
	preview: Res<PreviewConfig>,
	shoot: Res<ShootConfig>,
	mut status: ResMut<GameCommandStatusText>,
) {
	let firing = if shoot.enabled { "shooting" } else { "idle" };
	status.0 = format!("{} · {firing}", preview.status_label());
}
