//! Brodler species definition.
//!
//! Cartoon humanoid: shared Standard body mesh, Gaunt/Full heads, crown horns,
//! species-owned skin/eye colors, and shared hair/clothing catalogs.

pub mod assets;
pub mod recipe;
pub use recipe::Brodler;
pub mod palette;
pub mod pose;

use crate::{
	species::common::{EarMesh, EyeMesh, HairMesh, MouthMesh, NoseMesh},
	CharacterRecipe, ClothingLayer,
};

use character_items::{
	ClothingColor, ClothingHost, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub use assets::HornMesh;
pub use palette::{BrodlerEyeColor, BrodlerHornColor, BrodlerSkinColor};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BrodlerHeadMesh {
	#[default]
	Gaunt,
	Full,
}

impl BrodlerHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Gaunt, Self::Full];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Gaunt => "gaunt",
			Self::Full => "full",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		use crate::species::common::{HEAD_FULL, HEAD_GAUNT};
		match self {
			Self::Gaunt => HEAD_GAUNT,
			Self::Full => HEAD_FULL,
		}
	}
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BrodlerColors {
	pub skin: BrodlerSkinColor,
	pub eyes: BrodlerEyeColor,
	pub horns: BrodlerHornColor,
	pub mouth: ItemColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for BrodlerColors {
	fn default() -> Self {
		Self {
			skin: BrodlerSkinColor::Crimson,
			eyes: BrodlerEyeColor::LightBlue,
			horns: BrodlerHornColor::LightBrown,
			mouth: ItemColor::Natural,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl BrodlerColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | HeadMesh | HeadRig | Nose | EarLeft | EarRight => self.skin.color(),
			Horns => self.horns.color(),
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
pub struct BrodlerConfig {
	pub head: BrodlerHeadMesh,
	pub horns: HornMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: BrodlerColors,
}

impl Default for BrodlerConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl BrodlerConfig {
	pub fn default_preview() -> Self {
		Self {
			head: BrodlerHeadMesh::Gaunt,
			horns: HornMesh::HarrowedCrown,
			eye: EyeMesh::Standard,
			nose: NoseMesh::Standard,
			mouth: MouthMesh::Standard,
			ear: EarMesh::Flank,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: BrodlerColors::default(),
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
			"brodler head={} horns={} eye={} nose={} mouth={} ear={} hair={} clothing={} skin={} eyes={} horn_color={} hair_color={}",
			self.head.label(),
			self.horns.label(),
			self.eye.label(),
			self.nose.label(),
			self.mouth.label(),
			self.ear.label(),
			self.hair.label(),
			clothing,
			self.colors.skin.label(),
			self.colors.eyes.label(),
			self.colors.horns.label(),
			self.colors.hair.label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for BrodlerConfig {
	type Components = Brodler;

	fn components(&self) -> Self::Components {
		Brodler::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			ClothingHost::HUMANOID,
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
