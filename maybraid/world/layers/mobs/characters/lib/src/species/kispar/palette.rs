//! Kispar color palettes — soft kite / hawk tones.

use clap::ValueEnum;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KisparPlumageColor {
	#[default]
	Ash,
	Rust,
	Cream,
	Charcoal,
}

impl KisparPlumageColor {
	pub const VALUES: &'static [Self] = &[Self::Ash, Self::Rust, Self::Cream, Self::Charcoal];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Ash => "ash",
			Self::Rust => "rust",
			Self::Cream => "cream",
			Self::Charcoal => "charcoal",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Ash => "#9A9590",
			Self::Rust => "#A86A4A",
			Self::Cream => "#E8DCC8",
			Self::Charcoal => "#3A3A40",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Ash => bevy::prelude::Color::srgb(0.60, 0.58, 0.56),
			Self::Rust => bevy::prelude::Color::srgb(0.66, 0.42, 0.29),
			Self::Cream => bevy::prelude::Color::srgb(0.91, 0.86, 0.78),
			Self::Charcoal => bevy::prelude::Color::srgb(0.23, 0.23, 0.25),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KisparEyeColor {
	#[default]
	SoftAmber,
	Gold,
	Dark,
}

impl KisparEyeColor {
	pub const VALUES: &'static [Self] = &[Self::SoftAmber, Self::Gold, Self::Dark];

	pub const fn label(self) -> &'static str {
		match self {
			Self::SoftAmber => "soft-amber",
			Self::Gold => "gold",
			Self::Dark => "dark",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::SoftAmber => "#D4A86A",
			Self::Gold => "#D4B04A",
			Self::Dark => "#1A1A1E",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::SoftAmber => bevy::prelude::Color::srgb(0.83, 0.66, 0.42),
			Self::Gold => bevy::prelude::Color::srgb(0.83, 0.69, 0.29),
			Self::Dark => bevy::prelude::Color::srgb(0.10, 0.10, 0.12),
		}
	}
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, ValueEnum)]
pub enum KisparBeakColor {
	#[default]
	Horn,
	Rust,
	Charcoal,
}

impl KisparBeakColor {
	pub const VALUES: &'static [Self] = &[Self::Horn, Self::Rust, Self::Charcoal];

	pub const fn label(self) -> &'static str {
		match self {
			Self::Horn => "horn",
			Self::Rust => "rust",
			Self::Charcoal => "charcoal",
		}
	}

	pub const fn color_hex(self) -> &'static str {
		match self {
			Self::Horn => "#C4A06A",
			Self::Rust => "#A86A4A",
			Self::Charcoal => "#3A3A40",
		}
	}

	pub fn color(self) -> bevy::prelude::Color {
		match self {
			Self::Horn => bevy::prelude::Color::srgb(0.77, 0.63, 0.42),
			Self::Rust => bevy::prelude::Color::srgb(0.66, 0.42, 0.29),
			Self::Charcoal => bevy::prelude::Color::srgb(0.23, 0.23, 0.25),
		}
	}
}
