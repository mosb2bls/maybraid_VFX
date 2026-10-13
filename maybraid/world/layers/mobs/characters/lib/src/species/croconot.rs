//! Croconot species definition.
//!
//! Low-slung quadruped: Dragloon body on the quadruped rig, pronograde canine head,
//! flank ears, lerodon tail and snout, and optional harrowed crown horns.

pub mod assets;
pub mod recipe;
pub use recipe::Croconot;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::EyeMesh;
use character_items::ItemColor;
use sliders::CroconotSliders;

pub use assets::{CroconotBodyMesh, CroconotHeadMesh, CroconotHornMesh, CroconotMouthMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CroconotColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub ears: ItemColor,
	pub mouth: ItemColor,
	pub tail: ItemColor,
	pub horns: ItemColor,
}

impl Default for CroconotColors {
	fn default() -> Self {
		let body = ItemColor::Green;
		Self {
			body,
			head: body,
			eyes: ItemColor::Gold,
			ears: body,
			mouth: ItemColor::Cool,
			tail: body,
			horns: ItemColor::Warm,
		}
	}
}

impl CroconotColors {
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
pub struct CroconotConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: CroconotHornMesh,
	pub eye: EyeMesh,
	pub colors: CroconotColors,
	pub sliders: CroconotSliders,
}

impl Default for CroconotConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl CroconotConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			horns: CroconotHornMesh::None,
			eye: EyeMesh::Standard,
			colors: CroconotColors::default(),
			sliders: CroconotSliders::default(),
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

	pub fn with_sliders(mut self, sliders: CroconotSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"croconot gender={} build={} horns={} eye={} colors=body:{} head:{} eyes:{} ears:{} mouth:{} tail:{} horns_color:{} sliders={}",
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

impl CharacterRecipe for CroconotConfig {
	type Components = Croconot;

	fn components(&self) -> Self::Components {
		Croconot::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
