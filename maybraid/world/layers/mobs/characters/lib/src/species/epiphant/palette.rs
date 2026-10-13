//! Epiphant color palettes.
//!
//! Skin-forward swatches: grays and blue-grays for the elephant silhouette,
//! soft earth red / lavender accents, plus the shared [`ItemColor`] set kept
//! available but not as the default.

use clap::ValueEnum;

/// Shared Epiphant swatch set for body, features, and accents.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum EpiphantColor {
	// Grays
	MistGray,
	StoneGray,
	Charcoal,
	// Blue grays
	MistBlue,
	#[default]
	Slate,
	Steel,
	// Soft earth accents
	SoftEarthRed,
	SoftLavender,
	// Legacy shared item palette (kept, not default)
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

impl EpiphantColor {
	pub const VALUES: &'static [Self] = &[
		Self::MistGray,
		Self::StoneGray,
		Self::Charcoal,
		Self::MistBlue,
		Self::Slate,
		Self::Steel,
		Self::SoftEarthRed,
		Self::SoftLavender,
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
			Self::MistGray => "mist-gray",
			Self::StoneGray => "stone-gray",
			Self::Charcoal => "charcoal",
			Self::MistBlue => "mist-blue",
			Self::Slate => "slate",
			Self::Steel => "steel",
			Self::SoftEarthRed => "soft-earth-red",
			Self::SoftLavender => "soft-lavender",
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
			Self::MistGray => "#C8C4BE",
			Self::StoneGray => "#9A9690",
			Self::Charcoal => "#4A4744",
			Self::MistBlue => "#B0B8C0",
			Self::Slate => "#7A8490",
			Self::Steel => "#5A6570",
			Self::SoftEarthRed => "#C4A49A",
			Self::SoftLavender => "#B8A8B4",
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

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::MistGray => bevy::prelude::Color::srgb(0.78, 0.77, 0.74),
			Self::StoneGray => bevy::prelude::Color::srgb(0.60, 0.59, 0.56),
			Self::Charcoal => bevy::prelude::Color::srgb(0.29, 0.28, 0.27),
			Self::MistBlue => bevy::prelude::Color::srgb(0.69, 0.72, 0.75),
			Self::Slate => bevy::prelude::Color::srgb(0.48, 0.52, 0.56),
			Self::Steel => bevy::prelude::Color::srgb(0.35, 0.40, 0.44),
			Self::SoftEarthRed => bevy::prelude::Color::srgb(0.77, 0.64, 0.60),
			Self::SoftLavender => bevy::prelude::Color::srgb(0.72, 0.66, 0.71),
			Self::Natural => bevy::prelude::Color::srgb(0.72, 0.54, 0.42),
			Self::Warm => bevy::prelude::Color::srgb(0.86, 0.58, 0.38),
			Self::Cool => bevy::prelude::Color::srgb(0.46, 0.60, 0.72),
			Self::Dark => bevy::prelude::Color::srgb(0.18, 0.16, 0.15),
			Self::Light => bevy::prelude::Color::srgb(0.88, 0.80, 0.68),
			Self::Red => bevy::prelude::Color::srgb(0.72, 0.18, 0.16),
			Self::Blue => bevy::prelude::Color::srgb(0.18, 0.30, 0.76),
			Self::Green => bevy::prelude::Color::srgb(0.22, 0.52, 0.28),
			Self::Gold => bevy::prelude::Color::srgb(0.88, 0.68, 0.22),
		}
	}
}
