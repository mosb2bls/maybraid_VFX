//! Topple species definition.
//!
//! Softer, taller Tipple sibling (~2 ft): whelp body, cartoonishly large meerkat
//! head, selectable beak, pastel plumage. Overall size via body-rig asset
//! normalization (~0.30×).

pub mod assets;
pub mod recipe;
pub use recipe::Topple;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, ClothingLayer,
};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};

pub use assets::{ToppleBeakMesh, ToppleHeadMesh};
pub use palette::{ToppleBeakColor, ToppleEyeColor, TopplePlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToppleColors {
	pub plumage: TopplePlumageColor,
	pub eyes: ToppleEyeColor,
	pub beak: ToppleBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for ToppleColors {
	fn default() -> Self {
		Self {
			plumage: TopplePlumageColor::Cream,
			eyes: ToppleEyeColor::SoftAmber,
			beak: ToppleBeakColor::Peach,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl ToppleColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig | Hair => self.plumage.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.beak.color(),
			_ => self.plumage.color(),
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToppleConfig {
	pub beak: ToppleBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: ToppleColors,
}

impl Default for ToppleConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl ToppleConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: ToppleBeakMesh::Beak,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: ToppleColors::default(),
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
			"topple beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
			self.beak.label(),
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.plumage.label(),
			self.colors.eyes.label(),
			self.colors.beak.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for ToppleConfig {
	type Components = Topple;

	fn components(&self) -> Self::Components {
		Topple::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::WHELP,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
