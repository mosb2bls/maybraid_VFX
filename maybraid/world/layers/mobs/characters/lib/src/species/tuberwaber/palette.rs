//! Tuberwaber color palettes.
//!
//! Colorful skin swatches with a light gray-blue default — cooler and more
//! chromatic than Braidman's brown-leaning earth tones.

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

/// Shared Tuberwaber swatch set for body and head features.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum TuberwaberColor {
	#[default]
	MistBlue,
	Ice,
	Periwinkle,
	Teal,
	Coral,
	Lilac,
	Mint,
	Saffron,
	Rose,
	Jade,
	Amber,
	Copper,
	Slate,
}

impl TuberwaberColor {
	pub const VALUES: &'static [Self] = &[
		Self::MistBlue,
		Self::Ice,
		Self::Periwinkle,
		Self::Teal,
		Self::Coral,
		Self::Lilac,
		Self::Mint,
		Self::Saffron,
		Self::Rose,
		Self::Jade,
		Self::Amber,
		Self::Copper,
		Self::Slate,
	];

	pub const fn label(self) -> &'static str {
		match self {
			Self::MistBlue => "mist-blue",
			Self::Ice => "ice",
			Self::Periwinkle => "periwinkle",
			Self::Teal => "teal",
			Self::Coral => "coral",
			Self::Lilac => "lilac",
			Self::Mint => "mint",
			Self::Saffron => "saffron",
			Self::Rose => "rose",
			Self::Jade => "jade",
			Self::Amber => "amber",
			Self::Copper => "copper",
			Self::Slate => "slate",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::MistBlue => "#B8C4D0",
			Self::Ice => "#D4DEE8",
			Self::Periwinkle => "#8A9CC8",
			Self::Teal => "#3AA8A0",
			Self::Coral => "#E07868",
			Self::Lilac => "#B090C8",
			Self::Mint => "#78C8A0",
			Self::Saffron => "#E8B040",
			Self::Rose => "#D87898",
			Self::Jade => "#38A070",
			Self::Amber => "#E0A028",
			Self::Copper => "#D07848",
			Self::Slate => "#6A7888",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::MistBlue => bevy::prelude::Color::srgb(0.72, 0.77, 0.82),
			Self::Ice => bevy::prelude::Color::srgb(0.83, 0.87, 0.91),
			Self::Periwinkle => bevy::prelude::Color::srgb(0.54, 0.61, 0.78),
			Self::Teal => bevy::prelude::Color::srgb(0.23, 0.66, 0.63),
			Self::Coral => bevy::prelude::Color::srgb(0.88, 0.47, 0.41),
			Self::Lilac => bevy::prelude::Color::srgb(0.69, 0.56, 0.78),
			Self::Mint => bevy::prelude::Color::srgb(0.47, 0.78, 0.63),
			Self::Saffron => bevy::prelude::Color::srgb(0.91, 0.69, 0.25),
			Self::Rose => bevy::prelude::Color::srgb(0.85, 0.47, 0.60),
			Self::Jade => bevy::prelude::Color::srgb(0.22, 0.63, 0.44),
			Self::Amber => bevy::prelude::Color::srgb(0.88, 0.63, 0.16),
			Self::Copper => bevy::prelude::Color::srgb(0.82, 0.47, 0.28),
			Self::Slate => bevy::prelude::Color::srgb(0.42, 0.47, 0.53),
		}
	}
}
