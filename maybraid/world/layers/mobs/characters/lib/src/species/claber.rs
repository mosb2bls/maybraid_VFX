//! Claber species definition.
//!
//! Large low-slung quadruped: thinned Gumbus body on the quadruped rig (≈2×
//! croconot midback span, short low limbs), Caole head, Robrek snout
//! (shorter/wider), flank ears, lerodon tail, and a prominent harrowed crown.

pub mod assets;
pub mod recipe;
pub use recipe::Claber;
pub mod palette;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use crate::species::common::EyeMesh;
use sliders::ClaberSliders;

pub use assets::{ClaberBodyMesh, ClaberHeadMesh, ClaberHornMesh, ClaberMouthMesh};
pub use palette::ClaberColor;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClaberColors {
	pub body: ClaberColor,
	pub head: ClaberColor,
	pub eyes: ClaberColor,
	pub ears: ClaberColor,
	pub mouth: ClaberColor,
	pub tail: ClaberColor,
	pub horns: ClaberColor,
}

impl Default for ClaberColors {
	fn default() -> Self {
		let body = ClaberColor::SoftPurple;
		Self {
			body,
			head: body,
			eyes: ClaberColor::SoftGold,
			ears: body,
			mouth: ClaberColor::SoftRed,
			tail: body,
			horns: ClaberColor::SoftGold,
		}
	}
}

impl ClaberColors {
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

	pub fn skin_color(&self) -> ClaberColor {
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
pub struct ClaberConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub horns: ClaberHornMesh,
	pub eye: EyeMesh,
	pub colors: ClaberColors,
	pub sliders: ClaberSliders,
}

impl Default for ClaberConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl ClaberConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Average,
			horns: ClaberHornMesh::HarrowedCrown,
			eye: EyeMesh::Standard,
			colors: ClaberColors::default(),
			sliders: ClaberSliders::default(),
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

	pub fn with_sliders(mut self, sliders: ClaberSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"claber gender={} build={} horns={} eye={} colors=body:{} head:{} eyes:{} ears:{} mouth:{} tail:{} horns_color:{} sliders={}",
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

impl CharacterRecipe for ClaberConfig {
	type Components = Claber;

	fn components(&self) -> Self::Components {
		Claber::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
