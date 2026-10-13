//! Sonyak species definition.
//!
//! Gumbus quadruped with the Yilter/Dui barred-bowl head (orthograde), cow snout,
//! thorn eyes, and thick braids as a short mane. No intermediate neck armature.

pub mod assets;
pub mod recipe;
pub use recipe::Sonyak;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use character_items::ItemColor;
use sliders::SonyakSliders;

pub use assets::{SonyakBodyMesh, SonyakHeadMesh, SonyakMouthMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SonyakColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub mouth: ItemColor,
	pub hair: ItemColor,
	pub tail: ItemColor,
}

impl Default for SonyakColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: ItemColor::Cool,
			eyes: ItemColor::Dark,
			mouth: ItemColor::Warm,
			hair: ItemColor::Dark,
			tail: body,
		}
	}
}

impl SonyakColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh => self.body.color(),
			HeadMesh | HeadRig => self.head.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Hair => self.hair.color(),
			Mouth => self.mouth.color(),
			Tail => self.tail.color(),
			_ => self.body.color(),
		}
	}

	pub fn skin_color(&self) -> ItemColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.tail = skin;
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct SonyakConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: SonyakMouthMesh,
	pub colors: SonyakColors,
	pub sliders: SonyakSliders,
}

impl Default for SonyakConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl SonyakConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			mouth: SonyakMouthMesh::Cow,
			colors: SonyakColors::default(),
			sliders: SonyakSliders::default(),
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

	pub fn with_sliders(mut self, sliders: SonyakSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"sonyak gender={} build={} mouth={} colors=body:{} head:{} eyes:{} mouth:{} hair:{} tail:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.mouth.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.mouth.label(),
			self.colors.hair.label(),
			self.colors.tail.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for SonyakConfig {
	type Components = Sonyak;

	fn components(&self) -> Self::Components {
		Sonyak::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
