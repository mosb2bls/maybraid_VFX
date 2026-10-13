//! Brenal species definition.
//!
//! Quadruped grazer: Gumbus body on the quadruped rig, pronograde canine head,
//! flank ears, cat tail, and optional harrowed crown horns.

pub mod assets;
pub mod recipe;
pub use recipe::Brenal;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::EyeMesh;
use character_items::ItemColor;
use sliders::BrenalSliders;

pub use assets::{BrenalBodyMesh, BrenalHeadMesh, BrenalHornMesh, BrenalMouthMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrenalColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub ears: ItemColor,
	pub mouth: ItemColor,
	pub tail: ItemColor,
	pub horns: ItemColor,
}

impl Default for BrenalColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: body,
			eyes: ItemColor::Blue,
			ears: body,
			mouth: ItemColor::Natural,
			tail: body,
			horns: ItemColor::Warm,
		}
	}
}

impl BrenalColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh => self.body.color(),
			HeadMesh | HeadRig | EarLeft | EarRight => self.skin_color().color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.mouth.color(),
			Horns => self.horns.color(),
			Tail => self.tail.color(),
			_ => self.body.color(),
		}
	}

	pub fn skin_color(&self) -> ItemColor {
		self.body
	}

	pub fn sync_skin_from_body(&mut self) {
		let skin = self.body;
		self.head = skin;
		self.ears = skin;
		self.tail = skin;
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct BrenalConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: BrenalHornMesh,
	pub eye: EyeMesh,
	pub colors: BrenalColors,
	pub sliders: BrenalSliders,
}

impl Default for BrenalConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl BrenalConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			horns: BrenalHornMesh::None,
			eye: EyeMesh::Standard,
			colors: BrenalColors::default(),
			sliders: BrenalSliders::default(),
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

	pub fn with_sliders(mut self, sliders: BrenalSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"brenal gender={} build={} horns={} eye={} colors=body:{} head:{} eyes:{} ears:{} mouth:{} tail:{} horns_color:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.horns.label(),
			self.eye.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.mouth.label(),
			self.colors.tail.label(),
			self.colors.horns.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for BrenalConfig {
	type Components = Brenal;

	fn components(&self) -> Self::Components {
		Brenal::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
