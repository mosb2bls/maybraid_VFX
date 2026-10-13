//! Chupri asset catalog.

use clap::ValueEnum;

use crate::{assets::AssetPath, species::common::assets::HEAD_STANDARD};

const BEAK: AssetPath = AssetPath::new("characters/snouts/beak.glb");
const HOOK_BEAK: AssetPath = AssetPath::new("characters/snouts/hook_beak.glb");
const SHARP_BEAK: AssetPath = AssetPath::new("characters/snouts/sharp_beak.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ChupriHeadMesh {
	#[default]
	Meerkat,
}

impl ChupriHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ChupriBeakMesh {
	#[default]
	Beak,
	Hook,
	Sharp,
}

impl ChupriBeakMesh {
	pub const VALUES: &'static [Self] = &[Self::Beak, Self::Hook, Self::Sharp];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Beak => "beak",
			Self::Hook => "hook",
			Self::Sharp => "sharp",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::Beak => BEAK,
			Self::Hook => HOOK_BEAK,
			Self::Sharp => SHARP_BEAK,
		}
	}
}
