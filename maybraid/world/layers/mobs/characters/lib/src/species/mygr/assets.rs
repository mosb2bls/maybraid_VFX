//! Mygr asset catalog.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::species::common::assets::{HEAD_ORTHO_BEAR, MOUTH_CANINE_SNOUT};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum MygrHeadMesh {
	#[default]
	OrthoBear,
}

impl MygrHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::OrthoBear];

	pub const fn label(self) -> &'static str {
		"ortho-bear"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_ORTHO_BEAR
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum MygrMouthMesh {
	#[default]
	CanineSnout,
}

impl MygrMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::CanineSnout];

	pub const fn label(self) -> &'static str {
		"canine-snout"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_CANINE_SNOUT
	}
}
