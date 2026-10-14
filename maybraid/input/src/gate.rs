//! Gameplay suppress flag and which keyboard keys feed the pad.

use bevy::prelude::*;

/// When [`Self::is_enabled`] is false, analog and pad buttons are cleared after
/// produce. The keyboard overlay stays so debug keys and text hosts can still
/// see physical keys.
///
/// Hosts write independent lanes so they cannot clobber each other. The pad is
/// on only when every lane allows it.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct PadGameplayEnabled {
	/// Command console / other text-entry hosts.
	from_text: bool,
	/// Menu lock. A short-text modal still allows the pad so the keypad works.
	from_menu: bool,
}

impl Default for PadGameplayEnabled {
	fn default() -> Self {
		Self { from_text: true, from_menu: true }
	}
}

impl PadGameplayEnabled {
	pub fn is_enabled(self) -> bool {
		self.from_text && self.from_menu
	}

	pub fn set_text(&mut self, allow: bool) {
		self.from_text = allow;
	}

	pub fn set_menu(&mut self, allow: bool) {
		self.from_menu = allow;
	}

	pub fn text_allows(self) -> bool {
		self.from_text
	}

	pub fn menu_allows(self) -> bool {
		self.from_menu
	}
}

/// When true, a focused menu owns **B** as Back. Character `collect` skips stance.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MenuOwnsBack(pub bool);

/// Which keyboard keys become pad buttons. Gamepad bindings are unchanged.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum KeyboardPadMap {
	/// WASD + arrows on the move stick, Space → A, Escape → B.
	#[default]
	Gameplay,
	/// Menus and text: arrows on the D-Pad, Escape → Back, Enter → Start.
	Menu,
}

#[cfg(test)]
mod tests {
	use super::PadGameplayEnabled;

	#[test]
	fn both_lanes_must_allow() {
		let mut enabled = PadGameplayEnabled::default();
		assert!(enabled.is_enabled());
		enabled.set_text(false);
		assert!(!enabled.is_enabled());
		assert!(enabled.menu_allows());
		enabled.set_menu(false);
		enabled.set_text(true);
		assert!(!enabled.is_enabled());
		assert!(enabled.text_allows());
		enabled.set_menu(true);
		assert!(enabled.is_enabled());
	}

	#[test]
	fn writers_do_not_clobber() {
		let mut enabled = PadGameplayEnabled::default();
		enabled.set_menu(true);
		enabled.set_text(false);
		enabled.set_menu(true);
		assert!(!enabled.is_enabled(), "text lane stays off after a later menu write");
		enabled.set_text(true);
		assert!(enabled.is_enabled());
	}
}
