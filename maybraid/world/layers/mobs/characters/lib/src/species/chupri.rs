//! Chupri species definition.
//!
//! Tiny bipedal bird sibling of Lidder: crane body on the humanoid rig at authored
//! proportions, meerkat head, beak in the mouth slot, plumage-tinted featherhawk.
//! Overall size is about one foot via body-rig asset normalization (~0.15× the
//! ~2 m biped), not per-bone root scale (pelvis/buttocks are root siblings).

pub mod assets;
pub mod recipe;
pub use recipe::Chupri;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EyeMesh, HairMesh},
	CharacterRecipe, ClothingLayer,
};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};

pub use assets::{ChupriBeakMesh, ChupriHeadMesh};
pub use palette::{ChupriBeakColor, ChupriEyeColor, ChupriPlumageColor};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChupriColors {
	pub plumage: ChupriPlumageColor,
	pub eyes: ChupriEyeColor,
	pub beak: ChupriBeakColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for ChupriColors {
	fn default() -> Self {
		Self {
			plumage: ChupriPlumageColor::Magenta,
			eyes: ChupriEyeColor::Turquoise,
			beak: ChupriBeakColor::Tangerine,
			// Crest tint follows plumage; this field is only for shared hair menus.
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl ChupriColors {
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
pub struct ChupriConfig {
	pub beak: ChupriBeakMesh,
	pub eye: EyeMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: ChupriColors,
}

impl Default for ChupriConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl ChupriConfig {
	pub fn default_preview() -> Self {
		Self {
			beak: ChupriBeakMesh::Beak,
			eye: EyeMesh::Falcon,
			hair: HairMesh::FeatherHawk,
			clothing: Vec::new(),
			colors: ChupriColors::default(),
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
			"chupri beak={} eye={} hair={} clothing={} plumage={} eyes={} beak_color={}",
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

impl CharacterRecipe for ChupriConfig {
	type Components = Chupri;

	fn components(&self) -> Self::Components {
		Chupri::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::CRANE,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
