//! Lero color palettes.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum LeroSkinColor {
	#[default]
	FadedGreen,
	MossDrift,
	DustySage,
	FadedRed,
	WeatheredRose,
	ClayRust,
}

impl LeroSkinColor {
	pub const VALUES: &'static [Self] = &[
		Self::FadedGreen,
		Self::MossDrift,
		Self::DustySage,
		Self::FadedRed,
		Self::WeatheredRose,
		Self::ClayRust,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::FadedGreen => "faded-green",
			Self::MossDrift => "moss-drift",
			Self::DustySage => "dusty-sage",
			Self::FadedRed => "faded-red",
			Self::WeatheredRose => "weathered-rose",
			Self::ClayRust => "clay-rust",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::FadedGreen => "#849475",
			Self::MossDrift => "#73856B",
			Self::DustySage => "#949E85",
			Self::FadedRed => "#946B66",
			Self::WeatheredRose => "#9E7A75",
			Self::ClayRust => "#856157",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::FadedGreen => bevy::prelude::Color::srgb(0.52, 0.58, 0.48),
			Self::MossDrift => bevy::prelude::Color::srgb(0.45, 0.52, 0.42),
			Self::DustySage => bevy::prelude::Color::srgb(0.58, 0.62, 0.52),
			Self::FadedRed => bevy::prelude::Color::srgb(0.58, 0.42, 0.40),
			Self::WeatheredRose => bevy::prelude::Color::srgb(0.62, 0.48, 0.46),
			Self::ClayRust => bevy::prelude::Color::srgb(0.52, 0.38, 0.34),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum LeroEyeColor {
	#[default]
	Gold,
	Amber,
	PaleYellow,
}

impl LeroEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Gold, Self::Amber, Self::PaleYellow];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Gold => "gold",
			Self::Amber => "amber",
			Self::PaleYellow => "pale-yellow",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Gold => "#D1B861",
			Self::Amber => "#C79E47",
			Self::PaleYellow => "#E0D18C",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Gold => bevy::prelude::Color::srgb(0.82, 0.72, 0.38),
			Self::Amber => bevy::prelude::Color::srgb(0.78, 0.62, 0.28),
			Self::PaleYellow => bevy::prelude::Color::srgb(0.88, 0.82, 0.55),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum LeroMouthColor {
	#[default]
	SoftBlush,
	PaleRose,
	Buttercream,
	PaleGold,
}

impl LeroMouthColor {
	pub const VALUES: &'static [Self] =
		&[Self::SoftBlush, Self::PaleRose, Self::Buttercream, Self::PaleGold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftBlush => "soft-blush",
			Self::PaleRose => "pale-rose",
			Self::Buttercream => "buttercream",
			Self::PaleGold => "pale-gold",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::SoftBlush => "#E5C7BC",
			Self::PaleRose => "#E0B8AD",
			Self::Buttercream => "#EBDBB3",
			Self::PaleGold => "#E5D19E",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftBlush => bevy::prelude::Color::srgb(0.90, 0.78, 0.74),
			Self::PaleRose => bevy::prelude::Color::srgb(0.88, 0.72, 0.68),
			Self::Buttercream => bevy::prelude::Color::srgb(0.92, 0.86, 0.70),
			Self::PaleGold => bevy::prelude::Color::srgb(0.90, 0.82, 0.62),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum LeroTailColor {
	#[default]
	Pearl,
	PaleIvory,
	Sand,
	PaleMint,
}

impl LeroTailColor {
	pub const VALUES: &'static [Self] = &[Self::Pearl, Self::PaleIvory, Self::Sand, Self::PaleMint];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pearl => "pearl",
			Self::PaleIvory => "pale-ivory",
			Self::Sand => "sand",
			Self::PaleMint => "pale-mint",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Pearl => "#E5E0D1",
			Self::PaleIvory => "#E0D6C2",
			Self::Sand => "#D1BD99",
			Self::PaleMint => "#C7E0D1",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Pearl => bevy::prelude::Color::srgb(0.90, 0.88, 0.82),
			Self::PaleIvory => bevy::prelude::Color::srgb(0.88, 0.84, 0.76),
			Self::Sand => bevy::prelude::Color::srgb(0.82, 0.74, 0.60),
			Self::PaleMint => bevy::prelude::Color::srgb(0.78, 0.88, 0.82),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum LeroSpineColor {
	#[default]
	Pearl,
	PaleIvory,
	Sand,
	PaleMint,
}

impl LeroSpineColor {
	pub const VALUES: &'static [Self] = &[Self::Pearl, Self::PaleIvory, Self::Sand, Self::PaleMint];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Pearl => "pearl",
			Self::PaleIvory => "pale-ivory",
			Self::Sand => "sand",
			Self::PaleMint => "pale-mint",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Pearl => "#E5E0D1",
			Self::PaleIvory => "#E0D6C2",
			Self::Sand => "#D1BD99",
			Self::PaleMint => "#C7E0D1",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Pearl => bevy::prelude::Color::srgb(0.90, 0.88, 0.82),
			Self::PaleIvory => bevy::prelude::Color::srgb(0.88, 0.84, 0.76),
			Self::Sand => bevy::prelude::Color::srgb(0.82, 0.74, 0.60),
			Self::PaleMint => bevy::prelude::Color::srgb(0.78, 0.88, 0.82),
		}
	}
}
