//! Sonyak asset catalog: Gumbus body + Yilter/Dui head stack + thick-braid mane.

use clap::ValueEnum;

use crate::{
	assets::AssetPath,
	species::common::{BODY_GUMBUS, MOUTH_COW_SNOUT},
};

const HEAD_BARRED_BOWL: AssetPath = AssetPath::new("characters/heads/barred_bowl_head.glb");
pub(crate) const EYE_THORN: AssetPath = AssetPath::new("characters/horns/single_thorn_left.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SonyakBodyMesh {
	#[default]
	Gumbus,
}

impl SonyakBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SonyakHeadMesh {
	#[default]
	BarredBowl,
}

impl SonyakHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::BarredBowl];

	pub const fn label(self) -> &'static str {
		"barred-bowl"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_BARRED_BOWL
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum SonyakMouthMesh {
	#[default]
	Cow,
}

impl SonyakMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Cow];

	pub const fn label(self) -> &'static str {
		"cow"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_COW_SNOUT
	}
}
