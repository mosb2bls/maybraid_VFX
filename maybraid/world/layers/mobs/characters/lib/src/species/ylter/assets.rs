//! Yilter asset catalog: Hars neck stack + Dui barred-bowl head + cow snout.

use clap::ValueEnum;

use crate::{
	assets::AssetPath,
	species::common::{BODY_RUMBLER, MOUTH_COW_SNOUT},
};

const HEAD_BARRED_BOWL: AssetPath = AssetPath::new("characters/heads/barred_bowl_head.glb");
pub(crate) const EYE_THORN: AssetPath = AssetPath::new("characters/horns/single_thorn_left.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum YilterBodyMesh {
	#[default]
	Rumbler,
}

impl YilterBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Rumbler];

	pub const fn label(self) -> &'static str {
		"rumbler"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_RUMBLER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum YilterHeadMesh {
	#[default]
	BarredBowl,
}

impl YilterHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::BarredBowl];

	pub const fn label(self) -> &'static str {
		"barred-bowl"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_BARRED_BOWL
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum YilterMouthMesh {
	#[default]
	Cow,
}

impl YilterMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
