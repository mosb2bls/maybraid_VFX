//! Character appearance as a 1:1 Bevy relationship, persisted beside inventory.

use bevy::prelude::*;
use character_persist::{CharacterId, PersistError, SaveRoot};
use characters::CharacterAppearance;
use serde::{Deserialize, Serialize};
use std::fs;

const VERSION: u32 = 1;
pub const UNNAMED_CHARACTER: &str = "Unnamed";

/// Capsule/session using a character appearance record.
///
/// 1:1 onto the model entity (`model`). Inserting it stamps [`ModeledBy`].
#[derive(Component, Debug, Clone, Copy)]
#[relationship(relationship_target = ModeledBy)]
pub struct CharacterModelUser {
	#[relationship]
	pub model: Entity,
}

impl CharacterModelUser {
	pub fn of(model: Entity) -> Self {
		Self { model }
	}
}

/// Model-side 1:1 target of [`CharacterModelUser`].
#[derive(Component, Debug)]
#[relationship_target(relationship = CharacterModelUser)]
pub struct ModeledBy(Entity);

/// Live appearance record on the model entity.
#[derive(Component, Clone, Debug, PartialEq)]
pub struct CharacterModel {
	pub id: CharacterId,
	pub name: String,
	pub appearance: CharacterAppearance,
}

impl CharacterModel {
	pub fn new(id: CharacterId, name: impl Into<String>, appearance: CharacterAppearance) -> Self {
		let mut appearance = appearance;
		appearance.strip_clothing();
		Self { id, name: saved_name(&name.into()), appearance }
	}
}

pub fn saved_name(name: &str) -> String {
	let name = name.trim();
	if name.is_empty() {
		UNNAMED_CHARACTER.into()
	} else {
		name.into()
	}
}

/// Row for the character gallery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CharacterSummary {
	pub id: CharacterId,
	pub name: String,
	pub species_title: &'static str,
}

pub struct CharacterModelUserPlugin;

impl Plugin for CharacterModelUserPlugin {
	fn build(&self, _app: &mut App) {}
}

#[derive(Serialize, Deserialize)]
struct CharacterFile {
	version: u32,
	id: CharacterId,
	name: String,
	appearance: CharacterAppearance,
}

/// Write `characters/{id}.json`. Clothing on the appearance is stripped first.
pub fn save(root: &SaveRoot, model: &CharacterModel) -> Result<(), PersistError> {
	root.ensure_dirs()?;
	let mut appearance = model.appearance.clone();
	appearance.strip_clothing();
	let file =
		CharacterFile { version: VERSION, id: model.id, name: saved_name(&model.name), appearance };
	let json = serde_json::to_string_pretty(&file)?;
	fs::write(root.character_path(model.id), json)?;
	Ok(())
}

pub fn load(root: &SaveRoot, id: CharacterId) -> Result<CharacterModel, PersistError> {
	let json = fs::read_to_string(root.character_path(id))?;
	let mut file: CharacterFile = serde_json::from_str(&json)?;
	file.appearance.strip_clothing();
	Ok(CharacterModel { id: file.id, name: saved_name(&file.name), appearance: file.appearance })
}

pub fn list_summaries(root: &SaveRoot) -> Vec<CharacterSummary> {
	let Ok(ids) = root.list_ids() else {
		return Vec::new();
	};
	let mut summaries = Vec::new();
	for id in ids {
		let Ok(model) = load(root, id) else {
			continue;
		};
		summaries.push(CharacterSummary {
			id: model.id,
			name: model.name,
			species_title: model.appearance.species_title(),
		});
	}
	summaries
}

pub fn spawn_model(commands: &mut Commands, host: Entity, model: CharacterModel) -> Entity {
	let model_entity = commands.spawn(model).id();
	commands.entity(host).insert(CharacterModelUser::of(model_entity));
	model_entity
}

#[cfg(test)]
mod tests {
	use super::*;
	use characters::species::braidman::BraidmanConfig;

	#[test]
	fn braidman_round_trips_without_clothing() {
		let dir = tempfile::tempdir().expect("tempdir");
		let root = SaveRoot::at(dir.path());
		let mut config = BraidmanConfig::default_preview();
		config.clothing.push(character_items::ClothingMesh::Pants);
		let model = CharacterModel::new(
			CharacterId(11),
			"  Misty  ",
			CharacterAppearance::Braidman(config),
		);
		assert!(match &model.appearance {
			CharacterAppearance::Braidman(config) => config.clothing.is_empty(),
			_ => false,
		});
		save(&root, &model).expect("save");
		let loaded = load(&root, model.id).expect("load");
		assert_eq!(loaded.name, "Misty");
		assert_eq!(loaded.appearance.species_id(), "braidman");
		assert_eq!(loaded.appearance, model.appearance);
		let listed = list_summaries(&root);
		assert_eq!(listed.len(), 1);
		assert_eq!(listed[0].name, "Misty");
		assert_eq!(listed[0].species_title, "Braidman");
	}

	#[test]
	fn empty_name_becomes_unnamed() {
		assert_eq!(saved_name("   "), UNNAMED_CHARACTER);
	}
}
