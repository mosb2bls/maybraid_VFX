//! Epiphant species definition.
//!
//! Elephant-like quadruped: Epiphant body on the quadruped rig, pronograde
//! meerkat head, Epiphant ears, trunkish nose, and cat tail.

pub mod assets;
pub mod recipe;
pub use recipe::Epiphant;
pub mod palette;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::EyeMesh;
use sliders::EpiphantSliders;

pub use assets::{EpiphantBodyMesh, EpiphantEarMesh, EpiphantHeadMesh, EpiphantNoseMesh};
pub use palette::EpiphantColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpiphantColors {
	pub body: EpiphantColor,
	pub head: EpiphantColor,
	pub eyes: EpiphantColor,
	pub ears: EpiphantColor,
	pub nose: EpiphantColor,
	pub tail: EpiphantColor,
}

impl Default for EpiphantColors {
	fn default() -> Self {
		let body = EpiphantColor::Slate;
		Self {
			body,
			head: body,
			eyes: EpiphantColor::Blue,
			ears: body,
			nose: EpiphantColor::SoftEarthRed,
			tail: body,
		}
	}
}

impl EpiphantColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh => self.body.color(),
			HeadMesh | HeadRig => self.head.color(),
			EyeLeft | EyeRight => self.eyes.color(),
			EarLeft | EarRight => self.ears.color(),
			Nose => self.nose.color(),
			Tail => self.tail.color(),
			_ => self.body.color(),
		}
	}

	pub fn skin_color(&self) -> EpiphantColor {
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
pub struct EpiphantConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub body: EpiphantBodyMesh,
	pub head: EpiphantHeadMesh,
	pub ear: EpiphantEarMesh,
	pub nose: EpiphantNoseMesh,
	pub eye: EyeMesh,
	pub colors: EpiphantColors,
	pub sliders: EpiphantSliders,
}

impl Default for EpiphantConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl EpiphantConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			body: EpiphantBodyMesh::Epiphant,
			head: EpiphantHeadMesh::Meerkat,
			ear: EpiphantEarMesh::Epiphant,
			nose: EpiphantNoseMesh::Trunkish,
			eye: EyeMesh::Standard,
			colors: EpiphantColors::default(),
			sliders: EpiphantSliders::default(),
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

	pub fn with_sliders(mut self, sliders: EpiphantSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"epiphant gender={} build={} body={} head={} ear={} nose={} eye={} colors=body:{} head:{} eyes:{} ears:{} nose:{} tail:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.body.label(),
			self.head.label(),
			self.ear.label(),
			self.nose.label(),
			self.eye.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.nose.label(),
			self.colors.tail.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for EpiphantConfig {
	type Components = Epiphant;

	fn components(&self) -> Self::Components {
		Epiphant::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
