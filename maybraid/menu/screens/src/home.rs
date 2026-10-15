//! Maybraid home screen: top-left title plus destination labels.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use character_model_user::list_summaries;
use character_persist::SaveRoot;
use item_icons::RequestPruneItemIconCache;
use maybraid_menu_controller::MenuController;
use menu_components::info::description::{set_description_for_menu, TextMenuDescription};
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::single_select::{republish_menu_activate, MenuFocus};
use menu_components::TextMenuPlugin;
use menu_components::{MenuObjectiveKind, TextCursorRow};

use crate::input::add_menu_input;
use crate::show::{add_menu_show_systems, MenuScreens, MenuShowRequest};
use crate::MenuScreen;

/// Queue a home-screen spawn (despawns any existing home UI first).
#[derive(Component, Debug, Clone, Copy)]
#[require(MenuShowRequest = MenuShowRequest::next())]
pub struct RequestShowHome;

/// Marker on the spawned home-screen root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct HomeScreen;

/// Home destinations. Each pickable row stamps this as a component.
///
/// In-screen: [`MenuFocus<Self>`] / [`menu_components::MenuActivate<Self>`] bubble
/// to this root. Outside the screen: [`republish_menu_activate`] copies activate
/// as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum HomeMenuChoice {
	#[default]
	Discovery,
	Reliquary,
	Characters,
	TrainingGround,
	Settings,
}

impl HomeMenuChoice {
	pub const ALL: [Self; 5] =
		[Self::Discovery, Self::Reliquary, Self::Characters, Self::TrainingGround, Self::Settings];

	pub fn label(self) -> &'static str {
		match self {
			Self::Discovery => "Discovery",
			Self::Reliquary => "Reliquary",
			Self::Characters => "Characters",
			Self::TrainingGround => "Training Ground",
			Self::Settings => "Settings",
		}
	}

	pub fn description(self) -> &'static str {
		match self {
			Self::Discovery => {
				"Roam the world of Maybraid. Find the remnants of the hidden thread. Grow within the world."
			}
			Self::Reliquary => {
				"Team up to gather and return artifacts from an opponent's reliquary. Move fast. Wager yourself."
			}
			Self::Characters => "Create and edit your characters. Check your inventory.",
			Self::TrainingGround => "Temporarily removed while it is rebuilt.",
			Self::Settings => "Adjust user and system settings to your liking.",
		}
	}

	/// Home row chrome for the current roster. Reliquary is always coming-soon and
	/// Training Ground is always temporarily removed.
	pub fn row(self, has_characters: bool) -> TextCursorRow<Self> {
		let row = TextCursorRow::new(self.label(), self);
		match self {
			Self::Reliquary => row.with_objective(MenuObjectiveKind::ComingSoon).locked(),
			Self::TrainingGround => {
				row.with_objective(MenuObjectiveKind::TemporarilyRemoved).locked()
			}
			Self::Characters if !has_characters => row.with_objective(MenuObjectiveKind::StartHere),
			Self::Discovery if !has_characters => {
				row.with_objective(MenuObjectiveKind::NeedsCharacter).locked()
			}
			_ => row,
		}
	}
}

fn roster_has_characters(save_root: Option<&SaveRoot>) -> bool {
	save_root.is_some_and(|root| !list_summaries(root).is_empty())
}

impl HomeScreen {
	pub fn scene(has_characters: bool) -> impl Scene + 'static {
		let children: Vec<Box<dyn Scene>> = vec![Box::new(
			TextCursorColumn::rows(
				"Maybraid",
				HomeMenuChoice::ALL.into_iter().map(|choice| choice.row(has_characters)),
			)
			.with_description(HomeMenuChoice::Discovery.description())
			.scene(),
		)];
		bsn! {
			HomeScreen
			MenuScreen
			MenuController
			BackgroundColor(Color::NONE)
			Node {
				width: percent(100),
				height: percent(100),
			}
			Pickable::IGNORE
			on(sync_home_description)
			on(republish_menu_activate::<HomeMenuChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_home(commands: &mut Commands) {
	commands.spawn(RequestShowHome);
	commands.spawn(RequestPruneItemIconCache);
}

pub struct HomeScreenPlugin;

impl Plugin for HomeScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<HomeMenuChoice>::default());
		add_menu_show_systems(app, apply_show_home);
	}
}

fn apply_show_home(
	mut commands: Commands,
	requests: Query<(), With<RequestShowHome>>,
	screens: MenuScreens,
	save_root: Option<Res<SaveRoot>>,
) {
	if screens.take(&mut commands, &requests).is_none() {
		return;
	}
	let has_characters = roster_has_characters(save_root.as_deref());
	commands.spawn_scene(HomeScreen::scene(has_characters));
}

fn sync_home_description(
	focus: On<MenuFocus<HomeMenuChoice>>,
	children: Query<&Children>,
	mut lines: Query<&mut Text, With<TextMenuDescription>>,
) {
	set_description_for_menu(
		focus.event().entity,
		focus.event().choice.description(),
		&children,
		&mut lines,
	);
}

#[cfg(test)]
mod tests {
	use super::{roster_has_characters, HomeMenuChoice};
	use menu_components::MenuObjectiveKind;

	#[test]
	fn descriptions_are_nonempty() {
		for choice in HomeMenuChoice::ALL {
			assert!(!choice.description().is_empty());
		}
	}

	#[test]
	fn reliquary_is_always_coming_soon_and_locked() {
		for has_characters in [false, true] {
			let row = HomeMenuChoice::Reliquary.row(has_characters);
			assert_eq!(row.objective, Some(MenuObjectiveKind::ComingSoon));
			assert!(row.locked);
			assert!(row.subtext.is_none());
		}
	}

	#[test]
	fn training_ground_is_always_temporarily_removed_and_locked() {
		for has_characters in [false, true] {
			let row = HomeMenuChoice::TrainingGround.row(has_characters);
			assert_eq!(row.objective, Some(MenuObjectiveKind::TemporarilyRemoved));
			assert!(row.locked);
			assert!(row.subtext.is_none());
		}
	}

	#[test]
	fn empty_roster_onboards_from_characters() {
		let characters = HomeMenuChoice::Characters.row(false);
		assert_eq!(characters.objective, Some(MenuObjectiveKind::StartHere));
		assert!(!characters.locked);

		let discovery = HomeMenuChoice::Discovery.row(false);
		assert_eq!(discovery.objective, Some(MenuObjectiveKind::NeedsCharacter));
		assert!(discovery.locked);
		assert!(discovery.subtext.is_none());

		let settings = HomeMenuChoice::Settings.row(false);
		assert!(settings.objective.is_none());
		assert!(!settings.locked);
	}

	#[test]
	fn saved_roster_clears_start_here_and_needs_character() {
		assert!(HomeMenuChoice::Characters.row(true).objective.is_none());
		assert!(!HomeMenuChoice::Characters.row(true).locked);

		let discovery = HomeMenuChoice::Discovery.row(true);
		assert!(discovery.objective.is_none());
		assert!(!discovery.locked);
	}

	#[test]
	fn missing_save_root_is_an_empty_roster() {
		assert!(!roster_has_characters(None));
	}

	#[test]
	fn showing_home_also_requests_an_icon_cache_prune() {
		let mut world = bevy::prelude::World::new();
		let mut commands = world.commands();
		super::request_show_home(&mut commands);
		world.flush();
		assert_eq!(world.query::<&super::RequestShowHome>().iter(&world).count(), 1);
		assert_eq!(world.query::<&item_icons::RequestPruneItemIconCache>().iter(&world).count(), 1);
	}
}
