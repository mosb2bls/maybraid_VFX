//! Hars asset catalog.

use clap::ValueEnum;

use crate::species::common::{BODY_RUMBLER, HEAD_COWDER, MOUTH_COW_SNOUT};

pub use crate::species::common::EyeMesh;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HarsBodyMesh {
	#[default]
	Rumbler,
}

impl HarsBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Rumbler];

	pub const fn label(self) -> &'static str {
		"rumbler"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_RUMBLER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HarsHeadMesh {
	#[default]
	Cowder,
}

impl HarsHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Cowder];

	pub const fn label(self) -> &'static str {
		"cowder"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_COWDER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum HarsMouthMesh {
	#[default]
	Cow,
}

impl HarsMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
