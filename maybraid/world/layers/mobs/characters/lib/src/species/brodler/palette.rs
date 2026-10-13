//! Brodler color palettes.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BrodlerSkinColor {
	#[default]
	Crimson,
	Umber,
	Ochre,
}

impl BrodlerSkinColor {
	pub const VALUES: &'static [Self] = &[Self::Crimson, Self::Umber, Self::Ochre];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Crimson => "crimson",
			Self::Umber => "umber",
			Self::Ochre => "ochre",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Crimson => "#941E1E",
			Self::Umber => "#4D3329",
			Self::Ochre => "#AD8529",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Crimson => bevy::prelude::Color::srgb(0.58, 0.14, 0.12),
			Self::Umber => bevy::prelude::Color::srgb(0.30, 0.20, 0.16),
			Self::Ochre => bevy::prelude::Color::srgb(0.68, 0.52, 0.26),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BrodlerEyeColor {
	Black,
	#[default]
	LightBlue,
	Yellow,
}

impl BrodlerEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Black, Self::LightBlue, Self::Yellow];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Black => "black",
			Self::LightBlue => "light-blue",
			Self::Yellow => "yellow",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Black => "#141419",
			Self::LightBlue => "#85B3D1",
			Self::Yellow => "#D1B847",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Black => bevy::prelude::Color::srgb(0.08, 0.08, 0.10),
			Self::LightBlue => bevy::prelude::Color::srgb(0.52, 0.70, 0.82),
			Self::Yellow => bevy::prelude::Color::srgb(0.82, 0.72, 0.28),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum BrodlerHornColor {
	#[default]
	LightBrown,
	Yellow,
}

impl BrodlerHornColor {
	pub const VALUES: &'static [Self] = &[Self::LightBrown, Self::Yellow];

	pub const fn label(self) -> &'static str {
		match self {
			Self::LightBrown => "light-brown",
			Self::Yellow => "yellow",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::LightBrown => "#9E7A4D",
			Self::Yellow => "#C7A847",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::LightBrown => bevy::prelude::Color::srgb(0.62, 0.48, 0.30),
			Self::Yellow => bevy::prelude::Color::srgb(0.78, 0.66, 0.28),
		}
	}
}
