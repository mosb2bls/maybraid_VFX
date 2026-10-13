use bevy::prelude::*;
use game_commands::ui::GameCommandStatusText;

pub(crate) fn write_status(
	status: &mut Option<ResMut<GameCommandStatusText>>,
	text: impl Into<String>,
) {
	if let Some(status) = status {
		status.0 = text.into();
	}
}
