//! Hars species definition.
//!
//! Horse-like quadruped: Rumbler body on the quadruped rig, Cowder head on a
//! pronograde head rig. The neck bone is pitched ~45° with a matching
//! head-socket counterpose; neck and limbs are lengthened. Flank ears, cat
//! tail, and cow snout.

pub mod assets;
pub mod recipe;
pub use recipe::Hars;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::EyeMesh;
use character_items::ItemColor;
use sliders::HarsSliders;

pub use assets::{HarsBodyMesh, HarsHeadMesh, HarsMouthMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HarsColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub ears: ItemColor,
	pub mouth: ItemColor,
	pub tail: ItemColor,
}

impl Default for HarsColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: body,
			eyes: ItemColor::Dark,
			ears: body,
			mouth: ItemColor::Warm,
			tail: body,
		}
	}
}

impl HarsColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh => self.body.color(),
			HeadMesh | HeadRig | EarLeft | EarRight => self.skin_color().color(),
			EyeLeft | EyeRight => self.eyes.color(),
			Mouth => self.mouth.color(),
			Tail => self.tail.color(),
			NeckMesh | NeckRig => self.body.color(),
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
pub struct HarsConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: HarsMouthMesh,
	pub eye: EyeMesh,
	pub colors: HarsColors,
	pub sliders: HarsSliders,
}

impl Default for HarsConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl HarsConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			mouth: HarsMouthMesh::Cow,
			eye: EyeMesh::Standard,
			colors: HarsColors::default(),
			sliders: HarsSliders::default(),
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

	pub fn with_sliders(mut self, sliders: HarsSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"hars gender={} build={} mouth={} eye={} colors=body:{} head:{} eyes:{} ears:{} mouth:{} tail:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.mouth.label(),
			self.eye.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.ears.label(),
			self.colors.mouth.label(),
			self.colors.tail.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for HarsConfig {
	type Components = Hars;

	fn components(&self) -> Self::Components {
		Hars::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
