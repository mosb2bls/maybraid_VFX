//! Kaller species definition.
//!
//! Kispar-scale (~2 ft) sparrow body with meerkat head, fixed robrek snout, and
//! a fixed harrowed crown. Reptilian olive/moss palette. Overall size via
//! body-rig asset normalization (~0.30×).

pub mod assets;
pub mod recipe;
pub use recipe::Kaller;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, ClothingLayer,
};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};

pub use assets::{KallerHeadMesh, KallerHornMesh, KallerSnoutMesh};
pub use palette::{KallerCrownColor, KallerEyeColor, KallerPlumageColor, KallerSnoutColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KallerColors {
	pub plumage: KallerPlumageColor,
	pub eyes: KallerEyeColor,
	pub snout: KallerSnoutColor,
	pub crown: KallerCrownColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for KallerColors {
	fn default() -> Self {
		Self {
			plumage: KallerPlumageColor::Olive,
			eyes: KallerEyeColor::Amber,
			snout: KallerSnoutColor::Horn,
			crown: KallerCrownColor::Charcoal,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl KallerColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig | Hair => self.plumage.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.snout.color(),
			Horns => self.crown.color(),
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
pub struct KallerConfig {
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: KallerColors,
}

impl Default for KallerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl KallerConfig {
	pub fn default_preview() -> Self {
		Self {
			eye: EyeMesh::Falcon,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: KallerColors::default(),
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
			"kaller eye={} hair={} clothing={} plumage={} eyes={} snout_color={} crown_color={}",
			self.eye.label(),
			self.hair.label(),
			clothing,
			self.colors.plumage.label(),
			self.colors.eyes.label(),
			self.colors.snout.label(),
			self.colors.crown.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for KallerConfig {
	type Components = Kaller;

	fn components(&self) -> Self::Components {
		Kaller::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::SPARROW,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
