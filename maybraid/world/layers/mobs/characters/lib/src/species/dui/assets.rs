//! Dui asset catalog.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::{assets::AssetPath, species::common::assets::MOUTH_STANDARD};

const HEAD_BARRED_BOWL: AssetPath = AssetPath::new("characters/heads/barred_bowl_head.glb");
const EYE_THORN: AssetPath = AssetPath::new("characters/horns/single_thorn_left.glb");
const NOSE_TBAR: AssetPath = AssetPath::new("characters/noses/tbar_nose.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiHeadMesh {
	#[default]
	BarredBowl,
}

impl DuiHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::BarredBowl];

	pub const fn label(self) -> &'static str {
		"barred-bowl"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_BARRED_BOWL
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiEyeMesh {
	#[default]
	Thorn,
}

impl DuiEyeMesh {
	pub const VALUES: &'static [Self] = &[Self::Thorn];

	pub const fn label(self) -> &'static str {
		"thorn"
	}

	pub const fn path(self) -> AssetPath {
		EYE_THORN
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiNoseMesh {
	#[default]
	None,
	Tbar,
}

impl DuiNoseMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::Tbar];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::Tbar => "tbar",
		}
	}

	pub const fn path(self) -> AssetPath {
		match self {
			Self::None => NOSE_TBAR,
			Self::Tbar => NOSE_TBAR,
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiMouthMesh {
	#[default]
	SmallCommon,
}

impl DuiMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::SmallCommon];

	pub const fn label(self) -> &'static str {
		"small-common"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_STANDARD
	}
}
