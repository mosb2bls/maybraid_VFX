//! Croconot asset catalog.

use clap::ValueEnum;

use crate::species::common::{
	BODY_DRAGLOON, HEAD_CANINE, HORNS_HARROWED_CROWN, MOUTH_LERODON_SNOUT,
};

pub use crate::species::common::EyeMesh;

/// Lerodon snout scale on the pronograde mouth socket (from Lero, enlarged for Croconot).
pub(crate) const SNOUT_XY_SCALE: f32 = 2.25;
pub(crate) const SNOUT_Z_SCALE: f32 = 6.2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotBodyMesh {
	#[default]
	Dragloon,
}

impl CroconotBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Dragloon];

	pub const fn label(self) -> &'static str {
		"dragloon"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_DRAGLOON
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotHeadMesh {
	#[default]
	Canine,
}

impl CroconotHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Canine];

	pub const fn label(self) -> &'static str {
		"canine-head"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_CANINE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotMouthMesh {
	#[default]
	Lerodon,
}

impl CroconotMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Lerodon];

	pub const fn label(self) -> &'static str {
		"lerodon"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_LERODON_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum CroconotHornMesh {
	#[default]
	None,
	HarrowedCrown,
}

impl CroconotHornMesh {
	pub const VALUES: &'static [Self] = &[Self::None, Self::HarrowedCrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::None => "none",
			Self::HarrowedCrown => "harrowed-crown",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		match self {
			Self::None => HORNS_HARROWED_CROWN,
			Self::HarrowedCrown => HORNS_HARROWED_CROWN,
		}
	}
}
