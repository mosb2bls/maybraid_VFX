//! Brokker asset catalog.

use clap::ValueEnum;

use crate::assets::AssetPath;

const HEAD_ORTHO_TEE: AssetPath = AssetPath::new("characters/heads/ortho_tee_head.glb");
const SNOUT_IGNY: AssetPath = AssetPath::new("characters/snouts/igny_snout.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrokkerHeadMesh {
	#[default]
	OrthoTee,
}

impl BrokkerHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::OrthoTee];

	pub const fn label(self) -> &'static str {
		"ortho-tee"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_ORTHO_TEE
	}
}

/// Fixed igny snout — always attached; kept as an enum for menu identity traits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrokkerSnoutMesh {
	#[default]
	Igny,
}

impl BrokkerSnoutMesh {
	pub const VALUES: &'static [Self] = &[Self::Igny];

	pub const fn label(self) -> &'static str {
		"igny"
	}

	pub const fn path(self) -> AssetPath {
		SNOUT_IGNY
	}
}
