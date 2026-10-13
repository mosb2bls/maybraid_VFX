//! Tuberwaber species definition.
//!
//! Biped similar to Braidman: humanoid rig with the tuberwaber body and head on
//! an orthograde head stack, shared features (eyes/nose/mouth/hair/clothing), a
//! fixed harrowed crown, and a colorful cool-toned skin palette (no ears).

pub mod assets;
pub mod recipe;
pub use recipe::Tuberwaber;
pub mod palette;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::{EyeMesh, HairMesh, MouthMesh, NoseMesh};
use character_items::{
	ClothingColor, ClothingMaterial, ClothingMaterialChoice, ClothingMesh, ItemColor,
};
use serde::{Deserialize, Serialize};
use sliders::TuberwaberSliders;

pub use assets::{TuberwaberBodyMesh, TuberwaberHeadMesh};
pub use palette::TuberwaberColor;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TuberwaberColors {
	pub body: TuberwaberColor,
	pub head: TuberwaberColor,
	pub eyes: TuberwaberColor,
	pub nose: TuberwaberColor,
	pub mouth: TuberwaberColor,
	pub horns: TuberwaberColor,
	pub hair: ItemColor,
	pub clothing_default: ItemColor,
	pub clothing_material: ClothingMaterial,
	pub clothing_materials: Vec<ClothingMaterialChoice>,
	pub clothing: Vec<ClothingColor>,
}

impl Default for TuberwaberColors {
	fn default() -> Self {
		let body = TuberwaberColor::MistBlue;
		Self {
			body,
			head: body,
			eyes: TuberwaberColor::Teal,
			nose: body,
			mouth: TuberwaberColor::Coral,
			horns: TuberwaberColor::Slate,
			hair: ItemColor::Dark,
			clothing_default: ItemColor::Cool,
			clothing_material: ClothingMaterial::Cloth,
			clothing_materials: Vec::new(),
			clothing: Vec::new(),
		}
	}
}

impl TuberwaberColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh => self.body.color(),
			HeadMesh | HeadRig | Nose => self.skin_color().color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.mouth.color(),
			Horns => self.horns.color(),
			Hair => self.hair.color(),
			_ => self.body.color(),
		}
	}

	pub fn skin_color(&self) -> TuberwaberColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.head = skin;
		self.nose = skin;
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TuberwaberConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: TuberwaberBodyMesh,
	pub head: TuberwaberHeadMesh,
	pub eye: EyeMesh,
	pub nose: NoseMesh,
	pub mouth: MouthMesh,
	pub hair: HairMesh,
	pub clothing: Vec<ClothingMesh>,
	pub colors: TuberwaberColors,
	pub sliders: TuberwaberSliders,
}

impl Default for TuberwaberConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl TuberwaberConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: TuberwaberBodyMesh::Tuberwaber,
			head: TuberwaberHeadMesh::Tuberwaber,
			eye: EyeMesh::Standard,
			nose: NoseMesh::Loaf,
			mouth: MouthMesh::Standard,
			hair: HairMesh::None,
			clothing: Vec::new(),
			colors: TuberwaberColors::default(),
			sliders: TuberwaberSliders::default(),
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

	pub fn with_sliders(mut self, sliders: TuberwaberSliders) -> Self {
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
			"tuberwaber gender={} build={} body={} head={} eye={} nose={} mouth={} hair={} clothing={} colors=body:{} head:{} eyes:{} nose:{} mouth:{} horns:{} hair:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.head.label(),
			self.eye.label(),
			self.nose.label(),
			self.mouth.label(),
			self.hair.label(),
			clothing,
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.nose.label(),
			self.colors.mouth.label(),
			self.colors.horns.label(),
			self.colors.hair.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for TuberwaberConfig {
	type Components = Tuberwaber;

	fn components(&self) -> Self::Components {
		Tuberwaber::from_config(self)
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
