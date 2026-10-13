//! Kaller asset catalog.

use clap::ValueEnum;

use crate::{
	assets::AssetPath,
	species::common::{HEAD_STANDARD, HORNS_HARROWED_CROWN, MOUTH_ROBREK_SNOUT},
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerHeadMesh {
	#[default]
	Meerkat,
}

impl KallerHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD
	}
}

/// Fixed robrek snout — always attached; kept as an enum for menu identity traits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerSnoutMesh {
	#[default]
	Robrek,
}

impl KallerSnoutMesh {
	pub const VALUES: &'static [Self] = &[Self::Robrek];

	pub const fn label(self) -> &'static str {
		"robrek"
	}

	pub const fn path(self) -> AssetPath {
		MOUTH_ROBREK_SNOUT
	}
}

/// Fixed harrowed crown — always attached; kept as an enum for menu identity traits.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KallerHornMesh {
	#[default]
	HarrowedCrown,
}

impl KallerHornMesh {
	pub const VALUES: &'static [Self] = &[Self::HarrowedCrown];

	pub const fn label(self) -> &'static str {
		"harrowed-crown"
	}

	pub const fn path(self) -> AssetPath {
		HORNS_HARROWED_CROWN
	}
}
