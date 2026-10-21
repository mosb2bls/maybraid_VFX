//! Derived pad policy. Written only by the capture resolver (U5).

use bevy::prelude::*;

/// Keyboard layout and analog suppress derived from [`InputRouter`] capture.
///
/// One writer. Hosts must not poke independent lanes — push or pop a capture
/// layer instead.
#[derive(Resource, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PadInputPolicy {
	keyboard_menu: bool,
	suppress_gameplay: bool,
}

impl PadInputPolicy {
	pub const fn new(keyboard_menu: bool, suppress_gameplay: bool) -> Self {
		Self { keyboard_menu, suppress_gameplay }
	}

	pub const fn gameplay() -> Self {
		Self { keyboard_menu: false, suppress_gameplay: false }
	}

	pub const fn menu() -> Self {
		Self { keyboard_menu: true, suppress_gameplay: false }
	}

	pub const fn console() -> Self {
		Self { keyboard_menu: true, suppress_gameplay: true }
	}

	pub const fn keyboard_menu(self) -> bool {
		self.keyboard_menu
	}

	pub const fn suppress_gameplay(self) -> bool {
		self.suppress_gameplay
	}
}

#[cfg(test)]
mod tests {
	use super::PadInputPolicy;

	#[test]
	fn gameplay_keeps_the_pad() {
		let policy = PadInputPolicy::gameplay();
		assert!(!policy.keyboard_menu());
		assert!(!policy.suppress_gameplay());
	}

	#[test]
	fn menu_keeps_the_pad_for_nav() {
		let policy = PadInputPolicy::menu();
		assert!(policy.keyboard_menu());
		assert!(!policy.suppress_gameplay());
	}

	#[test]
	fn console_suppresses_analog() {
		let policy = PadInputPolicy::console();
		assert!(policy.keyboard_menu());
		assert!(policy.suppress_gameplay());
	}
}
