//! Spibmom species definition.
//!
//! Meerkat-headed Wumbus body with a long neck, snail-back spine, finbone crown,
//! small flank ears, and igny snout. Soft blue skin with light contrasting accents.

pub mod assets;
pub mod recipe;
pub use recipe::Spibmom;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, ClothingLayer,
};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};
use serde::{Deserialize, Serialize};

pub use assets::{SpibmomCrownMesh, SpibmomHeadMesh, SpibmomMouthMesh};
pub use palette::{
	SpibmomCrownColor, SpibmomEarColor, SpibmomEyeColor, SpibmomMouthColor, SpibmomSkinColor,
	SpibmomSpineColor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpibmomColors {
	pub skin: SpibmomSkinColor,
	pub eyes: SpibmomEyeColor,
	pub ears: SpibmomEarColor,
	pub mouth: SpibmomMouthColor,
	pub crown: SpibmomCrownColor,
	pub spine: SpibmomSpineColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for SpibmomColors {
	fn default() -> Self {
		Self {
			skin: SpibmomSkinColor::PowderBlue,
			eyes: SpibmomEyeColor::Pearl,
			ears: SpibmomEarColor::Umber,
			mouth: SpibmomMouthColor::Espresso,
			crown: SpibmomCrownColor::Charcoal,
			spine: SpibmomSpineColor::Charcoal,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl SpibmomColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig => self.skin.color(),
			Horns => self.crown.color(),
			Spine => self.spine.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			EarLeft | EarRight => self.ears.color(),
			Mouth => self.mouth.color(),
			Hair => self.hair.color(),
			_ => self.skin.color(),
		}
	}

	pub fn clothing_color(&self, clothing: ClothingMesh) -> ItemColor {
		ClothingColor::resolve(&self.clothing, self.clothing_default, clothing)
	}

	pub fn set_clothing_color(&mut self, clothing: ClothingMesh, color: ItemColor) {
		ClothingColor::set(&mut self.clothing, clothing, color);
	}

	pub fn clothing_material_for(&self, clothing: ClothingMesh) -> ClothingMaterial {
		ClothingMaterialChoice::resolve(&self.clothing_materials, self.clothing_material, clothing)
	}

	pub fn set_clothing_material(&mut self, clothing: ClothingMesh, material: ClothingMaterial) {
		ClothingMaterialChoice::set(&mut self.clothing_materials, clothing, material);
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SpibmomConfig {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: SpibmomColors,
}

impl Default for SpibmomConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl SpibmomConfig {
	pub fn default_preview() -> Self {
		Self {
			eye: EyeMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: SpibmomColors::default(),
		}
	}

	pub fn status_label(&self) -> String {
		let clothing = if self.clothing.is_empty() {
			"none".into()
		} else {
			self.clothing
				.iter()
				.map(|clothing| clothing.label())
				.collect::<Vec<_>>()
				.join(",")
		};
		format!(
			"spibmom eye={} hair={} clothing={} skin={} eyes={} ears={} mouth={} crown={} spine={} hair_color={}",
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.mouth.label(),
			self.colors.crown.label(),
			self.colors.spine.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for SpibmomConfig {
	type Components = Spibmom;

	fn components(&self) -> Self::Components {
		Spibmom::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::WUMBUS,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
