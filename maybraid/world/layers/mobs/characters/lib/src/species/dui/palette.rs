//! Dui color palettes.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiSkinColor {
	#[default]
	Purple,
	DesertBrown,
	Blue,
	Gold,
}

impl DuiSkinColor {
	pub const VALUES: &'static [Self] = &[Self::Purple, Self::DesertBrown, Self::Blue, Self::Gold];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Purple => "purple",
			Self::DesertBrown => "desert-brown",
			Self::Blue => "blue",
			Self::Gold => "gold",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Purple => "#8A787A",
			Self::DesertBrown => "#9E8970",
			Self::Blue => "#7F8A85",
			Self::Gold => "#A89470",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Purple => bevy::prelude::Color::srgb(0.54, 0.47, 0.48),
			Self::DesertBrown => bevy::prelude::Color::srgb(0.62, 0.54, 0.44),
			Self::Blue => bevy::prelude::Color::srgb(0.50, 0.54, 0.52),
			Self::Gold => bevy::prelude::Color::srgb(0.66, 0.58, 0.44),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiEyeColor {
	#[default]
	Black,
}

impl DuiEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Black];

	pub const fn label(self) -> &'static str {
		"black"
	}

	pub const fn color_hex(self) -> &'static str {
		"#141419"
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Black => bevy::prelude::Color::srgb(0.08, 0.08, 0.10),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiMouthColor {
	#[default]
	Red,
	Blue,
}

impl DuiMouthColor {
	pub const VALUES: &'static [Self] = &[Self::Red, Self::Blue];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Red => "red",
			Self::Blue => "blue",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Red => "#8C5C52",
			Self::Blue => "#667080",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Red => bevy::prelude::Color::srgb(0.55, 0.36, 0.32),
			Self::Blue => bevy::prelude::Color::srgb(0.40, 0.44, 0.50),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum DuiNoseColor {
	#[default]
	Black,
}

impl DuiNoseColor {
	pub const VALUES: &'static [Self] = &[Self::Black];

	pub const fn label(self) -> &'static str {
		"black"
	}

	pub const fn color_hex(self) -> &'static str {
		"#141419"
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Black => bevy::prelude::Color::srgb(0.08, 0.08, 0.10),
		}
	}
}
