//! Tipple asset catalog.

use clap::ValueEnum;

use crate::{assets::AssetPath, species::common::assets::HEAD_STANDARD};

const HOOK_BEAK: AssetPath = AssetPath::new("characters/snouts/hook_beak.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TippleHeadMesh {
	#[default]
	Meerkat,
}

impl TippleHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

/// Tipple only uses the hook beak.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum TippleBeakMesh {
	#[default]
	Hook,
}

impl TippleBeakMesh {
	pub const VALUES: &'static [Self] = &[Self::Hook];

	pub const fn label(self) -> &'static str {
		"hook"
	}

	pub const fn path(self) -> AssetPath {
		HOOK_BEAK
	}
}
