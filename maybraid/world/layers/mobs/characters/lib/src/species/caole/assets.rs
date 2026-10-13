//! Caole asset catalog.

use clap::ValueEnum;

use crate::species::common::{BODY_GUMBUS, BODY_RUMBLER, MOUTH_COW_SNOUT};

pub use crate::species::common::EyeMesh;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CaoleBodyMesh {
	#[default]
	Gumbus,
	Rumbler,
}

impl CaoleBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus, Self::Rumbler];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Gumbus => "gumbus",
			Self::Rumbler => "rumbler",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		match self {
			Self::Gumbus => BODY_GUMBUS,
			Self::Rumbler => BODY_RUMBLER,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CaoleMouthMesh {
	#[default]
	Cow,
}

impl CaoleMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
