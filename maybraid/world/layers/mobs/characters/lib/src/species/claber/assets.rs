//! Claber asset catalog.

use clap::ValueEnum;

use crate::species::common::{BODY_GUMBUS, HEAD_CAOLE, HORNS_HARROWED_CROWN, MOUTH_ROBREK_SNOUT};

pub use crate::species::common::EyeMesh;

/// Robrek snout on the pronograde mouth socket: wider XY, shorter Z than Croconot's Lerodon.
pub(crate) const SNOUT_XY_SCALE: f32 = 2.9;
pub(crate) const SNOUT_Z_SCALE: f32 = 4.4;

/// Enlarged harrowed crown on the pronograde crown socket.
pub(crate) const CROWN_SCALE: f32 = 1.75;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberBodyMesh {
	#[default]
	Gumbus,
}

impl ClaberBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Gumbus];

	pub const fn label(self) -> &'static str {
		"gumbus"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		BODY_GUMBUS
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberHeadMesh {
	#[default]
	Caole,
}

impl ClaberHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Caole];

	pub const fn label(self) -> &'static str {
		"caole-head"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		HEAD_CAOLE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberMouthMesh {
	#[default]
	Robrek,
}

impl ClaberMouthMesh {
	pub const VALUES: &'static [Self] = &[Self::Robrek];

	pub const fn label(self) -> &'static str {
		"robrek"
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		MOUTH_ROBREK_SNOUT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberHornMesh {
	None,
	#[default]
	HarrowedCrown,
}

impl ClaberHornMesh {
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
