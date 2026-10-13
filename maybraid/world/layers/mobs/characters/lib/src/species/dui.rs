//! Dui species definition.
//!
//! Tall slender humanoid: barred bowl head on the Igeo body, thorn horns as eyes,
//! optional t-bar nose, small common mouth, no ears, and soft earth-tone skin colors.

pub mod assets;
pub mod recipe;
pub use recipe::Dui;
pub mod palette;
pub mod pose;

use crate::{species::common::HairMesh, CharacterRecipe, ClothingLayer};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};
use serde::{Deserialize, Serialize};

pub use assets::{DuiEyeMesh, DuiHeadMesh, DuiMouthMesh, DuiNoseMesh};
pub use palette::{DuiEyeColor, DuiMouthColor, DuiNoseColor, DuiSkinColor};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuiColors {
	pub skin: DuiSkinColor,
	pub eyes: DuiEyeColor,
	pub nose_color: DuiNoseColor,
	pub mouth: DuiMouthColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for DuiColors {
	fn default() -> Self {
		Self {
			skin: DuiSkinColor::Purple,
			eyes: DuiEyeColor::Black,
			nose_color: DuiNoseColor::Black,
			mouth: DuiMouthColor::Red,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl DuiColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig => self.skin.color(),
			Nose => self.nose_color.color(),
			EyeLeft | EyeRight => self.eyes.color(),
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
pub struct DuiConfig {
	pub nose: DuiNoseMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: DuiColors,
}

impl Default for DuiConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl DuiConfig {
	pub fn default_preview() -> Self {
		Self {
			nose: DuiNoseMesh::None,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: DuiColors::default(),
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
			"dui nose={} hair={} clothing={} skin={} hair_color={}",
			self.nose.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for DuiConfig {
	type Components = Dui;

	fn components(&self) -> Self::Components {
		Dui::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::IGEO,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
