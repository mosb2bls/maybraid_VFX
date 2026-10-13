//! Mygr color palettes.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum MygrSkinColor {
	#[default]
	Ginger,
	Charcoal,
	Silver,
	Cream,
	Tawny,
}

impl MygrSkinColor {
	pub const VALUES: &'static [Self] =
		&[Self::Ginger, Self::Charcoal, Self::Silver, Self::Cream, Self::Tawny];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ginger => "ginger",
			Self::Charcoal => "charcoal",
			Self::Silver => "silver",
			Self::Cream => "cream",
			Self::Tawny => "tawny",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Ginger => "#C47A3A",
			Self::Charcoal => "#282624",
			Self::Silver => "#8A8F94",
			Self::Cream => "#E8DCC8",
			Self::Tawny => "#8B5E3C",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ginger => bevy::prelude::Color::srgb(0.77, 0.48, 0.23),
			Self::Charcoal => bevy::prelude::Color::srgb(0.16, 0.15, 0.14),
			Self::Silver => bevy::prelude::Color::srgb(0.54, 0.56, 0.58),
			Self::Cream => bevy::prelude::Color::srgb(0.91, 0.86, 0.78),
			Self::Tawny => bevy::prelude::Color::srgb(0.55, 0.37, 0.24),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum MygrEyeColor {
	#[default]
	Green,
	Amber,
	Blue,
}

impl MygrEyeColor {
	pub const VALUES: &'static [Self] = &[Self::Green, Self::Amber, Self::Blue];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Green => "green",
			Self::Amber => "amber",
			Self::Blue => "blue",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Green => "#4A8C4F",
			Self::Amber => "#C9A227",
			Self::Blue => "#6BA3D1",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Green => bevy::prelude::Color::srgb(0.29, 0.55, 0.31),
			Self::Amber => bevy::prelude::Color::srgb(0.79, 0.64, 0.15),
			Self::Blue => bevy::prelude::Color::srgb(0.42, 0.64, 0.82),
		}
	}
}
