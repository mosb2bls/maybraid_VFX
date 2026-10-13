//! Wumbus species definition.
//!
//! Bearlike humanoid: ortho bear head on the Wumbus body, flank ears, canine snout,
//! dark fur colors with lighter contrasting features, and optional harrowed crown.

pub mod assets;
pub mod recipe;
pub use recipe::Wumbus;
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

pub use assets::{WumbusHeadMesh, WumbusHornMesh, WumbusMouthMesh};
pub use palette::{
	WumbusEarColor, WumbusEyeColor, WumbusHornColor, WumbusMouthColor, WumbusSkinColor,
	WumbusSpineColor,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WumbusColors {
	pub skin: WumbusSkinColor,
	pub eyes: WumbusEyeColor,
	pub ears: WumbusEarColor,
	pub mouth: WumbusMouthColor,
	pub horns: WumbusHornColor,
	pub spine: WumbusSpineColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for WumbusColors {
	fn default() -> Self {
		Self {
			skin: WumbusSkinColor::Chocolate,
			eyes: WumbusEyeColor::PaleBlue,
			ears: WumbusEarColor::Cream,
			mouth: WumbusMouthColor::Blush,
			horns: WumbusHornColor::Ivory,
			spine: WumbusSpineColor::Ivory,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl WumbusColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig => self.skin.color(),
			Horns => self.horns.color(),
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
pub struct WumbusConfig {
	pub horns: WumbusHornMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: WumbusColors,
}

impl Default for WumbusConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl WumbusConfig {
	pub fn default_preview() -> Self {
		Self {
			horns: WumbusHornMesh::None,
			eye: EyeMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: WumbusColors::default(),
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
			"wumbus horns={} eye={} hair={} clothing={} skin={} eyes={} ears={} horn_color={} hair_color={}",
			self.horns.label(),
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.horns.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for WumbusConfig {
	type Components = Wumbus;

	fn components(&self) -> Self::Components {
		Wumbus::from_config(self)
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
