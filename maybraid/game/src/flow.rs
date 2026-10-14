//! Title / characters / world routing for the Maybraid executable.

use bevy::prelude::*;
use maybraid_game_mode_reliquary::{self as reliquary, ReliquaryRoute};
use menu_screens::{HomeMenuChoice, InGameMenuChoice};

/// Which shell the executable is showing. World gameplay is only live in
/// [`Self::World`] while [`WorldPause`] is [`WorldPause::Playing`].
#[derive(States, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameFlow {
	#[default]
	Home,
	Characters,
	LoadingWorld,
	World,
}

/// Pause overlay while [`GameFlow::World`] is active. Absent in other shells.
#[derive(SubStates, Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[source(GameFlow = GameFlow::World)]
pub enum WorldPause {
	#[default]
	Playing,
	Menu,
}

/// Which world mount is live. [`GameFlow::World`] is shared; this picks the scene.
#[derive(Resource, Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum PlaySession {
	#[default]
	None,
	Discovery,
}

impl PlaySession {
	pub fn label(self) -> &'static str {
		match self {
			Self::Discovery | Self::None => "Discovery",
		}
	}
}

/// What the executable does with a home-row pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HomeRoute {
	World { session: PlaySession },
	Characters,
	Settings,
	Unimplemented,
}

impl HomeRoute {
	pub fn from_choice(choice: HomeMenuChoice) -> Self {
		match choice {
			HomeMenuChoice::Discovery => Self::World { session: PlaySession::Discovery },
			HomeMenuChoice::TrainingGround => Self::Unimplemented,
			HomeMenuChoice::Characters => Self::Characters,
			HomeMenuChoice::Settings => Self::Settings,
			HomeMenuChoice::Reliquary => match reliquary::route() {
				ReliquaryRoute::Unimplemented => Self::Unimplemented,
			},
		}
	}
}

/// What the executable does with an in-game pause-row pick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PauseMenuRoute {
	Leave,
	Settings,
	Character,
	Stay,
}

impl PauseMenuRoute {
	pub fn from_choice(choice: InGameMenuChoice) -> Self {
		match choice {
			InGameMenuChoice::Leave => Self::Leave,
			InGameMenuChoice::Settings => Self::Settings,
			InGameMenuChoice::Character => Self::Character,
			InGameMenuChoice::Records | InGameMenuChoice::Help => Self::Stay,
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn home_menu_choices_map_to_executable_routes() {
		for (choice, want) in [
			(HomeMenuChoice::Discovery, HomeRoute::World { session: PlaySession::Discovery }),
			(HomeMenuChoice::TrainingGround, HomeRoute::Unimplemented),
			(HomeMenuChoice::Characters, HomeRoute::Characters),
			(HomeMenuChoice::Settings, HomeRoute::Settings),
			(HomeMenuChoice::Reliquary, HomeRoute::Unimplemented),
		] {
			assert_eq!(HomeRoute::from_choice(choice), want, "{choice:?}");
		}
	}

	#[test]
	fn pause_menu_choices_map_to_executable_routes() {
		for (choice, want) in [
			(InGameMenuChoice::Leave, PauseMenuRoute::Leave),
			(InGameMenuChoice::Settings, PauseMenuRoute::Settings),
			(InGameMenuChoice::Character, PauseMenuRoute::Character),
			(InGameMenuChoice::Records, PauseMenuRoute::Stay),
			(InGameMenuChoice::Help, PauseMenuRoute::Stay),
		] {
			assert_eq!(PauseMenuRoute::from_choice(choice), want, "{choice:?}");
		}
	}
}
