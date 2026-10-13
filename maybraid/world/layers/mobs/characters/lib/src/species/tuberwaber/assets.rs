//! Tuberwaber asset catalog: tuberwaber body + head on the humanoid biped stack.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::assets::AssetPath;

pub use crate::species::common::{EyeMesh, HairMesh, MouthMesh, NoseMesh};

const BODY_TUBERWABER: AssetPath = AssetPath::new("characters/bodies/biped/tuberwaber_body.glb");
const HEAD_TUBERWABER: AssetPath = AssetPath::new("characters/heads/tuberwaber_head.glb");

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum TuberwaberBodyMesh {
	#[default]
	Tuberwaber,
}

impl TuberwaberBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Tuberwaber];

	pub const fn label(self) -> &'static str {
		"tuberwaber"
	}

	pub const fn path(self) -> AssetPath {
		BODY_TUBERWABER
	}

	pub const fn clothing_host(self) -> character_items::ClothingHost {
		character_items::ClothingHost::TUBERWABER
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum TuberwaberHeadMesh {
	#[default]
	Tuberwaber,
}

impl TuberwaberHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Tuberwaber];

	pub const fn label(self) -> &'static str {
		"tuberwaber"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_TUBERWABER
	}
}
