//! Brodler asset catalog.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::species::common::{HORNS_HARROWED_CROWN, HORNS_LORKEN_CROWN};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum HornMesh {
	#[default]
	HarrowedCrown,
	LorkenCrown,
}

impl HornMesh {
	pub const VALUES: &'static [Self] = &[Self::HarrowedCrown, Self::LorkenCrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::HarrowedCrown => "harrowed-crown",
			Self::LorkenCrown => "lorken-crown",
		}
	}

	pub const fn path(self) -> crate::assets::AssetPath {
		match self {
			Self::HarrowedCrown => HORNS_HARROWED_CROWN,
			Self::LorkenCrown => HORNS_LORKEN_CROWN,
		}
	}
}
