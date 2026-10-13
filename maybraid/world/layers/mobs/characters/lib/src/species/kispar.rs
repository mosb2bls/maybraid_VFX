//! Kispar species definition.
//!
//! Kite-like ~2 ft bird: sparrow body silhouette on the humanoid rig, meerkat
//! head, selectable beak, soft hawk plumage. Overall size via body-rig asset
//! normalization (~0.30×).

pub mod assets;
pub mod recipe;
pub use recipe::Kispar;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, ClothingLayer,
};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};

pub use assets::{KisparBeakMesh, KisparHeadMesh};
pub use palette::{KisparBeakColor, KisparEyeColor, KisparPlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KisparColors {
	pub plumage: KisparPlumageColor,
	pub eyes: KisparEyeColor,
	pub beak: KisparBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for KisparColors {
	fn default() -> Self {
		Self {
			plumage: KisparPlumageColor::Ash,
			eyes: KisparEyeColor::SoftAmber,
			beak: KisparBeakColor::Horn,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl KisparColors {
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
pub struct KisparConfig {
	pub beak: KisparBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: KisparColors,
}

impl Default for KisparConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl KisparConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: KisparBeakMesh::Hook,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: KisparColors::default(),
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
			"kispar beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
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

impl CharacterRecipe for KisparConfig {
	type Components = Kispar;

	fn components(&self) -> Self::Components {
		Kispar::from_config(self)
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
