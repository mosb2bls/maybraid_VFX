//! Simplified Braidman species definition.
//!
//! This module is deliberately lean. It validates the organizational pattern for
//! species-owned assets, baseline proportions, presets, and slider resolution
//! before the full Braidman matrix is implemented.

pub mod recipe;
pub use recipe::Braidman;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::{BodyMesh, EarMesh, EyeMesh, HairMesh, HeadMesh, MouthMesh, NoseMesh};
use character_items::{
	ClothingColor, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};
use serde::{Deserialize, Serialize};
use sliders::BraidmanSliders;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BraidmanColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub nose: ItemColor,
	pub mouth: ItemColor,
	pub ears: ItemColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for BraidmanColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: body,
			eyes: ItemColor::Blue,
			nose: body,
			mouth: ItemColor::Warm,
			ears: body,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl BraidmanColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh | Spine | Tail => self.body.color(),
			HeadMesh | HeadRig | Nose | EarLeft | EarRight => self.skin_color().color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.mouth.color(),
			Hair => self.hair.color(),
			_ => self.body.color(),
		}
	}

	pub fn skin_color(&self) -> ItemColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.head = skin;
		self.nose = skin;
		self.ears = skin;
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

/// Minimal unresolved Braidman state used by commands and, later, UI fields.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BraidmanConfig {
	// Preset IDs are shared enums today; species-owned tables will live in `presets`.
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: BodyMesh,
	pub head: HeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub ear: EarMesh,
	pub hair: HairMesh,
	/// Multiple clothing layers compose; repeat `--clothing` on the CLI.
	pub clothing: Vec<ClothingMesh>,
	pub colors: BraidmanColors,
	pub sliders: BraidmanSliders,
}

impl Default for BraidmanConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl BraidmanConfig {
	/// Default command-driven preview, before any future UI field model exists.
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: BodyMesh::Standard,
			head: HeadMesh::Standard,
			eye: EyeMesh::Standard,
			nose: NoseMesh::Standard,
			mouth: MouthMesh::Standard,
			ear: EarMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: BraidmanColors::default(),
			sliders: BraidmanSliders::default(),
		}
	}

	pub fn with_gender(mut self, gender: GenderPreset) -> Self {
		self.gender = gender;
		self
	}

	pub fn with_build(mut self, build: BuildPreset) -> Self {
		self.build = build;
		self
	}

	pub fn with_sliders(mut self, sliders: BraidmanSliders) -> Self {
		self.sliders = sliders;
		self
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
			"braidman gender={} build={} body={} head={} eye={} nose={} mouth={} ear={} hair={} clothing={} colors=body:{} head:{} eyes:{} nose:{} mouth:{} ears:{} hair:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.head.label(),
			self.eye.label(),
			self.nose.label(),
			self.mouth.label(),
			self.ear.label(),
			self.hair.label(),
			clothing,
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.nose.label(),
			self.colors.mouth.label(),
			self.colors.ears.label(),
			self.colors.hair.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for BraidmanConfig {
	type Components = Braidman;

	fn components(&self) -> Self::Components {
		Braidman::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		crate::clothing_layers(
			self.clothing.iter().copied(),
			self.body.clothing_host(),
			|mesh| self.colors.clothing_material_for(mesh),
			|mesh| self.colors.clothing_color(mesh),
		)
	}
}
