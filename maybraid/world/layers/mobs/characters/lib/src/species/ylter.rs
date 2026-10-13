//! Yilter species definition.
//!
//! Long-necked quadruped: Rumbler body with the Hars-style triple-join neck,
//! Dui barred-bowl head on an orthograde head rig, cow snout, and long legs.

pub mod assets;
pub mod recipe;
pub use recipe::Yilter;
pub mod pose;
pub mod presets;
pub mod sliders;

use crate::{
	presets::{BuildPreset, GenderPreset},
	CharacterRecipe, ClothingLayer,
};

use character_items::ItemColor;
use sliders::YilterSliders;

pub use assets::{YilterBodyMesh, YilterHeadMesh, YilterMouthMesh};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct YilterColors {
	pub body: ItemColor,
	pub head: ItemColor,
	pub eyes: ItemColor,
	pub mouth: ItemColor,
	pub tail: ItemColor,
	pub neck: ItemColor,
}

impl Default for YilterColors {
	fn default() -> Self {
		let body = ItemColor::Natural;
		Self {
			body,
			head: ItemColor::Cool,
			eyes: ItemColor::Dark,
			mouth: ItemColor::Warm,
			tail: body,
			neck: body,
		}
	}
}

impl YilterColors {
	pub fn color_for_slot(&self, slot: crate::CharacterPartSlot) -> bevy::prelude::Color {
		use crate::CharacterPartSlot::*;
		match slot {
			BodyMesh => self.body.color(),
			HeadMesh | HeadRig => self.head.color(),
			NeckMesh | NeckRig => self.neck.color(),
			EyeLeft | EyeRight => self.eyes.color(),
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
		self.neck = skin;
	}
}

#[derive(Debug, Clone, PartialEq)]
pub struct YilterConfig {
	pub gender: GenderPreset,
	pub build: BuildPreset,
	pub mouth: YilterMouthMesh,
	pub colors: YilterColors,
	pub sliders: YilterSliders,
}

impl Default for YilterConfig {
	fn default() -> Self {
		Self::default_preview()
	}
}

impl YilterConfig {
	pub fn default_preview() -> Self {
		Self {
			gender: GenderPreset::Neutral,
			build: BuildPreset::Lanky,
			mouth: YilterMouthMesh::Cow,
			colors: YilterColors::default(),
			sliders: YilterSliders::default(),
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

	pub fn with_sliders(mut self, sliders: YilterSliders) -> Self {
		self.sliders = sliders;
		self
	}

	pub fn status_label(&self) -> String {
		format!(
			"ylter gender={} build={} mouth={} colors=body:{} head:{} eyes:{} mouth:{} neck:{} tail:{} sliders={}",
			self.gender.label(),
			self.build.label(),
			self.mouth.label(),
			self.colors.body.label(),
			self.colors.head.label(),
			self.colors.eyes.label(),
			self.colors.mouth.label(),
			self.colors.neck.label(),
			self.colors.tail.label(),
			self.sliders.status_label(),
		)
	}

	pub fn sync_key(&self) -> String {
		format!("{self:?}")
	}
}

impl CharacterRecipe for YilterConfig {
	type Components = Yilter;

	fn components(&self) -> Self::Components {
		Yilter::from_config(self)
	}

	fn clothing_layers(&self) -> Vec<ClothingLayer> {
		Vec::new()
	}
}
