//! Grener species definition.
//!
//! Shark on the forelimbed rig (~3 m). Authoring body length is ~2 m, so
//! overall scale is 1.5× via body-rig asset normalization.

pub mod recipe;
pub use recipe::Grener;
pub mod palette;
pub mod pose;

use crate::{CharacterRecipe, ClothingLayer};

pub use palette::GrenerBodyColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrenerColors {
	pub body: GrenerBodyColor,
}

impl Default for GrenerColors {
	fn default() -> Self {
		Self { body: GrenerBodyColor::Slate }
	}
}

impl GrenerColors {
	pub fn color_for_slot(&self, _slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		self.body.color()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrenerConfig {
	pub colors: GrenerColors,
}

impl Default for GrenerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl GrenerConfig {
	pub fn default_preview() -> Self {
		Self { colors: GrenerColors::default() }
	}

	pub fn status_label(&self) -> String {
		format!("grener body={}", self.colors.body.label())
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for GrenerConfig {
	type Components = Grener;

	fn components(&self) -> Self::Components {
		Grener::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
