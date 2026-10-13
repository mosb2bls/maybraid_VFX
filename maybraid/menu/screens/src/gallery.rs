//! Character gallery: pick a saved character or start a new one.

use bevy::prelude::*;
use bevy::scene::prelude::{bsn, Scene};
use character_model_user::{list_summaries, CharacterSummary};
use character_persist::{load_active, CharacterId, SaveRoot};
use maybraid_menu_controller::MenuController;
use menu_components::info::description::TextMenuDescription;
use menu_components::single_select::republish_menu_activate;
use menu_components::{
	screen_back_scene, screen_edit_scene, MenuObjectiveKind, TextCursorColumn, TextCursorRow,
	TextMenuPlugin,
};

use crate::input::add_menu_input;
use crate::show::take_menu_show_request;
use crate::MenuScreen;

/// Queue a gallery spawn (despawns any existing menu screen first).
#[derive(Component, Debug, Clone, Copy)]
pub struct RequestShowGallery;

/// Marker on the spawned gallery root.
#[derive(Component, Debug, Default, Clone, Copy)]
pub struct GalleryScreen;

/// Gallery destinations. Each pickable row stamps this as a component.
#[derive(Clone, Copy, Debug, Default, Message, Component, PartialEq, Eq)]
pub enum GalleryChoice {
	#[default]
	New,
	Select(CharacterId),
}

pub fn request_show_gallery(commands: &mut Commands) {
	commands.spawn(RequestShowGallery);
}

pub struct GalleryScreenPlugin;

impl Plugin for GalleryScreenPlugin {
	fn build(&self, app: &mut App) {
		add_menu_input(app);
		app.add_plugins(TextMenuPlugin::<GalleryChoice>::default())
			.add_systems(Update, apply_show_gallery);
	}
}

fn apply_show_gallery(
	mut commands: Commands,
	requests: Query<Entity, With<RequestShowGallery>>,
	existing: Query<Entity, With<MenuScreen>>,
	save_root: Option<Res<SaveRoot>>,
) {
	if !take_menu_show_request(&mut commands, &requests, &existing) {
		return;
	}
	let summaries = save_root.as_ref().map(|root| list_summaries(root)).unwrap_or_default();
	let active = save_root.as_ref().and_then(|root| load_active(root));
	commands.spawn_scene(gallery_scene(&summaries, active));
}

/// Index in the gallery column: 0 is Create a Character, then saved rows in order.
fn gallery_selected_index(summaries: &[CharacterSummary], active: Option<CharacterId>) -> usize {
	active
		.and_then(|id| summaries.iter().position(|summary| summary.id == id))
		.map(|index| index + 1)
		.unwrap_or(0)
}

impl GalleryChoice {
	pub fn create_row(empty_roster: bool) -> TextCursorRow<Self> {
		let row = TextCursorRow::new("Create a Character", Self::New);
		if empty_roster {
			row.with_objective(MenuObjectiveKind::StartHere)
		} else {
			row
		}
	}
}

fn gallery_rows(
	summaries: &[CharacterSummary],
	active: Option<CharacterId>,
) -> Vec<TextCursorRow<GalleryChoice>> {
	let mut rows = vec![GalleryChoice::create_row(summaries.is_empty())];
	rows.extend(summaries.iter().map(|summary| {
		let row = TextCursorRow::new(summary.name.clone(), GalleryChoice::Select(summary.id))
			.with_subtext(summary.species_title);
		if Some(summary.id) == active {
			row.with_objective(MenuObjectiveKind::Selected)
		} else {
			row.with_hidden_objective(MenuObjectiveKind::Selected)
		}
	}));
	rows
}

fn gallery_scene(
	summaries: &[CharacterSummary],
	active: Option<CharacterId>,
) -> impl Scene + 'static {
	let rows = gallery_rows(summaries, active);
	let selected = gallery_selected_index(summaries, active);
	let children: Vec<Box<dyn Scene>> = vec![
		Box::new(
			TextCursorColumn::rows("Characters", rows)
				.with_selected(selected)
				.scrollable()
				.scene(),
		),
		Box::new(TextMenuDescription::scene(String::new())),
		Box::new(screen_back_scene()),
		Box::new(screen_edit_scene()),
	];
	bsn! {
		GalleryScreen
		MenuScreen
		MenuController
		BackgroundColor(Color::NONE)
		Node {
			width: percent(100),
			height: percent(100),
		}
		Pickable::IGNORE
		on(republish_menu_activate::<GalleryChoice>)
		Children [ {children} ]
	}
}

#[cfg(test)]
mod tests {
	use super::{gallery_rows, gallery_selected_index, GalleryChoice};
	use character_model_user::CharacterSummary;
	use character_persist::CharacterId;
	use menu_components::MenuObjectiveKind;

	#[test]
	fn new_is_the_default_row() {
		assert_eq!(GalleryChoice::default(), GalleryChoice::New);
		assert_ne!(GalleryChoice::Select(CharacterId(1)), GalleryChoice::New);
	}

	#[test]
	fn selected_row_follows_the_active_character() {
		let jeff = CharacterId(1);
		let unnamed = CharacterId(2);
		let summaries = [
			CharacterSummary { id: jeff, name: "Jeff".into(), species_title: "Braidman" },
			CharacterSummary { id: unnamed, name: "Unnamed".into(), species_title: "Braidman" },
		];
		assert_eq!(gallery_selected_index(&summaries, Some(jeff)), 1);
		assert_eq!(gallery_selected_index(&summaries, Some(unnamed)), 2);
		assert_eq!(gallery_selected_index(&summaries, None), 0);
		assert_eq!(gallery_selected_index(&summaries, Some(CharacterId(99))), 0);
	}

	#[test]
	fn empty_roster_marks_create_a_character() {
		let row = GalleryChoice::create_row(true);
		assert_eq!(row.label, "Create a Character");
		assert_eq!(row.objective, Some(MenuObjectiveKind::StartHere));
		assert!(!row.locked);
		assert!(row.subtext.is_none());
		assert!(gallery_rows(&[], None).len() == 1);
	}

	#[test]
	fn saved_roster_drops_the_start_here_badge() {
		let summaries = [CharacterSummary {
			id: CharacterId(1),
			name: "Jeff".into(),
			species_title: "Braidman",
		}];
		let rows = gallery_rows(&summaries, None);
		assert!(rows[0].objective.is_none());
		assert_eq!(rows[0].label, "Create a Character");
		assert_eq!(rows[1].subtext.as_deref(), Some("Braidman"));
		assert_eq!(rows[1].objective, Some(MenuObjectiveKind::Selected));
		assert!(!rows[1].objective_visible);
	}

	#[test]
	fn selected_chip_is_visible_only_on_the_active_row() {
		let jeff = CharacterId(1);
		let unnamed = CharacterId(2);
		let summaries = [
			CharacterSummary { id: jeff, name: "Jeff".into(), species_title: "Braidman" },
			CharacterSummary { id: unnamed, name: "Unnamed".into(), species_title: "Braidman" },
		];
		let rows = gallery_rows(&summaries, Some(jeff));
		assert!(rows[0].objective.is_none());
		assert_eq!(rows[1].objective, Some(MenuObjectiveKind::Selected));
		assert!(rows[1].objective_visible);
		assert_eq!(rows[2].objective, Some(MenuObjectiveKind::Selected));
		assert!(!rows[2].objective_visible);
	}
}
