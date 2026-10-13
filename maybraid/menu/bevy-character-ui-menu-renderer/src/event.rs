use bevy::prelude::*;
use character_ui_menu::CameraFocus;

/// Outbound messages emitted by the character menu renderer.
#[derive(Clone, Debug, Message, PartialEq)]
pub enum CharacterMenuEvent<M> {
	CameraFocus(CameraFocus),
	MenuUpdate(M),
}
