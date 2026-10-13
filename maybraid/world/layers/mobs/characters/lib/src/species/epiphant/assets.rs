//! Epiphant asset catalog.

use clap::ValueEnum;

use crate::{assets::AssetPath, species::common::assets::HEAD_STANDARD_PRONOGRADE};

pub use crate::species::common::EyeMesh;

const BODY_EPIPHANT: AssetPath = AssetPath::new("characters/bodies/quadruped/epiphant.glb");
const EAR_EPIPHANT: AssetPath = AssetPath::new("characters/ears/epiphant_ear_left.glb");
const NOSE_TRUNKISH: AssetPath = AssetPath::new("characters/noses/trunkish_nose.glb");

/// Enlarge the pronograde head stack so the meerkat head reads as "large" on the body.
pub(crate) const HEAD_RIG_SOCKET_SCALE: f32 = 1.45;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantBodyMesh {
	#[default]
	Epiphant,
}

impl EpiphantBodyMesh {
	pub const VALUES: &'static [Self] = &[Self::Epiphant];

	pub const fn label(self) -> &'static str {
		"epiphant"
	}

	pub const fn path(self) -> AssetPath {
		BODY_EPIPHANT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantHeadMesh {
	#[default]
	Meerkat,
}

impl EpiphantHeadMesh {
	pub const VALUES: &'static [Self] = &[Self::Meerkat];

	pub const fn label(self) -> &'static str {
		"meerkat"
	}

	pub const fn path(self) -> AssetPath {
		HEAD_STANDARD_PRONOGRADE
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantEarMesh {
	#[default]
	Epiphant,
}

impl EpiphantEarMesh {
	pub const VALUES: &'static [Self] = &[Self::Epiphant];

	pub const fn label(self) -> &'static str {
		"epiphant"
	}

	pub const fn path(self) -> AssetPath {
		EAR_EPIPHANT
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantNoseMesh {
	#[default]
	Trunkish,
}

impl EpiphantNoseMesh {
	pub const VALUES: &'static [Self] = &[Self::Trunkish];

	pub const fn label(self) -> &'static str {
		"trunkish"
	}

	pub const fn path(self) -> AssetPath {
		NOSE_TRUNKISH
	}
}
