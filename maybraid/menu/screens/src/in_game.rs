//! In-game pause menu: centered actions plus upper-left brand / mode.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use maybraid_menu_controller::MenuController;
use menu_components::single_select::republish_menu_activate;
use menu_components::single_select::text_cursor::TextCursorColumn;
use menu_components::{
	set_brand_mode_title, BrandModeLine, BrandModeTitle, TextColumnAlign, TextColumnAnchor,
	TextCursorRow, TextMenuPlugin,
};

use crate::input::add_menu_input;
use crate::settings::InGameSettingsPlugin;
use crate::show::take_menu_show_request;
use crate::{GameMode, MenuScreen};

/// Queue an in-game menu spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct RequestShowInGame {
	pub mode: Option<String>,
	/// Row under the cursor when the menu opens. `None` is the first row.
	pub selected: Option<InGameMenuChoice>,
}

/// Marker on the spawned in-game menu root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct InGameScreen;

/// Pause-menu destinations. Each pickable row stamps this as a component.
///
/// In-screen: [`menu_components::MenuFocus<Self>`] /
/// [`menu_components::MenuActivate<Self>`] bubble to this root. Outside the
/// screen: [`republish_menu_activate`] copies activate as a [`Message`].
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum InGameMenuChoice {
	#[default]
	Character,
	Records,
	Help,
	Settings,
	Leave,
}

impl InGameMenuChoice {
	pub const ALL: [Self; 5] =
		[Self::Character, Self::Records, Self::Help, Self::Settings, Self::Leave];

	pub fn label(self) -> &'static str {
		match self {
			Self::Character => "Character",
			Self::Records => "Records",
			Self::Help => "Help",
			Self::Settings => "Settings",
			Self::Leave => "Leave",
		}
	}

	pub fn rows() -> Vec<TextCursorRow<Self>> {
		Self::ALL
			.into_iter()
			.map(|choice| TextCursorRow::new(choice.label(), choice))
			.collect()
	}
}

impl InGameScreen {
	pub fn scene(mode: &GameMode, selected: Option<InGameMenuChoice>) -> impl Scene + 'static {
		let rows = InGameMenuChoice::rows();
		let selected = selected
			.and_then(|selected| rows.iter().position(|row| row.action == selected))
			.unwrap_or(0);
		let children: Vec<Box<dyn Scene>> = vec![
			Box::new(
				TextCursorColumn::untitled_rows(rows)
					.anchored(TextColumnAnchor::Center)
					.aligned(TextColumnAlign::Center)
					.with_selected(selected)
					.scene(),
			),
			Box::new(BrandModeLine::new(mode.label.clone()).scene()),
		];
		bsn! {
			InGameScreen
			MenuScreen
			MenuController
			Node {
				width: percent(100),
				height: percent(100),
				justify_content: JustifyContent::Center,
				align_items: AlignItems::Center,
			}
			Pickable::IGNORE
			on(republish_menu_activate::<InGameMenuChoice>)
			Children [ {children} ]
		}
	}
}

pub fn request_show_in_game(commands: &mut Commands) {
	commands.spawn(RequestShowInGame { mode: None, selected: None });
}

pub fn request_show_in_game_with_mode(commands: &mut Commands, mode: impl Into<String>) {
	commands.spawn(RequestShowInGame { mode: Some(mode.into()), selected: None });
}

/// Re-show the pause menu with `selected` under the cursor.
pub fn request_show_in_game_at(commands: &mut Commands, selected: InGameMenuChoice) {
	commands.spawn(RequestShowInGame { mode: None, selected: Some(selected) });
}

pub struct InGameScreenPlugin;

impl Plugin for InGameScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.init_resource::<GameMode>()
			.add_plugins(TextMenuPlugin::<InGameMenuChoice>::default())
			.add_plugins(InGameSettingsPlugin)
			.add_systems(Update, (apply_show_in_game, sync_in_game_brand));
	}
}

fn apply_show_in_game(
	mut commands: Commands,
	requests: Query<(Entity, &RequestShowInGame)>,
	existing: Query<Entity, With<MenuScreen>>,
	mut mode: ResMut<GameMode>,
) {
	let Some((_, request)) = requests.iter().last() else {
		return;
	};
	if let Some(label) = request.mode.as_ref() {
		mode.label.clone_from(label);
	}
	let selected = request.selected;
	if !take_menu_show_request(&mut commands, requests.iter().map(|(entity, _)| entity), &existing)
	{
		return;
	}
	commands.spawn_scene(InGameScreen::scene(&mode, selected));
}

fn sync_in_game_brand(
	mode: Res<GameMode>,
	screens: Query<Entity, With<InGameScreen>>,
	children: Query<&Children>,
	mut titles: Query<&mut Text, With<BrandModeTitle>>,
) {
	if !mode.is_changed() {
		return;
	}
	let title = mode.title();
	for screen in &screens {
		set_brand_mode_title(screen, title.clone(), &children, &mut titles);
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn actions(rows: &[TextCursorRow<InGameMenuChoice>]) -> Vec<InGameMenuChoice> {
		rows.iter().map(|row| row.action).collect()
	}

	#[test]
	fn labels_are_nonempty() {
		for choice in InGameMenuChoice::ALL {
			assert!(!choice.label().is_empty());
		}
	}

	#[test]
	fn every_pause_row_is_pickable() {
		let rows = InGameMenuChoice::rows();
		assert_eq!(actions(&rows), InGameMenuChoice::ALL.to_vec());
		assert!(rows.iter().all(|row| !row.locked && row.subtext.is_none()));
	}
}
