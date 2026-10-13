//! Lero species definition.
//!
//! Reptilian humanoid: ortho tee head on the Leron body, lerodon tail and spine,
//! lerodon or robrek snout, faded green and red scales, and light accent colors.

pub mod assets;
pub mod recipe;
pub use recipe::Lero;
pub mod palette;
pub mod pose;

use crate::{species::common::HairMesh, CharacterRecipe, ClothingLayer};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};
use serde::{Deserialize, Serialize};

pub use assets::{LeroHeadMesh, LeroMouthMesh};
pub use palette::{LeroEyeColor, LeroMouthColor, LeroSkinColor, LeroSpineColor, LeroTailColor};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LeroColors {
	pub skin: LeroSkinColor,
	pub eyes: LeroEyeColor,
	pub mouth: LeroMouthColor,
	pub tail: LeroTailColor,
	pub spine: LeroSpineColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for LeroColors {
	fn default() -> Self {
		Self {
			skin: LeroSkinColor::FadedGreen,
			eyes: LeroEyeColor::Gold,
			mouth: LeroMouthColor::SoftBlush,
			tail: LeroTailColor::Pearl,
			spine: LeroSpineColor::Pearl,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl LeroColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig => self.skin.color(),
			Mouth => self.mouth.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Tail => self.tail.color(),
			Spine => self.spine.color(),
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
pub struct LeroConfig {
	pub mouth: LeroMouthMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: LeroColors,
}

impl Default for LeroConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl LeroConfig {
	pub fn default_preview() -> Self {
		Self {
			mouth: LeroMouthMesh::Lerodon,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: LeroColors::default(),
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
			"lero mouth={} hair={} clothing={} skin={} eyes={} snout={} tail={} spine={} hair_color={}",
			self.mouth.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.mouth.label(),
			self.colors.tail.label(),
			self.colors.spine.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for LeroConfig {
	type Components = Lero;

	fn components(&self) -> Self::Components {
		Lero::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::LERON,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
