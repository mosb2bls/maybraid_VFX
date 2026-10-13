//! Thumplus species definition.
//!
//! Whale on the forelimbed rig (~6 m). Authoring body length is ~2 m, so
//! overall scale is 3× via body-rig asset normalization.

pub mod recipe;
pub use recipe::Thumplus;
pub mod palette;
pub mod pose;

use crate::{CharacterRecipe, ClothingLayer};

pub use palette::ThumplusBodyColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumplusColors {
	pub body: ThumplusBodyColor,
}

impl Default for ThumplusColors {
	fn default() -> Self {
		Self { body: ThumplusBodyColor::Ocean }
	}
}

impl ThumplusColors {
	pub fn color_for_slot(&self, _slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		self.body.color()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThumplusConfig {
	pub colors: ThumplusColors,
}

impl Default for ThumplusConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl ThumplusConfig {
	pub fn default_preview() -> Self {
		Self { colors: ThumplusColors::default() }
	}

	pub fn status_label(&self) -> String {
		format!("thumplus body={}", self.colors.body.label())
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for ThumplusConfig {
	type Components = Thumplus;

	fn components(&self) -> Self::Components {
		Thumplus::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
