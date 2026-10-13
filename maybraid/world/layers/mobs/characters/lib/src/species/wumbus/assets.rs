//! Wumbus asset catalog.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{
	assets::AssetPath,
	species::common::assets::{HEAD_ORTHO_BEAR, HORNS_HARROWED_CROWN, MOUTH_CANINE_SNOUT},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusHeadMesh {
	#[default]
	OrthoBear,
}

impl WumbusHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::OrthoBear];

	pub const fn label(self) -> &'static str {
		"ortho-bear"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_ORTHO_BEAR
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusMouthMesh {
	#[default]
	CanineSnout,
}

impl WumbusMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::CanineSnout];

	pub const fn label(self) -> &'static str {
		"canine-snout"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_CANINE_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum WumbusHornMesh {
	#[default]
	None,
	HarrowedCrown,
}

impl WumbusHornMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::HarrowedCrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::HarrowedCrown => "harrowed-crown",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::None => HORNS_HARROWED_CROWN,
			Self::HarrowedCrown => HORNS_HARROWED_CROWN,
		}
	}
}
