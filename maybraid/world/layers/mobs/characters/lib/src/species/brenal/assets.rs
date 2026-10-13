//! Brenal asset catalog.

use clap::ValueEnum;

use crate::species::common::{BODY_GUMBUS, HEAD_CANINE, HORNS_HARROWED_CROWN, MOUTH_CANINE_SNOUT};

pub use crate::species::common::EyeMesh;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrenalBodyMesh {
	#[default]
	Gumbus,
}

impl BrenalBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrenalHeadMesh {
	#[default]
	Canine,
}

impl BrenalHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Canine];

	pub const fn label(self) -> &'static str {
		"canine-head"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_CANINE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrenalMouthMesh {
	#[default]
	CanineSnout,
}

impl BrenalMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::CanineSnout];

	pub const fn label(self) -> &'static str {
		"canine-snout"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_CANINE_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum BrenalHornMesh {
	#[default]
	None,
	HarrowedCrown,
}

impl BrenalHornMesh {
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
