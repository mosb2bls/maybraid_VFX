//! Mistler species definition.
//!
//! Sprite fish on the forelimbed rig (~1 ft). Authoring body length is ~2 m, so
//! overall scale is ~0.15× via body-rig asset normalization.

pub mod recipe;
pub use recipe::Mistler;
pub mod palette;
pub mod pose;

use crate::{CharacterRecipe, ClothingLayer};

pub use palette::MistlerBodyColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MistlerColors {
	pub body: MistlerBodyColor,
}

impl Default for MistlerColors {
	fn default() -> Self {
		Self { body: MistlerBodyColor::Coral }
	}
}

impl MistlerColors {
	pub fn color_for_slot(&self, _slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		self.body.color()
	}
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MistlerConfig {
	pub colors: MistlerColors,
}

impl Default for MistlerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl MistlerConfig {
	pub fn default_preview() -> Self {
		Self { colors: MistlerColors::default() }
	}

	pub fn status_label(&self) -> String {
		format!("mistler body={}", self.colors.body.label())
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for MistlerConfig {
	type Components = Mistler;

	fn components(&self) -> Self::Components {
		Mistler::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
