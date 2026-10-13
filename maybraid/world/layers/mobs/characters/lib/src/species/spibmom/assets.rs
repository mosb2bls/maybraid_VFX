//! Spibmom asset catalog.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{assets::AssetPath, species::common::assets::HEAD_STANDARD};

const HORNS_FINBONE_CROWN: AssetPath = AssetPath::new("characters/horns/finbone_crown.glb");
const NOSE_TRUNKISH: AssetPath = AssetPath::new("characters/noses/trunkish_nose.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomHeadMesh {
	#[default]
	Meerkat,
}

impl SpibmomHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomMouthMesh {
	#[default]
	Trunkish,
}

impl SpibmomMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Trunkish];

	pub const fn label(self) -> &'static str {
		"trunkish"
	}

	pub const fn path(self) -> AssetPath {
		NOSE_TRUNKISH
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SpibmomCrownMesh {
	#[default]
	Finbone,
}

impl SpibmomCrownMesh {
	pub const VALUES: &'static [Self] = &[Self::Finbone];

	pub const fn label(self) -> &'static str {
		"finbone"
	}

	pub const fn path(self) -> AssetPath {
		HORNS_FINBONE_CROWN
	}
}
