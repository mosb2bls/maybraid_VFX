//! Shared item color palette.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// General-purpose color palette for items (and species that adopt it, such
/// as Braidman skin/hair and every species' hair color).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ItemColor {
	#[default]
	Natural,
	Warm,
	Cool,
	Dark,
	Light,
	Red,
	Blue,
	Green,
	Gold,
}

impl ItemColor {
	pub const VALUES: &'static [Self] = &[
		Self::Natural,
		Self::Warm,
		Self::Cool,
		Self::Dark,
		Self::Light,
		Self::Red,
		Self::Blue,
		Self::Green,
		Self::Gold,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Natural => "natural",
			Self::Warm => "warm",
			Self::Cool => "cool",
			Self::Dark => "dark",
			Self::Light => "light",
			Self::Red => "red",
			Self::Blue => "blue",
			Self::Green => "green",
			Self::Gold => "gold",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Natural => "#B88A6B",
			Self::Warm => "#DB9441",
			Self::Cool => "#7599B8",
			Self::Dark => "#2E2926",
			Self::Light => "#E0CCAE",
			Self::Red => "#B82E29",
			Self::Blue => "#2E4DC2",
			Self::Green => "#388547",
			Self::Gold => "#E0AD38",
		}
	}

	pub const fn rgb(self) -> [f32; 3] {
		match self {
			Self::Natural => [0.72, 0.54, 0.42],
			Self::Warm => [0.86, 0.58, 0.38],
			Self::Cool => [0.46, 0.60, 0.72],
			Self::Dark => [0.18, 0.16, 0.15],
			Self::Light => [0.88, 0.80, 0.68],
			Self::Red => [0.72, 0.18, 0.16],
			Self::Blue => [0.18, 0.30, 0.76],
			Self::Green => [0.22, 0.52, 0.28],
			Self::Gold => [0.88, 0.68, 0.22],
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		let [r, g, b] = self.rgb();
		bevy::prelude::Color::srgb(r, g, b)
	}
}
