//! Character-sheet archetypes independent of species silhouette.

use character_items::CharacterSheet;
use characters::BuildPreset;

use crate::number::{index, FromMobNumber};

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CharacterBuild {
	#[default]
	Base,
	Wraith,
	Tank,
	Brawler,
	Renegade,
	Warrior,
	Master,
}

use bevy::prelude::Component;

impl CharacterBuild {
	pub const VALUES: [Self; 7] = [
		Self::Base,
		Self::Wraith,
		Self::Tank,
		Self::Brawler,
		Self::Renegade,
		Self::Warrior,
		Self::Master,
	];

	pub fn apply(self, mut sheet: CharacterSheet) -> CharacterSheet {
		let (health, running, jump, agility, strength, damage) = match self {
			Self::Base => (0, 0, 0, 0, 0, 0),
			Self::Wraith => (-35, 35, 20, 15, -10, 0),
			Self::Tank => (85, -25, -15, -10, 25, 0),
			Self::Brawler => (15, -5, 0, -25, 55, 8),
			Self::Renegade => (0, 15, 5, 5, 0, 12),
			Self::Warrior => (15, 10, 10, 10, 10, 8),
			Self::Master => (60, 30, 30, 30, 30, 20),
		};
		sheet.health = sheet.health.saturating_add(health).max(1);
		sheet.running = sheet.running.saturating_add(running).max(1);
		sheet.jump = sheet.jump.saturating_add(jump).max(1);
		sheet.agility = sheet.agility.saturating_add(agility).max(1);
		sheet.strength = sheet.strength.saturating_add(strength).max(1);
		sheet.damage = sheet.damage.saturating_add(damage);
		sheet
	}

	pub const fn visual_preset(self) -> BuildPreset {
		match self {
			Self::Base => BuildPreset::Average,
			Self::Wraith => BuildPreset::Lanky,
			Self::Tank => BuildPreset::Heavy,
			Self::Brawler => BuildPreset::Stocky,
			Self::Renegade | Self::Warrior | Self::Master => BuildPreset::Athletic,
		}
	}
}

impl FromMobNumber for CharacterBuild {
	fn from_num(num: f32) -> Self {
		Self::VALUES[index(num, 0xB017_D5E2, Self::VALUES.len())]
	}
}
