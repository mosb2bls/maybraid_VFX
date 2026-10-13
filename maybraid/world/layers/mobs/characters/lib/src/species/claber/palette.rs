//! Claber color palettes.
//!
//! Soft earth tones a step above root gray/brown — muted purple, gold, and red
//! borrowed from the Dui skin/mouth range.

use clap::ValueEnum;

/// Shared Claber swatch set for body, features, and accents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum ClaberColor {
	#[default]
	SoftPurple,
	SoftGold,
	SoftRed,
	DesertBrown,
}

impl ClaberColor {
	pub const VALUES: &'static [Self] =
		&[Self::SoftPurple, Self::SoftGold, Self::SoftRed, Self::DesertBrown];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftPurple => "soft-purple",
			Self::SoftGold => "soft-gold",
			Self::SoftRed => "soft-red",
			Self::DesertBrown => "desert-brown",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			// Dui Purple / Gold / Mouth Red / DesertBrown
			Self::SoftPurple => "#8A787A",
			Self::SoftGold => "#A89470",
			Self::SoftRed => "#8C5C52",
			Self::DesertBrown => "#9E8970",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftPurple => bevy::prelude::Color::srgb(0.54, 0.47, 0.48),
			Self::SoftGold => bevy::prelude::Color::srgb(0.66, 0.58, 0.44),
			Self::SoftRed => bevy::prelude::Color::srgb(0.55, 0.36, 0.32),
			Self::DesertBrown => bevy::prelude::Color::srgb(0.62, 0.54, 0.44),
		}
	}
}
