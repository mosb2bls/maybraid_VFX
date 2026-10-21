//! Actor- and session-scoped participation. Install or remove — do not
//! recompute a global mode table each frame. Suspension is a separate owned
//! token stack.

use bevy::prelude::*;
use maybraid_controllers_core::{SuspensionScope, SuspensionStack};

/// Installed participation. Suspend lives on [`SuspensionStack`], not here.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Participation {
	#[default]
	Absent,
	Active,
}

impl Participation {
	pub const fn is_installed(self) -> bool {
		matches!(self, Self::Active)
	}

	pub fn install(&mut self) {
		*self = Self::Active;
	}

	pub fn uninstall(&mut self) {
		*self = Self::Absent;
	}
}

/// Session-scoped capabilities. Independent bits, not a game-mode enum.
///
/// Lifecycle methods change participation. Update systems apply projections
/// and honor owned suspension tokens; they do not rebuild this table from
/// shell state every frame.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionCapabilities {
	gameplay: Participation,
	physics: Participation,
	streaming: Participation,
	scenery: Participation,
	menu_shell: Participation,
	simulation: Participation,
}

impl Default for SessionCapabilities {
	fn default() -> Self {
		let mut capabilities = Self {
			gameplay: Participation::Absent,
			physics: Participation::Absent,
			streaming: Participation::Absent,
			scenery: Participation::Absent,
			menu_shell: Participation::Absent,
			simulation: Participation::Absent,
		};
		capabilities.menu_shell.install();
		capabilities
	}
}

impl SessionCapabilities {
	pub const fn gameplay(self) -> Participation {
		self.gameplay
	}

	pub const fn physics(self) -> Participation {
		self.physics
	}

	pub const fn streaming(self) -> Participation {
		self.streaming
	}

	pub const fn scenery(self) -> Participation {
		self.scenery
	}

	pub const fn menu_shell(self) -> Participation {
		self.menu_shell
	}

	pub const fn simulation(self) -> Participation {
		self.simulation
	}

	pub fn gameplay_enabled(self, suspensions: &SuspensionStack) -> bool {
		self.gameplay.is_installed() && !suspensions.freezes(SuspensionScope::Gameplay)
	}

	pub const fn physics_installed(self) -> bool {
		self.physics.is_installed()
	}

	pub const fn streaming_enabled(self) -> bool {
		self.streaming.is_installed()
	}

	pub const fn scenery_active(self) -> bool {
		self.scenery.is_installed()
	}

	pub const fn menu_shell_active(self) -> bool {
		self.menu_shell.is_installed()
	}

	pub fn simulation_active(self, suspensions: &SuspensionStack) -> bool {
		self.simulation.is_installed() && !suspensions.freezes(SuspensionScope::Simulation)
	}

	/// Player realization this frame. Capture and the map sheet are not
	/// simulation halt.
	pub fn player_realization_allowed(
		self,
		suspensions: &SuspensionStack,
		capture_blocks: bool,
		map_open: bool,
	) -> bool {
		self.gameplay_enabled(suspensions) && !capture_blocks && !map_open
	}

	pub fn combat_hud_visible(self, suspensions: &SuspensionStack, map_open: bool) -> bool {
		self.gameplay_enabled(suspensions) && !map_open
	}

	/// Home / Characters: preview shell, no play session.
	pub fn install_menu_shell(&mut self) {
		self.gameplay.uninstall();
		self.physics.uninstall();
		self.streaming.uninstall();
		self.scenery.uninstall();
		self.simulation.uninstall();
		self.menu_shell.install();
	}

	/// LoadingWorld: stream the playable rings when Discovery is armed.
	pub fn install_loading(&mut self, stream: bool) {
		self.gameplay.uninstall();
		self.physics.uninstall();
		self.scenery.uninstall();
		self.menu_shell.uninstall();
		self.simulation.uninstall();
		if stream {
			self.streaming.install();
		} else {
			self.streaming.uninstall();
		}
	}

	/// Live Discovery play. Motors are installed; NPCs keep simulating.
	pub fn install_play(&mut self, stream: bool) {
		self.gameplay.install();
		self.physics.install();
		self.scenery.install();
		self.menu_shell.uninstall();
		self.simulation.install();
		if stream {
			self.streaming.install();
		} else {
			self.streaming.uninstall();
		}
	}

	/// Leave / Home teardown. Pause must not call this.
	pub fn uninstall_play(&mut self) {
		self.gameplay.uninstall();
		self.physics.uninstall();
		self.streaming.uninstall();
		self.scenery.uninstall();
		self.simulation.uninstall();
		self.menu_shell.install();
	}
}

#[cfg(test)]
mod tests {
	use super::{Participation, SessionCapabilities};
	use maybraid_controllers_core::SuspensionStack;

	#[test]
	fn install_is_not_suspend() {
		let mut bit = Participation::Absent;
		bit.install();
		assert!(bit.is_installed());
		bit.uninstall();
		assert!(!bit.is_installed());
	}

	#[test]
	fn loading_streams_without_gameplay() {
		let mut session = SessionCapabilities::default();
		let suspensions = SuspensionStack::default();
		session.install_loading(true);
		assert!(session.streaming_enabled());
		assert!(!session.gameplay_enabled(&suspensions));
		assert!(!session.physics_installed());
		assert!(!session.scenery_active());
		assert!(!session.menu_shell_active());
		assert!(!session.simulation_active(&suspensions));
	}

	#[test]
	fn pause_is_not_session_end() {
		let mut session = SessionCapabilities::default();
		let mut suspensions = SuspensionStack::default();
		session.install_play(true);
		suspensions.set_pause(true);
		assert!(!session.gameplay_enabled(&suspensions));
		assert!(session.gameplay().is_installed());
		assert!(session.physics_installed());
		assert!(session.scenery_active());
		assert!(session.streaming_enabled());
		assert!(session.simulation_active(&suspensions));
		assert!(!session.menu_shell_active());
		suspensions.set_pause(false);
		assert!(session.gameplay_enabled(&suspensions));
		session.uninstall_play();
		suspensions.release_play();
		assert!(!session.physics_installed());
		assert!(session.menu_shell_active());
	}

	#[test]
	fn capture_blocks_the_player_not_npcs() {
		let mut session = SessionCapabilities::default();
		let suspensions = SuspensionStack::default();
		session.install_play(true);
		assert!(!session.player_realization_allowed(&suspensions, true, false));
		assert!(session.simulation_active(&suspensions));
		assert!(session.player_realization_allowed(&suspensions, false, false));
		assert!(!session.player_realization_allowed(&suspensions, false, true));
	}

	#[test]
	fn loading_play_pause_inventory_resume_home() {
		let mut session = SessionCapabilities::default();
		let mut suspensions = SuspensionStack::default();
		session.install_loading(true);
		assert!(session.streaming_enabled());
		assert!(!session.gameplay_enabled(&suspensions));

		session.install_play(true);
		assert!(session.gameplay_enabled(&suspensions));
		assert!(session.physics_installed());

		suspensions.set_pause(true);
		assert!(!session.gameplay_enabled(&suspensions));
		assert!(session.physics_installed());
		assert!(session.simulation_active(&suspensions));

		suspensions.set_pause(false);
		assert!(session.gameplay_enabled(&suspensions));

		session.uninstall_play();
		suspensions.release_play();
		assert!(!session.gameplay_enabled(&suspensions));
		assert!(!session.physics_installed());
		assert!(session.menu_shell_active());
	}
}
